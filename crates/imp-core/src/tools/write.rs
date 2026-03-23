use std::path::Path;

use async_trait::async_trait;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
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
        "Write content to a file. Creates parent directories automatically."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to write" },
                "content": { "type": "string", "description": "Content to write" }
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

        let path = if Path::new(raw_path).is_absolute() {
            std::path::PathBuf::from(raw_path)
        } else {
            ctx.cwd.join(raw_path)
        };

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

        let mut text = format!("{display}: {bytes_written} bytes {action}");
        if let Some(warning) = tracker_warning {
            text.push('\n');
            text.push_str(&warning);
        }

        Ok(ToolOutput {
            content: vec![imp_llm::ContentBlock::Text { text }],
            details: json!({
                "path": display,
                "bytes": bytes_written,
                "created": !existed,
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

    fn test_ctx(dir: &Path) -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(crate::ui::NullInterface),
            file_cache: Arc::new(crate::tools::FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(crate::tools::FileTracker::new())),
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
}
