use async_trait::async_trait;
use serde_json::json;

use super::{truncate_head, Tool, ToolContext, ToolOutput};
use crate::error::Result;

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
        "Write full content to a file, creating parent directories automatically. Best for creating new files or intentionally replacing an entire file."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to write" },
                "content": { "type": "string", "description": "Full file content to write" }
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

        if raw_path.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: path"));
        }

        let path = super::resolve_path(&ctx.cwd, raw_path);

        let existed = path.exists();

        // Check for unread or stale file — warn but don't block (only relevant for overwrites).
        let tracker_warning = if existed {
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
        } else {
            None
        };

        // Create parent directories
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

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

        let bytes_written = normalized.len();
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

        let mut warnings = Vec::new();
        if let Some(warning) = tracker_warning {
            warnings.push(warning);
        }

        let mut text = summary.clone();
        for warning in &warnings {
            text.push('\n');
            text.push_str(warning);
        }

        Ok(ToolOutput {
            content: vec![imp_llm::ContentBlock::Text { text }],
            details: json!({
                "path": display,
                "bytes": bytes_written,
                "created": !existed,
                "summary": summary,
                "warnings": warnings,
                "display_content": display_content,
                "display_note": display_note,
            }),
            is_error: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolContext;
    use std::path::Path;
    use std::sync::Arc;

    fn test_ctx(dir: &Path) -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(crate::ui::NullInterface),
            file_cache: Arc::new(crate::tools::FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(crate::tools::FileTracker::new())),
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
        }
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
