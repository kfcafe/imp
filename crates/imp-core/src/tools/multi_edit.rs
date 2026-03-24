use async_trait::async_trait;
use serde_json::json;

use super::edit::apply_edit;
use super::{generate_diff, suggest_similar_files, Tool, ToolContext, ToolOutput};
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
        "Apply multiple find-and-replace edits to a single file in one call."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to edit" },
                "edits": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "oldText": { "type": "string" },
                            "newText": { "type": "string" }
                        },
                        "required": ["oldText", "newText"]
                    },
                    "description": "Array of {oldText, newText} pairs"
                }
            },
            "required": ["path", "edits"]
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
        let edits = params["edits"].as_array();

        if raw_path.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: path"));
        }

        let edits = match edits {
            Some(e) if !e.is_empty() => e,
            _ => return Ok(ToolOutput::error("Missing or empty edits array")),
        };

        let path = super::resolve_path(&ctx.cwd, raw_path);

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
        let original = raw_content.replace("\r\n", "\n");
        let has_crlf = raw_content.contains("\r\n");

        // Validate ALL edits first (atomic: all-or-nothing)
        let mut current = original.clone();
        let mut any_fuzzy = false;

        for (i, edit) in edits.iter().enumerate() {
            let old_text = edit["oldText"].as_str().unwrap_or("").replace("\r\n", "\n");
            let new_text = edit["newText"].as_str().unwrap_or("").replace("\r\n", "\n");

            if old_text.is_empty() {
                return Ok(ToolOutput::error(format!(
                    "Edit {}: missing oldText",
                    i + 1
                )));
            }

            match apply_edit(&current, &old_text, &new_text) {
                Ok((new_content, was_fuzzy)) => {
                    if was_fuzzy {
                        any_fuzzy = true;
                    }
                    current = new_content;
                }
                Err(_) => {
                    return Ok(ToolOutput::error(format!(
                        "Edit {} of {} failed: could not find oldText in file (after applying previous edits).\n\
                         oldText starts with: {:?}",
                        i + 1,
                        edits.len(),
                        &old_text[..old_text.len().min(80)]
                    )));
                }
            }
        }

        // All edits validated — write the result
        let diff = generate_diff(raw_path, &original, &current);

        let final_content = if has_crlf {
            current.replace('\n', "\r\n")
        } else {
            current
        };

        tokio::fs::write(&path, &final_content).await?;

        let mut msg = format!("Applied {} edits to {raw_path}\n\n{diff}", edits.len());
        if any_fuzzy {
            msg.push_str("\n(some edits used fuzzy matching)");
        }
        if let Some(warning) = tracker_warning {
            msg.push('\n');
            msg.push_str(&warning);
        }

        Ok(ToolOutput {
            content: vec![imp_llm::ContentBlock::Text { text: msg }],
            details: json!({
                "path": path.display().to_string(),
                "edits_applied": edits.len(),
                "fuzzy_match": any_fuzzy,
            }),
            is_error: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolContext;
    use std::sync::Arc;

    fn test_ctx(dir: &std::path::Path) -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(crate::ui::NullInterface),
            file_cache: Arc::new(crate::tools::FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(crate::tools::FileTracker::new())),
            mode: crate::config::AgentMode::Full,
        }
    }

    #[tokio::test]
    async fn multi_edit_sequential() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("seq.txt");
        std::fs::write(&file, "aaa\nbbb\nccc\n").unwrap();

        let tool = MultiEditTool;
        let result = tool
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
    }

    #[tokio::test]
    async fn multi_edit_atomic_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("atomic.txt");
        std::fs::write(&file, "foo\nbar\nbaz\n").unwrap();

        let tool = MultiEditTool;
        let result = tool
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
        // File should be unchanged (atomic — nothing was written)
        let written = std::fs::read_to_string(&file).unwrap();
        assert_eq!(written, "foo\nbar\nbaz\n");
    }

    #[tokio::test]
    async fn multi_edit_sees_previous_results() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("chain.txt");
        std::fs::write(&file, "hello world\n").unwrap();

        let tool = MultiEditTool;
        // First edit changes "hello" to "goodbye", second edit changes "goodbye world" to "farewell"
        let result = tool
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
        let written = std::fs::read_to_string(&file).unwrap();
        assert_eq!(written, "farewell\n");
    }

    #[tokio::test]
    async fn multi_edit_empty_edits_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("empty_edits.txt");
        std::fs::write(&file, "content\n").unwrap();

        let tool = MultiEditTool;
        let result = tool
            .execute(
                "c5",
                json!({
                    "path": "empty_edits.txt",
                    "edits": []
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
    }

    #[tokio::test]
    async fn multi_edit_missing_path_error() {
        let dir = tempfile::tempdir().unwrap();

        let tool = MultiEditTool;
        let result = tool
            .execute(
                "c6",
                json!({
                    "edits": [{"oldText": "a", "newText": "b"}]
                }),
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

        let tool = MultiEditTool;
        // Each edit depends on the previous: apple→APPLE, APPLE banana→FRUIT, cherry→CHERRY
        let result = tool
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
        let written = std::fs::read_to_string(&file).unwrap();
        assert_eq!(written, "FRUIT CHERRY\n");
    }

    #[tokio::test]
    async fn multi_edit_combined_diff() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("diff.txt");
        std::fs::write(&file, "alpha\nbeta\ngamma\n").unwrap();

        let tool = MultiEditTool;
        let result = tool
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
        let text = result
            .content
            .iter()
            .find_map(|b| match b {
                imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap();
        // Diff should contain both changes
        assert!(text.contains("ALPHA"));
        assert!(text.contains("GAMMA"));
    }
}
