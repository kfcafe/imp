use std::path::Path;

use async_trait::async_trait;
use imp_llm::truncate_chars_with_suffix;
use serde_json::json;

use super::fuzzy;
use super::{generate_diff, suggest_similar_files, Tool, ToolContext, ToolOutput};
use crate::error::Result;

pub struct EditTool;

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }
    fn label(&self) -> &str {
        "Edit File"
    }
    fn description(&self) -> &str {
        "Edit files by exact replacement, anchored range, or edits[] transaction."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path or default edits[] path" },
                "old_text": { "type": "string", "description": "Text to replace" },
                "new_text": { "type": "string", "description": "Replacement text" },
                "dry_run": {
                    "type": "boolean",
                    "description": "Dry run; return diff only"
                },
                "expected_occurrences": {
                    "type": "integer",
                    "description": "Required exact old_text match count"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all exact matches"
                },
                "anchor_start": {
                    "type": "string",
                    "description": "Start anchor from read(anchors=true)"
                },
                "anchor_end": {
                    "type": "string",
                    "description": "Optional end anchor"
                },
                "edits": {
                    "type": "array",
                    "description": "Transactional edits[]",
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Per-edit path" },
                            "old_text": { "type": "string" },
                            "new_text": { "type": "string" }
                        },
                        "required": ["old_text", "new_text"]
                    }
                }
            },
            "required": []
        })
    }
    fn is_readonly(&self) -> bool {
        false
    }

    async fn execute(
        &self,
        call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        // Multi-edit mode: if `edits` array is present, delegate to MultiEditTool
        if params.get("edits").is_some_and(|v| v.is_array()) {
            return super::multi_edit::MultiEditTool
                .execute(call_id, params, ctx)
                .await;
        }

        let raw_path = params["path"].as_str().unwrap_or("");
        let old_text = get_str_param(&params, "old_text", "oldText").unwrap_or("");
        let new_text = get_str_param(&params, "new_text", "newText").unwrap_or("");
        let dry_run = get_bool_param(&params, "dry_run", "dryRun").unwrap_or(false);
        let replace_all = get_bool_param(&params, "replace_all", "replaceAll").unwrap_or(false);
        let expected_occurrences = params
            .get("expected_occurrences")
            .or_else(|| params.get("expectedOccurrences"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        if raw_path.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: path"));
        }

        let path = super::resolve_path(&ctx.cwd, raw_path);

        if get_str_param(&params, "anchor_start", "anchorStart").is_some() {
            return execute_anchor_edit(&path, raw_path, &params, ctx).await;
        }

        if old_text.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: old_text"));
        }

        if let Err(error) = ctx.check_write_path(&path) {
            return Ok(ToolOutput::error(error));
        }
        if !path.exists() {
            let suggestions = suggest_similar_files(&ctx.cwd, raw_path);
            let mut msg = format!("File not found: {}", path.display());
            if !suggestions.is_empty() {
                msg.push_str("\n\nDid you mean:");
                for s in &suggestions {
                    msg.push_str(&format!("\n  {s}"));
                }
            }
            return Ok(ToolOutput::error(msg));
        }

        // Check for unread or stale file — warn but don't block.
        let tracker_warning = {
            let tracker = ctx.file_tracker.lock().ok();
            match tracker {
                Some(t) if !t.was_read(&path) => Some(format!(
                    "Warning: editing {} without reading it first. Consider reading to verify current content.",
                    path.display()
                )),
                Some(t) if t.is_stale(&path) => Some(format!(
                    "Warning: {} was modified externally since last read. Re-read to verify current content.",
                    path.display()
                )),
                _ => None,
            }
        };

        let raw_content = tokio::fs::read_to_string(&path).await?;

        // Normalize to LF for internal processing
        let content = raw_content.replace("\r\n", "\n");
        let has_crlf = raw_content.contains("\r\n");
        let old_normalized = old_text.replace("\r\n", "\n");
        let new_normalized = new_text.replace("\r\n", "\n");

        let exact_occurrences = count_occurrences(&content, &old_normalized);
        if let Some(expected) = expected_occurrences {
            if exact_occurrences != expected {
                return Ok(ToolOutput::error(format!(
                    "Expected {expected} exact occurrence(s) of old_text in {raw_path}, found {exact_occurrences}. No changes made."
                )));
            }
        }

        let (new_content, was_fuzzy, replacements) = if replace_all {
            if exact_occurrences == 0 {
                return match apply_edit(&content, &old_normalized, &new_normalized) {
                    Ok((_, true)) => Ok(ToolOutput::error(
                        "replaceAll requires exact matches and does not use fuzzy matching. Found 0 exact matches, but a fuzzy match exists. No changes made.",
                    )),
                    Ok(_) => unreachable!("apply_edit cannot exact-match when exact_occurrences is 0"),
                    Err(output) => Ok(output),
                };
            }
            (
                content.replace(&old_normalized, &new_normalized),
                false,
                exact_occurrences,
            )
        } else {
            match apply_edit(&content, &old_normalized, &new_normalized) {
                Ok((new_content, was_fuzzy)) => (new_content, was_fuzzy, 1),
                Err(output) => return Ok(output),
            }
        };

        let diff = generate_diff(raw_path, &content, &new_content);

        // Restore original line endings if needed
        let final_content = if has_crlf {
            new_content.replace('\n', "\r\n")
        } else {
            new_content
        };

        if !dry_run {
            ctx.checkpoint_state.snapshot_paths(
                std::slice::from_ref(&path),
                Some(format!("edit {}", path.display())),
            )?;
            tokio::fs::write(&path, &final_content).await?;
        }

        let mut msg = diff;
        if dry_run {
            msg.push_str("\n(dry run: no changes written)");
        }
        if was_fuzzy {
            msg.push_str(
                "\n(matched using fuzzy matching: trailing whitespace/unicode normalized)",
            );
        }
        if let Some(warning) = tracker_warning {
            msg.push('\n');
            msg.push_str(&warning);
        }

        Ok(ToolOutput {
            content: vec![imp_llm::ContentBlock::Text { text: msg }],
            details: json!({
                "action": "edit",
                "mode": "single",
                "path": path.display().to_string(),
                "fuzzy_match": was_fuzzy,
                "dry_run": dry_run,
                "replace_all": replace_all,
                "exact_occurrences": exact_occurrences,
                "replacements": replacements,
            }),
            is_error: false,
        })
    }
}

