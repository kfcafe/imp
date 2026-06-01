use async_trait::async_trait;
use imp_llm::truncate_chars_with_suffix;
use serde_json::json;

use super::edit::apply_edit;
use super::{
    generate_diff, line_change_counts, suggest_similar_files, Tool, ToolContext, ToolOutput,
};
use crate::error::Result;

pub struct MultiEditTool;

#[async_trait]
impl Tool for MultiEditTool {
    fn name(&self) -> &str {
        "multi_edit"
    }
    fn label(&self) -> &str {
        "Multi Edit"
    }
    fn description(&self) -> &str {
        "Legacy compatibility shim for multi-edit transactions. Prefer the canonical edit tool with edits[]."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Default path to edit; may be omitted when each edit includes its own path" },
                "dry_run": { "type": "boolean", "description": "Validate and return combined diff without writing files" },
                "edits": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Optional per-edit path for multi-file transactions" },
                            "old_text": { "type": "string" },
                            "new_text": { "type": "string" }
                        },
                        "required": ["old_text", "new_text"]
                    },
                    "description": "Array of {old_text, new_text, path?} edits validated before any file is written"
                }
            },
            "required": ["edits"]
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
        let dry_run = params
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .or_else(|| params.get("dryRun").and_then(|v| v.as_bool()))
            .unwrap_or(false);
        let edits = match params["edits"].as_array() {
            Some(e) if !e.is_empty() => e,
            _ => return Ok(ToolOutput::error("Missing or empty edits array")),
        };

        let mut edits_by_path: std::collections::BTreeMap<String, Vec<&serde_json::Value>> =
            std::collections::BTreeMap::new();
        for edit in edits {
            let edit_path = edit["path"].as_str().unwrap_or(raw_path);
            if edit_path.is_empty() {
                return Ok(ToolOutput::error(
                    "Missing required parameter: path (top-level path or per-edit path)",
                ));
            }
            edits_by_path
                .entry(edit_path.to_string())
                .or_default()
                .push(edit);
        }

        let mut prepared = Vec::new();
        let mut any_fuzzy = false;
        let mut total_edits = 0usize;
        let mut warnings = Vec::new();

        for (edit_path, file_edits) in edits_by_path {
            let path = super::resolve_path(&ctx.cwd, &edit_path);
            if let Err(error) = ctx.check_write_path(&path) {
                return Ok(ToolOutput::error(error));
            }
            if !path.exists() {
                let suggestions = suggest_similar_files(&ctx.cwd, &edit_path);
                let mut msg = format!("File not found: {}", path.display());
                if !suggestions.is_empty() {
                    msg.push_str("\n\nDid you mean:");
                    for s in &suggestions {
                        msg.push_str(&format!("\n  {s}"));
                    }
                }
                return Ok(ToolOutput::error(msg));
            }

            if let Some(warning) = tracker_warning(&ctx, &path) {
                warnings.push(warning);
            }

            let raw_content = tokio::fs::read_to_string(&path).await?;
            let original = raw_content.replace("\r\n", "\n");
            let has_crlf = raw_content.contains("\r\n");

            if let Err(error) = reject_overlapping_exact_edits(&edit_path, &original, &file_edits) {
                return Ok(ToolOutput::error(error.to_string()));
            }

            let mut current = original.clone();
            for (i, edit) in file_edits.iter().enumerate() {
                let old_text = edit
                    .get("old_text")
                    .and_then(|v| v.as_str())
                    .or_else(|| edit.get("oldText").and_then(|v| v.as_str()))
                    .unwrap_or("")
                    .replace("\r\n", "\n");
                let new_text = edit
                    .get("new_text")
                    .and_then(|v| v.as_str())
                    .or_else(|| edit.get("newText").and_then(|v| v.as_str()))
                    .unwrap_or("")
                    .replace("\r\n", "\n");
                if old_text.is_empty() {
                    return Ok(ToolOutput::error(format!(
                        "Edit {} in {edit_path}: missing old_text",
                        i + 1
                    )));
                }
                match apply_edit(&current, &old_text, &new_text) {
                    Ok((new_content, was_fuzzy)) => {
                        any_fuzzy |= was_fuzzy;
                        current = new_content;
                    }
                    Err(_) => {
                        return Ok(ToolOutput::error(format!(
                            "Edit {} of {} failed in {edit_path}: could not find old_text in file (after applying previous edits).\nold_text starts with: {:?}",
                            i + 1,
                            file_edits.len(),
                            truncate_chars_with_suffix(&old_text, 80, "")
                        )));
                    }
                }
            }

            total_edits += file_edits.len();
            let diff = generate_diff(&edit_path, &original, &current);
            let (lines_added, lines_removed) = line_change_counts(&original, &current);
            let final_content = if has_crlf {
                current.replace('\n', "\r\n")
            } else {
                current.clone()
            };
            prepared.push(PreparedEditFile {
                input_path: edit_path,
                path,
                final_content,
                diff,
                lines_added,
                lines_removed,
                edit_count: file_edits.len(),
            });
        }

        let touched_paths = prepared
            .iter()
            .map(|prepared| prepared.path.clone())
            .collect::<Vec<_>>();
        if !dry_run {
            ctx.checkpoint_state
                .snapshot_paths(&touched_paths, Some("multi_edit transaction".to_string()))?;
            for prepared in &prepared {
                tokio::fs::write(&prepared.path, &prepared.final_content).await?;
                if let Ok(mut tracker) = ctx.file_tracker.lock() {
                    tracker.record_read(&prepared.path);
                }
            }
        }

        let combined_diff = prepared
            .iter()
            .map(|prepared| prepared.diff.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let mut msg = format!(
            "Validated {} edits across {} file(s) as one transaction",
            total_edits,
            prepared.len()
        );
        if dry_run {
            msg.push_str(" (dry run: no changes written)");
        } else {
            msg.push_str(" and applied them");
        }
        msg.push_str("\n\n");
        msg.push_str(&combined_diff);
        if any_fuzzy {
            msg.push_str("\n(some edits used fuzzy matching)");
        }
        for warning in &warnings {
            msg.push('\n');
            msg.push_str(warning);
        }

        Ok(ToolOutput {
            content: vec![imp_llm::ContentBlock::Text { text: msg }],
            details: json!({
                "transaction": true,
                "dry_run": dry_run,
                "files": prepared.iter().map(|prepared| json!({
                    "path": prepared.path.display().to_string(),
                    "input_path": prepared.input_path,
                    "edit_count": prepared.edit_count,
                    "status": "modified",
                    "lines_added": prepared.lines_added,
                    "lines_removed": prepared.lines_removed,
                })).collect::<Vec<_>>(),
                "lines_added": prepared.iter().map(|prepared| prepared.lines_added).sum::<usize>(),
                "lines_removed": prepared.iter().map(|prepared| prepared.lines_removed).sum::<usize>(),
                "edit_count": total_edits,
                "edits_applied": if dry_run { 0 } else { total_edits },
                "fuzzy_match": any_fuzzy,
                "checkpoint_created": !dry_run,
            }),
            is_error: false,
        })
    }
}

