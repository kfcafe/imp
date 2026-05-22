use async_trait::async_trait;
use serde_json::json;

use super::{line_change_counts, truncate_head, Tool, ToolContext, ToolOutput};
use crate::config::WriteOverwritePolicy;
use crate::error::Result;
use crate::tools::code_intel;

pub struct WriteTool;

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }
    fn label(&self) -> &str {
        "Write File"
    }
    fn description(&self) -> &str {
        "Create or overwrite a file. Creates parent dirs automatically."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" },
                "mode": {
                    "type": "string",
                    "enum": ["create", "overwrite"],
                    "description": "Optional safety mode. create fails if the file exists; overwrite makes replacement explicit. Omitted preserves existing behavior."
                },
                "validate_syntax": {
                    "type": "boolean",
                    "description": "When true, parse source content before writing and reject syntax errors for supported languages."
                }
            },
            "required": ["path", "content"]
        })
    }
    fn is_readonly(&self) -> bool {
        false
    }

    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let raw_path = params["path"].as_str().unwrap_or("");
        let content = params["content"].as_str().unwrap_or("");
        let mode = params["mode"].as_str();
        let validate_syntax = params["validate_syntax"]
            .as_bool()
            .or_else(|| params["validateSyntax"].as_bool())
            .unwrap_or(false);

        if raw_path.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: path"));
        }

        let path = super::resolve_path(&ctx.cwd, raw_path);

        if let Err(error) = ctx.check_write_path(&path) {
            return Ok(ToolOutput::error(error));
        }

        if path.is_dir() {
            return Ok(ToolOutput::error(format!(
                "Path is a directory, not a file: {}",
                path.display()
            )));
        }

        let existed = path.exists();
        if matches!(mode, Some("create")) && existed {
            return Ok(ToolOutput::error(format!(
                "Write mode create refuses to overwrite existing file: {}",
                path.display()
            )));
        }

        let overwrite_check = if existed {
            evaluate_overwrite_policy(&path, &ctx)
        } else {
            OverwriteCheck::default()
        };
        if let Some(error) = overwrite_check.error {
            return Ok(ToolOutput::error(error));
        }

        let before_content = if existed {
            tokio::fs::read_to_string(&path).await.ok()
        } else {
            None
        };

        // Create parent directories
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let checkpoint = if existed {
            ctx.checkpoint_state.snapshot_paths(
                std::slice::from_ref(&path),
                Some(format!("write {}", path.display())),
            )?
        } else {
            None
        };

        // Detect existing line endings to preserve them, default to LF for new files
        let normalized = if existed {
            if let Ok(existing) = tokio::fs::read(&path).await {
                let has_crlf = existing.windows(2).any(|w| w == b"\r\n");
                if has_crlf {
                    // Preserve CRLF: ensure content uses CRLF
                    let lf_content = content.replace("\r\n", "\n");
                    lf_content.replace('\n', "\r\n")
                } else {
                    // LF or no newlines — ensure LF
                    content.replace("\r\n", "\n")
                }
            } else {
                content.replace("\r\n", "\n")
            }
        } else {
            content.replace("\r\n", "\n")
        };

        let syntax_validation =
            validate_syntax.then(|| code_intel::validate_syntax(&normalized, &path));
        if let Some(validation) = &syntax_validation {
            if validation.supported && !validation.valid {
                return Ok(ToolOutput::error(format!(
                    "Write would create syntax errors in {raw_path}: {:?}. No changes made.",
                    validation.errors
                )));
            }
        }
        let symbol_diff = before_content
            .as_deref()
            .map(|before| code_intel::diff_top_level_symbols(before, &normalized, &path));

        let bytes_written = normalized.len();
        let (lines_added, lines_removed) = if let Some(before) = before_content.as_deref() {
            line_change_counts(before, &normalized)
        } else {
            (normalized.lines().count(), 0)
        };
        tokio::fs::write(&path, &normalized).await?;

        let action = if existed { "overwritten" } else { "created" };
        let display = path.display().to_string();
        let summary = format!("{display}: {bytes_written} bytes {action}");

        const DISPLAY_MAX_LINES: usize = 40;
        const DISPLAY_MAX_BYTES: usize = 8_000;
        let display_source = normalized.replace("\r\n", "\n");
        let display_result = truncate_head(&display_source, DISPLAY_MAX_LINES, DISPLAY_MAX_BYTES);
        let display_content = display_result.content.trim_end_matches('\n').to_string();
        let display_note = if display_result.truncated {
            let note = format!(
                "[output truncated: showing {}/{} lines, {}/{} bytes]",
                display_result.output_lines,
                display_result.total_lines,
                display_result.output_bytes,
                display_result.total_bytes,
            );
            if let Some(ref tf) = display_result.temp_file {
                format!("{note} full output: {}", tf.display())
            } else {
                note
            }
        } else {
            String::new()
        };

        let warnings = overwrite_check.warning_messages;
        let warning_codes = overwrite_check.warning_codes;

        let mut text = summary.clone();
        for warning in &warnings {
            text.push('\n');
            text.push_str(warning);
        }

        Ok(ToolOutput {
            content: vec![imp_llm::ContentBlock::Text { text }],
            details: json!({
                "action": action,
                "path": display,
                "bytes_written": bytes_written,
                "line_ending": if normalized.contains("\r\n") { "crlf" } else { "lf" },
                "created": !existed,
                "overwritten": existed,
                "lines_added": lines_added,
                "lines_removed": lines_removed,
                "files": [{
                    "path": display,
                    "status": if existed { "modified" } else { "created" },
                    "lines_added": lines_added,
                    "lines_removed": lines_removed,
                }],
                "checkpoint_id": checkpoint.as_ref().map(|c| c.id.clone()),
                "checkpoint_label": checkpoint.as_ref().and_then(|c| c.label.clone()),
                "summary": summary,
                "warnings": warnings,
                "warning_codes": warning_codes,
                "overwrite_policy": ctx.config.write.overwrite_policy,
                "mode": mode,
                "syntax_validation": syntax_validation.as_ref().map(|validation| json!({
                    "supported": validation.supported,
                    "valid": validation.valid,
                    "language": validation.language,
                    "errors": validation.errors.iter().map(|error| json!({
                        "start_line": error.start_line,
                        "end_line": error.end_line,
                        "kind": error.kind,
                    })).collect::<Vec<_>>(),
                })),
                "symbol_diff": symbol_diff.as_ref().map(|diff| json!({
                    "added": diff.added.iter().cloned().collect::<Vec<_>>(),
                    "removed": diff.removed.iter().cloned().collect::<Vec<_>>(),
                    "before": diff.before.iter().cloned().collect::<Vec<_>>(),
                    "after": diff.after.iter().cloned().collect::<Vec<_>>(),
                })),
                "display_content": display_content,
                "display_note": display_note,
            }),
            is_error: false,
        })
    }
}