fn get_str_param<'a>(
    params: &'a serde_json::Value,
    primary: &str,
    legacy: &str,
) -> Option<&'a str> {
    params
        .get(primary)
        .and_then(|v| v.as_str())
        .or_else(|| params.get(legacy).and_then(|v| v.as_str()))
}

fn get_bool_param(params: &serde_json::Value, primary: &str, legacy: &str) -> Option<bool> {
    params
        .get(primary)
        .and_then(|v| v.as_bool())
        .or_else(|| params.get(legacy).and_then(|v| v.as_bool()))
}

async fn execute_anchor_edit(
    path: &Path,
    raw_path: &str,
    params: &serde_json::Value,
    ctx: ToolContext,
) -> Result<ToolOutput> {
    let Some(anchor_start_id) = get_str_param(params, "anchor_start", "anchorStart") else {
        return Ok(ToolOutput::error(
            "Missing required parameter: anchor_start",
        ));
    };
    let anchor_end_id = get_str_param(params, "anchor_end", "anchorEnd").unwrap_or(anchor_start_id);
    let Some(replacement) = get_str_param(params, "new_text", "replacement") else {
        return Ok(ToolOutput::error(
            "Missing required parameter: new_text for anchored edit mode",
        ));
    };
    let dry_run = get_bool_param(params, "dry_run", "dryRun").unwrap_or(false);

    if let Err(error) = ctx.check_write_path(path) {
        return Ok(ToolOutput::error(error));
    }
    if !path.exists() {
        let suggestions = suggest_similar_files(&ctx.cwd, raw_path);
        let mut msg = format!("File not found: {}", path.display());
        if !suggestions.is_empty() {
            msg.push_str("\n\nDid you mean:");
            for s in &suggestions {
                msg.push_str(&format!("\n  {s}"));
            }
        }
        return Ok(ToolOutput::error(msg));
    }

    let Some(start_anchor) = ctx.anchor_store.get(path, anchor_start_id) else {
        return Ok(ToolOutput::error(format!(
            "Anchor not found or expired for {raw_path}: {anchor_start_id}. Re-read with anchors=true before editing."
        )));
    };
    let Some(end_anchor) = ctx.anchor_store.get(path, anchor_end_id) else {
        return Ok(ToolOutput::error(format!(
            "Anchor not found or expired for {raw_path}: {anchor_end_id}. Re-read with anchors=true before editing."
        )));
    };
    if start_anchor.line > end_anchor.line {
        return Ok(ToolOutput::error(
            "anchorStart must refer to a line before or equal to anchorEnd",
        ));
    }

    let raw_content = tokio::fs::read_to_string(path).await?;
    let content = raw_content.replace("\r\n", "\n");
    let has_crlf = raw_content.contains("\r\n");
    let lines = content.lines().collect::<Vec<_>>();
    let start_idx = start_anchor.line.saturating_sub(1);
    let end_idx = end_anchor.line.saturating_sub(1);
    if start_idx >= lines.len() || end_idx >= lines.len() {
        return Ok(ToolOutput::error(
            "Anchor line is outside the current file. Re-read with anchors=true before editing.",
        ));
    }
    if super::stable_hash(lines[start_idx]) != start_anchor.content_hash {
        return Ok(ToolOutput::error(format!(
            "Stale anchor at line {} in {raw_path}. Re-read with anchors=true before editing.",
            start_anchor.line
        )));
    }
    if super::stable_hash(lines[end_idx]) != end_anchor.content_hash {
        return Ok(ToolOutput::error(format!(
            "Stale anchor at line {} in {raw_path}. Re-read with anchors=true before editing.",
            end_anchor.line
        )));
    }

    let mut replacement_normalized = replacement.replace("\r\n", "\n");
    let had_trailing_newline = content.ends_with('\n');
    let mut new_lines = Vec::with_capacity(lines.len() + replacement_normalized.lines().count());
    new_lines.extend_from_slice(&lines[..start_idx]);
    if replacement_normalized.ends_with('\n') {
        replacement_normalized.pop();
    }
    if !replacement_normalized.is_empty() {
        new_lines.extend(replacement_normalized.lines());
    }
    new_lines.extend_from_slice(&lines[end_idx + 1..]);
    let mut new_content = new_lines.join("\n");
    if had_trailing_newline {
        new_content.push('\n');
    }

    let diff = generate_diff(raw_path, &content, &new_content);
    let final_content = if has_crlf {
        new_content.replace('\n', "\r\n")
    } else {
        new_content.clone()
    };

    if !dry_run {
        ctx.checkpoint_state.snapshot_paths(
            std::slice::from_ref(&path.to_path_buf()),
            Some(format!("anchored edit {}", path.display())),
        )?;
        tokio::fs::write(path, &final_content).await?;
        if let Ok(mut tracker) = ctx.file_tracker.lock() {
            tracker.record_read(path);
        }
    }

    let refreshed_lines = new_content.lines().collect::<Vec<_>>();
    let refreshed =
        ctx.anchor_store
            .record_lines(path, super::stable_hash(&new_content), 1, &refreshed_lines);
    let mut msg = diff;
    if dry_run {
        msg.push_str("\n(dry run: no changes written)");
    }
    msg.push_str("\n(anchored edit: anchors validated before replacement)");

    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text: msg }],
        details: json!({
            "action": "edit",
            "mode": "anchored",
            "path": path.display().to_string(),
            "dry_run": dry_run,
            "anchored": true,
            "start_line": start_anchor.line,
            "end_line": end_anchor.line,
            "refreshed_anchors": refreshed.iter().map(|anchor| json!({
                "line": anchor.line,
                "anchor": anchor.id,
                "content_hash": format!("{:016x}", anchor.content_hash),
            })).collect::<Vec<_>>(),
        }),
        is_error: false,
    })
}

