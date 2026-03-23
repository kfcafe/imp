use std::path::Path;

use async_trait::async_trait;
use serde_json::json;

use super::{truncate_head, Tool, ToolContext, ToolOutput};
use crate::error::Result;

const DEFAULT_LIMIT: usize = 1000;
const MAX_OUTPUT_LINES: usize = 2000;
const MAX_OUTPUT_BYTES: usize = 50_000;

pub struct FindTool;

#[async_trait]
impl Tool for FindTool {
    fn name(&self) -> &str {
        "find"
    }
    fn label(&self) -> &str {
        "Find Files"
    }
    fn description(&self) -> &str {
        "Search for files by glob pattern. Returns matching file paths relative to the search directory."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Glob pattern to match files" },
                "path": { "type": "string", "description": "Directory to search in" },
                "limit": { "type": "number", "description": "Maximum number of results" }
            },
            "required": ["pattern"]
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
        let pattern = match params["pattern"].as_str() {
            Some(p) => p,
            None => return Ok(ToolOutput::error("Missing required parameter: pattern")),
        };

        let raw_path = params["path"].as_str().unwrap_or(".");
        let limit = params["limit"].as_u64().unwrap_or(DEFAULT_LIMIT as u64) as usize;

        let search_dir = if Path::new(raw_path).is_absolute() {
            raw_path.into()
        } else {
            ctx.cwd.join(raw_path)
        };

        if !search_dir.exists() {
            return Ok(ToolOutput::error(format!(
                "Directory not found: {}",
                search_dir.display()
            )));
        }

        // Use fd if available, fall back to glob walk
        let results = if has_fd().await {
            find_fd(pattern, &search_dir, limit).await?
        } else {
            find_glob(pattern, &search_dir, limit)?
        };

        if results.is_empty() {
            return Ok(ToolOutput::text("No files found matching pattern"));
        }

        let total = results.len();
        let output = results.join("\n");
        let result = truncate_head(&output, MAX_OUTPUT_LINES, MAX_OUTPUT_BYTES);

        let mut text = result.content;
        if total >= limit {
            text.push_str(&format!(
                "\n\n[{limit} results limit reached. Refine pattern for more.]"
            ));
        }

        Ok(ToolOutput::text(text))
    }
}

async fn has_fd() -> bool {
    tokio::process::Command::new("fd")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn find_fd(
    pattern: &str,
    search_dir: &Path,
    limit: usize,
) -> std::result::Result<Vec<String>, crate::error::Error> {
    let output = tokio::process::Command::new("fd")
        .args([
            "--glob",
            pattern,
            "--max-results",
            &limit.to_string(),
            "--type",
            "f",
        ])
        .current_dir(search_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await?;

    let text = String::from_utf8_lossy(&output.stdout);
    let results: Vec<String> = text
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    Ok(results)
}

fn find_glob(
    pattern: &str,
    search_dir: &Path,
    limit: usize,
) -> std::result::Result<Vec<String>, crate::error::Error> {
    let full_pattern = search_dir.join(pattern);
    let pattern_str = full_pattern.to_string_lossy();

    let mut results = Vec::new();
    for entry in glob::glob(&pattern_str).map_err(|e| crate::error::Error::Tool(e.to_string()))? {
        if results.len() >= limit {
            break;
        }
        if let Ok(path) = entry {
            if path.is_file() {
                let relative = path
                    .strip_prefix(search_dir)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                results.push(relative);
            }
        }
    }

    results.sort();
    Ok(results)
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
    async fn find_matches_glob() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.txt"), "hi").unwrap();
        std::fs::write(dir.path().join("world.rs"), "fn main(){}").unwrap();

        let tool = FindTool;
        let result = tool
            .execute("c1", json!({"pattern": "*.txt"}), test_ctx(dir.path()))
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("hello.txt"));
        assert!(!text.contains("world.rs"));
    }

    #[tokio::test]
    async fn find_no_matches() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.txt"), "hi").unwrap();

        let tool = FindTool;
        let result = tool
            .execute(
                "c2",
                json!({"pattern": "*.nonexistent"}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("No files found"));
    }

    #[tokio::test]
    async fn find_missing_pattern_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let tool = FindTool;
        let result = tool
            .execute("c3", json!({}), test_ctx(dir.path()))
            .await
            .unwrap();

        assert!(result.is_error);
    }
}
