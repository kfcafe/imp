use async_trait::async_trait;
use serde_json::json;

use super::{truncate_head, Tool, ToolContext, ToolOutput};
use crate::error::Result;

const DEFAULT_LIMIT: usize = 500;
const MAX_OUTPUT_LINES: usize = 2000;
const MAX_OUTPUT_BYTES: usize = 50_000;

pub struct LsTool;

#[async_trait]
impl Tool for LsTool {
    fn name(&self) -> &str {
        "ls"
    }
    fn label(&self) -> &str {
        "List Directory"
    }
    fn description(&self) -> &str {
        "List directory contents. Returns entries sorted alphabetically, with '/' suffix for directories."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory to list" },
                "limit": { "type": "number", "description": "Maximum number of entries" }
            }
        })
    }
    fn is_readonly(&self) -> bool {
        true
    }

    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let raw_path = params["path"].as_str().unwrap_or(".");
        let limit = params["limit"]
            .as_u64()
            .unwrap_or(DEFAULT_LIMIT as u64)
            .max(1) as usize;

        let dir = super::resolve_path(&ctx.cwd, raw_path);

        if !dir.exists() {
            return Ok(ToolOutput::error(format!(
                "Directory not found: {}",
                dir.display()
            )));
        }
        if !dir.is_dir() {
            return Ok(ToolOutput::error(format!(
                "Not a directory: {}",
                dir.display()
            )));
        }

        let mut entries: Vec<String> = Vec::new();
        let mut read_dir = tokio::fs::read_dir(&dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry
                .file_type()
                .await
                .map(|ft| ft.is_dir())
                .unwrap_or(false);
            if is_dir {
                entries.push(format!("{name}/"));
            } else {
                entries.push(name);
            }
        }

        entries.sort_by_key(|a| a.to_lowercase());

        if entries.is_empty() {
            return Ok(ToolOutput::text("(empty directory)"));
        }

        let truncated_entries = if entries.len() > limit {
            let total = entries.len();
            let mut out: Vec<String> = entries.into_iter().take(limit).collect();
            out.push(format!(
                "\n[{limit} of {total} entries shown. Use limit={total} for more.]"
            ));
            out
        } else {
            entries
        };

        let output = truncated_entries.join("\n");
        let result = truncate_head(&output, MAX_OUTPUT_LINES, MAX_OUTPUT_BYTES);

        Ok(ToolOutput::text(result.content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn extract_text(output: &ToolOutput) -> String {
        output
            .content
            .iter()
            .filter_map(|b| match b {
                imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    #[tokio::test]
    async fn ls_lists_files_and_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.txt"), "hi").unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();

        let tool = LsTool;
        let result = tool
            .execute("c1", json!({}), test_ctx(dir.path()))
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("hello.txt"));
        assert!(text.contains("subdir/"));
    }

    #[tokio::test]
    async fn ls_missing_dir_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let tool = LsTool;
        let result = tool
            .execute("c2", json!({"path": "nonexistent"}), test_ctx(dir.path()))
            .await
            .unwrap();

        assert!(result.is_error);
    }

    #[tokio::test]
    async fn ls_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let tool = LsTool;
        let result = tool
            .execute("c3", json!({}), test_ctx(dir.path()))
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("empty directory"));
    }

    #[tokio::test]
    async fn ls_respects_limit() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..10 {
            std::fs::write(dir.path().join(format!("file{i}.txt")), "").unwrap();
        }

        let tool = LsTool;
        let result = tool
            .execute("c4", json!({"limit": 3}), test_ctx(dir.path()))
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("3 of 10 entries shown"));
    }
}
