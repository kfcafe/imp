use std::path::Path;

use async_trait::async_trait;
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
        "Edit a file. Single: oldText+newText. Multi: edits array of {oldText, newText}."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to edit" },
                "oldText": { "type": "string", "description": "Exact text to find and replace (single edit)" },
                "newText": { "type": "string", "description": "Replacement text (single edit)" },
                "edits": { "type": "array", "description": "Array of {oldText, newText} pairs, applied sequentially (multi edit)", "items": { "type": "object", "properties": { "oldText": { "type": "string" }, "newText": { "type": "string" } }, "required": ["oldText", "newText"] } }
            },
            "required": ["path"]
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
        let old_text = params["oldText"].as_str().unwrap_or("");
        let new_text = params["newText"].as_str().unwrap_or("");

        if raw_path.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: path"));
        }
        if old_text.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: oldText"));
        }

        let path = if Path::new(raw_path).is_absolute() {
            std::path::PathBuf::from(raw_path)
        } else {
            ctx.cwd.join(raw_path)
        };

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

        let raw_content = tokio::fs::read_to_string(&path).await?;

        // Normalize to LF for internal processing
        let content = raw_content.replace("\r\n", "\n");
        let has_crlf = raw_content.contains("\r\n");
        let old_normalized = old_text.replace("\r\n", "\n");
        let new_normalized = new_text.replace("\r\n", "\n");

        let (new_content, was_fuzzy) = match apply_edit(&content, &old_normalized, &new_normalized)
        {
            Ok(v) => v,
            Err(output) => return Ok(output),
        };

        let diff = generate_diff(raw_path, &content, &new_content);

        // Restore original line endings if needed
        let final_content = if has_crlf {
            new_content.replace('\n', "\r\n")
        } else {
            new_content
        };

        tokio::fs::write(&path, &final_content).await?;
        let mut msg = diff;
        if was_fuzzy {
            msg.push_str(
                "\n(matched using fuzzy matching: trailing whitespace/unicode normalized)",
            );
        }

        Ok(ToolOutput {
            content: vec![imp_llm::ContentBlock::Text { text: msg }],
            details: json!({
                "path": path.display().to_string(),
                "fuzzy_match": was_fuzzy,
            }),
            is_error: false,
        })
    }
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
    let preview_len = 200.min(content.len());
    let preview = &content[..preview_len];
    let msg = format!(
        "Could not find the specified text to replace.\n\
         First {preview_len} chars of file:\n{preview}"
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
        ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(crate::ui::NullInterface),
            file_cache: Arc::new(crate::tools::FileCache::new()),
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
        assert!(text.contains("oldText"));
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