struct PreparedEditFile {
    input_path: String,
    path: std::path::PathBuf,
    final_content: String,
    diff: String,
    lines_added: usize,
    lines_removed: usize,
    edit_count: usize,
}

fn tracker_warning(ctx: &ToolContext, path: &std::path::Path) -> Option<String> {
    let tracker = ctx.file_tracker.lock().ok()?;
    if !tracker.was_read(path) {
        Some(format!(
            "Warning: editing {} without reading it first. Consider reading to verify current content.",
            path.display()
        ))
    } else if tracker.is_stale(path) {
        Some(format!(
            "Warning: {} was modified externally since last read. Re-read to verify current content.",
            path.display()
        ))
    } else {
        None
    }
}

fn reject_overlapping_exact_edits(
    edit_path: &str,
    original: &str,
    file_edits: &[&serde_json::Value],
) -> Result<()> {
    let mut exact_ranges = Vec::new();
    for (i, edit) in file_edits.iter().enumerate() {
        let old_text = edit
            .get("old_text")
            .and_then(|v| v.as_str())
            .or_else(|| edit.get("oldText").and_then(|v| v.as_str()))
            .unwrap_or("")
            .replace("\r\n", "\n");
        if old_text.is_empty() {
            continue;
        }
        if let Some(pos) = original.find(&old_text) {
            exact_ranges.push((pos, pos + old_text.len(), i + 1));
        }
    }
    exact_ranges.sort_by_key(|(start, _, _)| *start);
    for pair in exact_ranges.windows(2) {
        let (_, prev_end, prev_idx) = pair[0];
        let (next_start, _, next_idx) = pair[1];
        if next_start < prev_end {
            return Err(crate::error::Error::Tool(format!(
                "Overlapping edits rejected in {edit_path}: edit {prev_idx} overlaps edit {next_idx}. No changes made."
            )));
        }
    }
    Ok(())
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
            supporting_provenance: Vec::new(),
        }
    }

    #[tokio::test]
    async fn multi_edit_sequential() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("seq.txt");
        std::fs::write(&file, "aaa\nbbb\nccc\n").unwrap();

        let result = MultiEditTool
            .execute(
                "c1",
                json!({
                    "path": "seq.txt",
                    "edits": [
                        {"oldText": "aaa", "newText": "AAA"},
                        {"oldText": "bbb", "newText": "BBB"}
                    ]
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let written = std::fs::read_to_string(&file).unwrap();
        assert!(written.contains("AAA"));
        assert!(written.contains("BBB"));
        assert!(written.contains("ccc"));
        assert_eq!(result.details["transaction"], true);
    }

    #[tokio::test]
    async fn multi_edit_atomic_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("atomic.txt");
        std::fs::write(&file, "foo\nbar\nbaz\n").unwrap();

        let result = MultiEditTool
            .execute(
                "c2",
                json!({
                    "path": "atomic.txt",
                    "edits": [
                        {"oldText": "foo", "newText": "FOO"},
                        {"oldText": "nonexistent", "newText": "X"}
                    ]
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "foo\nbar\nbaz\n");
    }

    #[tokio::test]
    async fn multi_edit_sees_previous_results() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("chain.txt");
        std::fs::write(&file, "hello world\n").unwrap();

        let result = MultiEditTool
            .execute(
                "c3",
                json!({
                    "path": "chain.txt",
                    "edits": [
                        {"oldText": "hello", "newText": "goodbye"},
                        {"oldText": "goodbye world", "newText": "farewell"}
                    ]
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "farewell\n");
    }

    #[tokio::test]
    async fn multi_edit_creates_checkpoint_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("checkpoint.txt");
        std::fs::write(&file, "foo\nbar\n").unwrap();

        let ctx = test_ctx(dir.path());
        let checkpoint_state = ctx.checkpoint_state.clone();
        let result = MultiEditTool
            .execute(
                "c-checkpoint",
                json!({
                    "path": "checkpoint.txt",
                    "edits": [
                        {"oldText": "foo", "newText": "FOO"},
                        {"oldText": "bar", "newText": "BAR"}
                    ]
                }),
                ctx,
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(
            checkpoint_state.original(&file).as_deref(),
            Some("foo\nbar\n")
        );
        assert_eq!(checkpoint_state.checkpoints().len(), 1);
        assert_eq!(result.details["checkpoint_created"], true);
    }

    #[tokio::test]
    async fn multi_edit_empty_edits_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("empty_edits.txt");
        std::fs::write(&file, "content\n").unwrap();

        let result = MultiEditTool
            .execute(
                "c5",
                json!({"path": "empty_edits.txt", "edits": []}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
    }

    #[tokio::test]
    async fn multi_edit_missing_path_error() {
        let dir = tempfile::tempdir().unwrap();

        let result = MultiEditTool
            .execute(
                "c6",
                json!({"edits": [{"oldText": "a", "newText": "b"}]}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
    }

    #[tokio::test]
    async fn multi_edit_chained_three_edits() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("chain3.txt");
        std::fs::write(&file, "apple banana cherry\n").unwrap();

        let result = MultiEditTool
            .execute(
                "c7",
                json!({
                    "path": "chain3.txt",
                    "edits": [
                        {"oldText": "apple", "newText": "APPLE"},
                        {"oldText": "APPLE banana", "newText": "FRUIT"},
                        {"oldText": "cherry", "newText": "CHERRY"}
                    ]
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "FRUIT CHERRY\n");
    }

    #[tokio::test]
    async fn multi_edit_combined_diff() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("diff.txt");
        std::fs::write(&file, "alpha\nbeta\ngamma\n").unwrap();

        let result = MultiEditTool
            .execute(
                "c4",
                json!({
                    "path": "diff.txt",
                    "edits": [
                        {"oldText": "alpha", "newText": "ALPHA"},
                        {"oldText": "gamma", "newText": "GAMMA"}
                    ]
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = result.text_content().unwrap();
        assert!(text.contains("ALPHA"));
        assert!(text.contains("GAMMA"));
    }

    #[tokio::test]
    async fn multi_edit_can_edit_two_files_transactionally() {
        let dir = tempfile::tempdir().unwrap();
        let one = dir.path().join("one.txt");
        let two = dir.path().join("two.txt");
        std::fs::write(&one, "alpha\n").unwrap();
        std::fs::write(&two, "beta\n").unwrap();

        let result = MultiEditTool
            .execute(
                "c-multi-file",
                json!({
                    "edits": [
                        {"path": "one.txt", "oldText": "alpha", "newText": "ALPHA"},
                        {"path": "two.txt", "oldText": "beta", "newText": "BETA"}
                    ]
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(std::fs::read_to_string(&one).unwrap(), "ALPHA\n");
        assert_eq!(std::fs::read_to_string(&two).unwrap(), "BETA\n");
        assert_eq!(result.details["files"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn multi_edit_rejects_overlaps_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("overlap.txt");
        std::fs::write(&file, "abcdef\n").unwrap();

        let result = MultiEditTool
            .execute(
                "c-overlap",
                json!({
                    "path": "overlap.txt",
                    "edits": [
                        {"oldText": "abc", "newText": "ABC"},
                        {"oldText": "bc", "newText": "BC"}
                    ]
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.text_content().unwrap().contains("Overlapping edits"));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "abcdef\n");
    }

    #[tokio::test]
    async fn multi_edit_dry_run_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("dry.txt");
        std::fs::write(&file, "alpha\n").unwrap();
        let ctx = test_ctx(dir.path());

        let result = MultiEditTool
            .execute(
                "c-dry",
                json!({
                    "path": "dry.txt",
                    "dryRun": true,
                    "edits": [{"oldText": "alpha", "newText": "ALPHA"}]
                }),
                ctx.clone(),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "alpha\n");
        assert!(ctx.checkpoint_state.checkpoints().is_empty());
        assert_eq!(result.details["dry_run"], true);
    }
}