#[derive(Default)]
struct OverwriteCheck {
    warning_messages: Vec<String>,
    warning_codes: Vec<&'static str>,
    error: Option<String>,
}

fn evaluate_overwrite_policy(path: &std::path::Path, ctx: &ToolContext) -> OverwriteCheck {
    let Ok(tracker) = ctx.file_tracker.lock() else {
        return OverwriteCheck::default();
    };

    let was_read = tracker.was_read(path);
    let is_stale = tracker.is_stale(path);
    let policy = ctx.config.write.overwrite_policy;

    if matches!(policy, WriteOverwritePolicy::Deny) {
        return OverwriteCheck {
            error: Some(format!(
                "Overwriting existing files is disabled by write overwrite policy: {}",
                path.display()
            )),
            ..OverwriteCheck::default()
        };
    }

    if matches!(policy, WriteOverwritePolicy::RequireRead) && !was_read {
        return OverwriteCheck {
            error: Some(format!(
                "Write overwrite policy requires reading the file before overwriting: {}",
                path.display()
            )),
            ..OverwriteCheck::default()
        };
    }

    if matches!(
        policy,
        WriteOverwritePolicy::RequireRead | WriteOverwritePolicy::BlockStale
    ) && is_stale
    {
        return OverwriteCheck {
            error: Some(format!(
                "Write overwrite policy blocks overwriting stale files. Re-read before overwriting: {}",
                path.display()
            )),
            ..OverwriteCheck::default()
        };
    }

    let mut check = OverwriteCheck::default();
    if !was_read {
        check.warning_codes.push("unread_overwrite");
        check.warning_messages.push(format!(
            "Warning: overwriting {} without reading it first. Consider reading to verify current content.",
            path.display()
        ));
    } else if is_stale {
        check.warning_codes.push("stale_overwrite");
        check.warning_messages.push(format!(
            "Warning: {} was modified externally since last read. Re-read to verify current content.",
            path.display()
        ));
    }

    check
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolContext;
    use std::path::Path;
    use std::sync::Arc;

    fn test_ctx(dir: &Path) -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(16);
        ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            command_tx: cmd_tx,
            ui: Arc::new(crate::ui::NullInterface),
            file_cache: Arc::new(crate::tools::FileCache::new()),
            checkpoint_state: Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(crate::tools::FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            config: Arc::new(crate::config::Config::default()),
            run_policy: Default::default(),
            supporting_provenance: Vec::new(),
        }
    }

    fn test_ctx_with_policy(dir: &Path, overwrite_policy: WriteOverwritePolicy) -> ToolContext {
        let mut ctx = test_ctx(dir);
        let mut config = crate::config::Config::default();
        config.write.overwrite_policy = overwrite_policy;
        ctx.config = Arc::new(config);
        ctx
    }

    fn test_ctx_with_run_policy(dir: &Path, run_policy: crate::policy::RunPolicy) -> ToolContext {
        let mut ctx = test_ctx(dir);
        ctx.run_policy = run_policy;
        ctx
    }

    #[tokio::test]
    async fn write_create_mode_refuses_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("lib.rs");
        std::fs::write(&file, "fn old() {}\n").unwrap();

        let tool = WriteTool;
        let result = tool
            .execute(
                "c-create-mode",
                serde_json::json!({"path": "lib.rs", "content": "fn new() {}\n", "mode": "create"}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result
            .text_content()
            .unwrap()
            .contains("refuses to overwrite"));
        assert_eq!(std::fs::read_to_string(file).unwrap(), "fn old() {}\n");
    }

    #[tokio::test]
    async fn write_overwrite_reports_symbol_diff() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("lib.rs");
        std::fs::write(&file, "fn old() {}\n").unwrap();

        let tool = WriteTool;
        let result = tool
            .execute(
                "c-symbol-diff",
                serde_json::json!({"path": "lib.rs", "content": "fn new() {}\n", "mode": "overwrite"}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.details["symbol_diff"]["added"][0], "new");
        assert_eq!(result.details["symbol_diff"]["removed"][0], "old");
    }

    #[tokio::test]
    async fn write_validate_syntax_blocks_invalid_source() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("lib.rs");
        std::fs::write(&file, "fn old() {}\n").unwrap();

        let tool = WriteTool;
        let result = tool
            .execute(
                "c-write-syntax",
                serde_json::json!({"path": "lib.rs", "content": "fn broken( {\n", "validateSyntax": true}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.text_content().unwrap().contains("syntax errors"));
        assert_eq!(std::fs::read_to_string(file).unwrap(), "fn old() {}\n");
    }

    #[tokio::test]
    async fn write_path_policy_allows_matching_file() {
        let dir = tempfile::tempdir().unwrap();
        let tool = WriteTool;

        let result = tool
            .execute(
                "c-allow-write",
                serde_json::json!({"path": "CHANGELOG.md", "content": "updated"}),
                test_ctx_with_run_policy(
                    dir.path(),
                    crate::policy::RunPolicy::new().allow_write("CHANGELOG.md"),
                ),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("CHANGELOG.md")).unwrap(),
            "updated"
        );
    }

    #[tokio::test]
    async fn write_path_policy_blocks_unlisted_file() {
        let dir = tempfile::tempdir().unwrap();
        let tool = WriteTool;

        let result = tool
            .execute(
                "c-deny-write",
                serde_json::json!({"path": "src/lib.rs", "content": "updated"}),
                test_ctx_with_run_policy(
                    dir.path(),
                    crate::policy::RunPolicy::new().allow_write("CHANGELOG.md"),
                ),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.text_content().unwrap().contains("write allowlist"));
        assert!(!dir.path().join("src/lib.rs").exists());
    }

    #[tokio::test]
    async fn write_path_policy_blocks_parent_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let relative =
            pathdiff::diff_paths(outside.path().join("CHANGELOG.md"), dir.path()).unwrap();
        let tool = WriteTool;

        let result = tool
            .execute(
                "c-traversal",
                serde_json::json!({"path": relative, "content": "updated"}),
                test_ctx_with_run_policy(
                    dir.path(),
                    crate::policy::RunPolicy::new().allow_write("CHANGELOG.md"),
                ),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result
            .text_content()
            .unwrap()
            .contains("outside the worker root"));
        assert!(!outside.path().join("CHANGELOG.md").exists());
    }

    #[tokio::test]
    async fn write_path_policy_deny_overrides_allow() {
        let dir = tempfile::tempdir().unwrap();
        let tool = WriteTool;

        let result = tool
            .execute(
                "c-deny-override",
                serde_json::json!({"path": "CHANGELOG.md", "content": "updated"}),
                test_ctx_with_run_policy(
                    dir.path(),
                    crate::policy::RunPolicy::new()
                        .allow_write("CHANGELOG.md")
                        .deny_write("CHANGELOG.md"),
                ),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.text_content().unwrap().contains("denylist"));
        assert!(!dir.path().join("CHANGELOG.md").exists());
    }

    #[tokio::test]
    async fn write_path_policy_glob_allows_matching_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("docs")).unwrap();
        let tool = WriteTool;

        let result = tool
            .execute(
                "c-glob-write",
                serde_json::json!({"path": "docs/CHANGELOG.md", "content": "updated"}),
                test_ctx_with_run_policy(
                    dir.path(),
                    crate::policy::RunPolicy::new().allow_write("docs/*.md"),
                ),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("docs/CHANGELOG.md")).unwrap(),
            "updated"
        );
    }

    #[tokio::test]
    async fn write_default_policy_warns_on_unread_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("existing.txt");
        std::fs::write(&file, "original").unwrap();

        let tool = WriteTool;
        let result = tool
            .execute(
                "c-warn",
                serde_json::json!({"path": "existing.txt", "content": "updated"}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.details["warning_codes"][0], "unread_overwrite");
        assert_eq!(result.details["overwritten"], true);
        assert!(result.details["checkpoint_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn write_require_read_policy_blocks_unread_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("existing.txt");
        std::fs::write(&file, "original").unwrap();

        let tool = WriteTool;
        let result = tool
            .execute(
                "c-block-unread",
                serde_json::json!({"path": "existing.txt", "content": "updated"}),
                test_ctx_with_policy(dir.path(), WriteOverwritePolicy::RequireRead),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert_eq!(std::fs::read_to_string(file).unwrap(), "original");
    }

    #[tokio::test]
    async fn write_block_stale_policy_blocks_stale_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("existing.txt");
        std::fs::write(&file, "original").unwrap();

        let ctx = test_ctx_with_policy(dir.path(), WriteOverwritePolicy::BlockStale);
        ctx.file_tracker.lock().unwrap().record_read(&file);
        std::thread::sleep(std::time::Duration::from_millis(5));
        std::fs::write(&file, "external").unwrap();

        let tool = WriteTool;
        let result = tool
            .execute(
                "c-block-stale",
                serde_json::json!({"path": "existing.txt", "content": "updated"}),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert_eq!(std::fs::read_to_string(file).unwrap(), "external");
    }

    #[tokio::test]
    async fn write_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let tool = WriteTool;

        let result = tool
            .execute(
                "c1",
                serde_json::json!({"path": "new.txt", "content": "hello world"}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let details = &result.details;
        assert_eq!(details["display_content"], "hello world");
        assert!(details["summary"]
            .as_str()
            .unwrap()
            .ends_with("new.txt: 11 bytes created"));
        let written = std::fs::read_to_string(dir.path().join("new.txt")).unwrap();
        assert_eq!(written, "hello world");
    }

    #[tokio::test]
    async fn write_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let tool = WriteTool;

        let result = tool
            .execute(
                "c2",
                serde_json::json!({"path": "a/b/c/deep.txt", "content": "deep"}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let written = std::fs::read_to_string(dir.path().join("a/b/c/deep.txt")).unwrap();
        assert_eq!(written, "deep");
    }

    #[tokio::test]
    async fn write_overwrite_creates_checkpoint_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("existing.txt");
        std::fs::write(&file, "original").unwrap();

        let tool = WriteTool;
        let ctx = test_ctx(dir.path());
        let checkpoint_state = ctx.checkpoint_state.clone();

        let result = tool
            .execute(
                "c-overwrite",
                serde_json::json!({"path": "existing.txt", "content": "updated"}),
                ctx,
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(
            checkpoint_state.original(&file).as_deref(),
            Some("original")
        );
        let checkpoints = checkpoint_state.checkpoints();
        assert_eq!(checkpoints.len(), 1);
        assert!(checkpoints[0].files.contains(&file));
    }

    #[tokio::test]
    async fn write_empty_content() {
        let dir = tempfile::tempdir().unwrap();
        let tool = WriteTool;

        let result = tool
            .execute(
                "c4",
                serde_json::json!({"path": "empty.txt", "content": ""}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let written = std::fs::read_to_string(dir.path().join("empty.txt")).unwrap();
        assert_eq!(written, "");
        assert_eq!(result.details["display_content"], "");
    }

    #[tokio::test]
    async fn write_missing_path_error() {
        let dir = tempfile::tempdir().unwrap();
        let tool = WriteTool;

        let result = tool
            .execute(
                "c5",
                serde_json::json!({"content": "hello"}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
    }

    #[tokio::test]
    async fn write_preserves_crlf_on_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("crlf.txt");
        // Write a CRLF file first
        std::fs::write(&file, "line1\r\nline2\r\n").unwrap();

        let tool = WriteTool;
        let result = tool
            .execute(
                "c6",
                serde_json::json!({"path": "crlf.txt", "content": "new1\nnew2\n"}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let raw = std::fs::read(dir.path().join("crlf.txt")).unwrap();
        // Should convert LF to CRLF since original had CRLF
        assert!(raw.windows(2).any(|w| w == b"\r\n"));
    }

    #[tokio::test]
    async fn write_deep_nested_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let tool = WriteTool;

        let result = tool
            .execute(
                "c7",
                serde_json::json!({"path": "x/y/z/w/v/deep.txt", "content": "deep content"}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let written = std::fs::read_to_string(dir.path().join("x/y/z/w/v/deep.txt")).unwrap();
        assert_eq!(written, "deep content");
    }

    #[tokio::test]
    async fn write_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("exist.txt");
        std::fs::write(&file, "old content").unwrap();

        let tool = WriteTool;
        let result = tool
            .execute(
                "c3",
                serde_json::json!({"path": "exist.txt", "content": "new content"}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = result
            .content
            .iter()
            .find_map(|b| match b {
                imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap();
        assert!(text.contains("overwritten"));
        let written = std::fs::read_to_string(&file).unwrap();
        assert_eq!(written, "new content");
    }

    #[tokio::test]
    async fn write_includes_display_content_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let tool = WriteTool;

        let result = tool
            .execute(
                "c8",
                serde_json::json!({"path": "preview.rs", "content": "fn main() {\n    println!(\"hi\");\n}\n"}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.details["path"]
            .as_str()
            .unwrap()
            .ends_with("preview.rs"));
        assert!(result.details["summary"]
            .as_str()
            .unwrap()
            .ends_with("preview.rs: 34 bytes created"));
        assert_eq!(
            result.details["display_content"],
            "fn main() {\n    println!(\"hi\");\n}"
        );
        assert_eq!(result.details["display_note"], "");
    }

    #[tokio::test]
    async fn write_display_content_truncates_large_content() {
        let dir = tempfile::tempdir().unwrap();
        let tool = WriteTool;
        let content = (0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");

        let result = tool
            .execute(
                "c9",
                serde_json::json!({"path": "large.txt", "content": content}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let display_content = result.details["display_content"].as_str().unwrap();
        assert!(display_content.lines().count() <= 40);
        assert!(result.details["display_note"]
            .as_str()
            .unwrap()
            .contains("output truncated"));
    }
}