fn count_occurrences(content: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    content.match_indices(needle).count()
}

/// Apply a single edit, returning the new content and whether fuzzy matching was used.
/// Extracted so multi_edit can reuse it.
pub(crate) fn apply_edit(
    content: &str,
    old_text: &str,
    new_text: &str,
) -> std::result::Result<(String, bool), ToolOutput> {
    // Try exact match first
    if let Some(pos) = content.find(old_text) {
        let mut result = String::with_capacity(content.len());
        result.push_str(&content[..pos]);
        result.push_str(new_text);
        result.push_str(&content[pos + old_text.len()..]);
        return Ok((result, false));
    }

    // Try fuzzy match
    if let Some(m) = fuzzy::fuzzy_find(content, old_text) {
        let mut result = String::with_capacity(content.len());
        result.push_str(&content[..m.start]);
        result.push_str(new_text);
        result.push_str(&content[m.end..]);
        return Ok((result, true));
    }

    // No match — build helpful error
    let preview = truncate_chars_with_suffix(content, 200, "");
    let msg = format!(
        "Could not find the specified text to replace.\n\
         First 200 chars of file:\n{preview}"
    );
    Err(ToolOutput::error(msg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolContext;
    use std::sync::Arc;

    fn test_ctx(dir: &std::path::Path) -> ToolContext {
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
        }
    }

    #[tokio::test]
    async fn edit_exact_match() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.rs");
        std::fs::write(&file, "fn main() {\n    println!(\"hello\");\n}\n").unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                "c1",
                json!({
                    "path": "test.rs",
                    "oldText": "println!(\"hello\")",
                    "newText": "println!(\"world\")"
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let written = std::fs::read_to_string(&file).unwrap();
        assert!(written.contains("world"));
        assert!(!written.contains("hello"));
    }

    #[tokio::test]
    async fn edit_dry_run_returns_diff_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("dry.txt");
        std::fs::write(&file, "alpha\n").unwrap();

        let tool = EditTool;
        let ctx = test_ctx(dir.path());
        let checkpoint_state = ctx.checkpoint_state.clone();
        let result = tool
            .execute(
                "c-dry",
                json!({
                    "path": "dry.txt",
                    "oldText": "alpha",
                    "newText": "beta",
                    "dryRun": true
                }),
                ctx,
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "alpha\n");
        assert!(checkpoint_state.checkpoints().is_empty());
        assert_eq!(result.details["dry_run"], true);
        let text = result.text_content().unwrap();
        assert!(text.contains("beta"));
        assert!(text.contains("dry run"));
    }

    #[tokio::test]
    async fn edit_expected_occurrences_mismatch_does_not_write() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("expected-mismatch.txt");
        std::fs::write(&file, "foo foo\n").unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                "c-expected-mismatch",
                json!({
                    "path": "expected-mismatch.txt",
                    "oldText": "foo",
                    "newText": "bar",
                    "expectedOccurrences": 1
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "foo foo\n");
        assert!(result.text_content().unwrap().contains("found 2"));
    }

    #[tokio::test]
    async fn edit_expected_occurrences_success_writes() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("expected-success.txt");
        std::fs::write(&file, "foo\n").unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                "c-expected-success",
                json!({
                    "path": "expected-success.txt",
                    "oldText": "foo",
                    "newText": "bar",
                    "expectedOccurrences": 1
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "bar\n");
        assert_eq!(result.details["exact_occurrences"], 1);
        assert_eq!(result.details["replacements"], 1);
    }

    #[tokio::test]
    async fn edit_replace_all_replaces_exact_matches() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("replace-all.txt");
        std::fs::write(&file, "foo bar foo baz foo\n").unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                "c-replace-all",
                json!({
                    "path": "replace-all.txt",
                    "oldText": "foo",
                    "newText": "zap",
                    "replaceAll": true,
                    "expectedOccurrences": 3
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "zap bar zap baz zap\n"
        );
        assert_eq!(result.details["replace_all"], true);
        assert_eq!(result.details["replacements"], 3);
    }

    #[tokio::test]
    async fn edit_creates_checkpoint_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("checkpoint.txt");
        std::fs::write(&file, "alpha\n").unwrap();

        let tool = EditTool;
        let ctx = test_ctx(dir.path());
        let checkpoint_state = ctx.checkpoint_state.clone();

        let result = tool
            .execute(
                "c-checkpoint",
                json!({
                    "path": "checkpoint.txt",
                    "oldText": "alpha",
                    "newText": "beta"
                }),
                ctx,
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(checkpoint_state.original(&file).as_deref(), Some("alpha\n"));
        assert_eq!(checkpoint_state.checkpoints().len(), 1);
    }

    #[tokio::test]
    async fn edit_fuzzy_trailing_whitespace() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("ws.txt");
        // File has trailing spaces on lines
        std::fs::write(&file, "hello   \nworld   \n").unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                "c2",
                json!({
                    "path": "ws.txt",
                    "oldText": "hello\nworld",
                    "newText": "goodbye\nuniverse"
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error, "Expected success but got error");
        let written = std::fs::read_to_string(&file).unwrap();
        assert!(written.contains("goodbye"));
    }

    #[tokio::test]
    async fn edit_fuzzy_unicode_quotes() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("uni.txt");
        // File has smart quotes
        std::fs::write(&file, "he said \u{201C}hello\u{201D}\n").unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                "c3",
                json!({
                    "path": "uni.txt",
                    "oldText": "he said \"hello\"",
                    "newText": "she said \"bye\""
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error, "Expected success but got error");
        let written = std::fs::read_to_string(&file).unwrap();
        assert!(written.contains("bye"));
    }

    #[tokio::test]
    async fn edit_crlf_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("crlf.txt");
        std::fs::write(&file, "line1\r\nline2\r\nline3\r\n").unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                "c5",
                json!({
                    "path": "crlf.txt",
                    "oldText": "line2",
                    "newText": "replaced"
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let written = std::fs::read_to_string(&file).unwrap();
        assert!(written.contains("replaced"));
        // CRLF line endings should be preserved
        assert!(written.contains("\r\n"));
        assert!(!written.contains("line2"));
    }

    #[tokio::test]
    async fn edit_replaces_first_occurrence_only() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("multi.txt");
        std::fs::write(&file, "foo bar foo baz foo\n").unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                "c6",
                json!({
                    "path": "multi.txt",
                    "oldText": "foo",
                    "newText": "REPLACED"
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let written = std::fs::read_to_string(&file).unwrap();
        // Should replace only the first occurrence
        assert_eq!(written, "REPLACED bar foo baz foo\n");
    }

    #[tokio::test]
    async fn edit_empty_old_text_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("empty.txt");
        std::fs::write(&file, "some content\n").unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                "c7",
                json!({
                    "path": "empty.txt",
                    "oldText": "",
                    "newText": "replacement"
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        let text = result
            .content
            .iter()
            .find_map(|b| match b {
                imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap();
        assert!(text.contains("old_text"));
    }

    #[tokio::test]
    async fn edit_nonexistent_file_error() {
        let dir = tempfile::tempdir().unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                "c8",
                json!({
                    "path": "does_not_exist.txt",
                    "oldText": "hello",
                    "newText": "world"
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        let text = result
            .content
            .iter()
            .find_map(|b| match b {
                imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap();
        assert!(text.contains("File not found"));
    }

    #[tokio::test]
    async fn edit_missing_path_error() {
        let dir = tempfile::tempdir().unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                "c9",
                json!({
                    "oldText": "hello",
                    "newText": "world"
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        let text = result
            .content
            .iter()
            .find_map(|b| match b {
                imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap();
        assert!(text.contains("path"));
    }

    #[tokio::test]
    async fn edit_warns_on_unread_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("unread.txt");
        std::fs::write(&file, "original content here\n").unwrap();

        // Use a fresh tracker (file never read)
        let tool = EditTool;
        let result = tool
            .execute(
                "c10",
                json!({
                    "path": "unread.txt",
                    "oldText": "original content",
                    "newText": "changed content"
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(
            !result.is_error,
            "edit should succeed even without prior read"
        );
        let text = result
            .content
            .iter()
            .find_map(|b| match b {
                imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap();
        assert!(
            text.contains("Warning"),
            "expected unread-file warning in output, got: {text}"
        );
    }

    #[tokio::test]
    async fn anchored_edit_replaces_validated_range_and_checkpoints() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("anchored.txt");
        std::fs::write(&file, "alpha\nbeta\ngamma\n").unwrap();
        let ctx = test_ctx(dir.path());
        let lines = ["beta"];
        let anchors = ctx.anchor_store.record_lines(
            &file,
            super::super::stable_hash("alpha\nbeta\ngamma\n"),
            2,
            &lines,
        );

        let result = EditTool
            .execute(
                "c-anchor",
                json!({
                    "path": "anchored.txt",
                    "anchor_start": anchors[0].id,
                    "new_text": "BETA",
                }),
                ctx.clone(),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "alpha\nBETA\ngamma\n"
        );
        assert_eq!(
            ctx.checkpoint_state.original(&file).as_deref(),
            Some("alpha\nbeta\ngamma\n")
        );
        assert_eq!(result.details["anchored"], true);
        assert_eq!(result.details["action"], "edit");
        assert_eq!(result.details["mode"], "anchored");
        assert_eq!(result.details["start_line"], 2);
        assert_eq!(result.details["end_line"], 2);
        assert!(
            result.details["refreshed_anchors"]
                .as_array()
                .unwrap()
                .len()
                >= 3
        );
    }

    #[tokio::test]
    async fn anchored_edit_rejects_stale_anchor_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("stale.txt");
        std::fs::write(&file, "alpha\nbeta\ngamma\n").unwrap();
        let ctx = test_ctx(dir.path());
        let lines = ["beta"];
        let anchors = ctx.anchor_store.record_lines(
            &file,
            super::super::stable_hash("alpha\nbeta\ngamma\n"),
            2,
            &lines,
        );
        std::fs::write(&file, "alpha\nchanged\ngamma\n").unwrap();

        let result = EditTool
            .execute(
                "c-anchor-stale",
                json!({
                    "path": "stale.txt",
                    "anchor_start": anchors[0].id,
                    "new_text": "BETA",
                }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.text_content().unwrap().contains("Stale anchor"));
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "alpha\nchanged\ngamma\n"
        );
    }

    #[tokio::test]
    async fn anchored_edit_dry_run_does_not_write() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("dry-anchor.txt");
        std::fs::write(&file, "alpha\nbeta\n").unwrap();
        let ctx = test_ctx(dir.path());
        let lines = ["beta"];
        let anchors = ctx.anchor_store.record_lines(
            &file,
            super::super::stable_hash("alpha\nbeta\n"),
            2,
            &lines,
        );

        let result = EditTool
            .execute(
                "c-anchor-dry",
                json!({
                    "path": "dry-anchor.txt",
                    "anchor_start": anchors[0].id,
                    "new_text": "BETA",
                    "dry_run": true,
                }),
                ctx.clone(),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "alpha\nbeta\n");
        assert!(ctx.checkpoint_state.checkpoints().is_empty());
        assert!(result.text_content().unwrap().contains("dry run"));
    }

    #[tokio::test]
    async fn edit_with_edits_uses_transaction_path() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("transaction.txt");
        std::fs::write(&file, "alpha\nbeta\n").unwrap();

        let result = EditTool
            .execute(
                "c-transaction",
                json!({
                    "path": "transaction.txt",
                    "edits": [
                        {"oldText": "alpha", "newText": "ALPHA"},
                        {"oldText": "beta", "newText": "BETA"}
                    ]
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "ALPHA\nBETA\n");
        assert_eq!(result.details["transaction"], true);
        assert_eq!(result.details["edit_count"], 2);
    }

    #[tokio::test]
    async fn edit_no_match_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("nope.txt");
        std::fs::write(&file, "some content here\n").unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                "c4",
                json!({
                    "path": "nope.txt",
                    "oldText": "this text does not exist",
                    "newText": "replacement"
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        let text = result
            .content
            .iter()
            .find_map(|b| match b {
                imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap();
        assert!(text.contains("Could not find"));
    }
}
