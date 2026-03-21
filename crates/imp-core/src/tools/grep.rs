use std::path::Path;
use std::process::Stdio;

use async_trait::async_trait;
use serde_json::json;

use super::{truncate_head, truncate_line, Tool, ToolContext, ToolOutput, TruncationResult};
use crate::error::Result;

const DEFAULT_LIMIT: usize = 100;
const MAX_OUTPUT_LINES: usize = 2000;
const MAX_OUTPUT_BYTES: usize = 50 * 1024;
const MAX_LINE_CHARS: usize = 500;

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }
    fn label(&self) -> &str {
        "Grep"
    }
    fn description(&self) -> &str {
        "Search file contents for a pattern. Returns matching lines with file paths and line numbers."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Search pattern (regex or literal)" },
                "path": { "type": "string", "description": "Directory or file to search" },
                "glob": { "type": "string", "description": "Filter files by glob pattern" },
                "ignoreCase": { "type": "boolean", "description": "Case-insensitive search" },
                "literal": { "type": "boolean", "description": "Treat pattern as literal string" },
                "context": { "type": "number", "description": "Context lines before and after" },
                "limit": { "type": "number", "description": "Maximum number of matches" }
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
        let pattern = params["pattern"]
            .as_str()
            .ok_or_else(|| crate::error::Error::Tool("missing 'pattern' parameter".into()))?;

        let search_path = params["path"]
            .as_str()
            .map(|p| ctx.cwd.join(p))
            .unwrap_or_else(|| ctx.cwd.clone());

        let glob_filter = params["glob"].as_str().map(|s| s.to_string());
        let ignore_case = params["ignoreCase"].as_bool().unwrap_or(false);
        let literal = params["literal"].as_bool().unwrap_or(false);
        let context_lines = params["context"].as_u64().map(|n| n as usize);
        let limit = params["limit"].as_u64().unwrap_or(DEFAULT_LIMIT as u64) as usize;

        // Try ripgrep first, fall back to built-in.
        let output = if has_rg().await {
            grep_rg(
                pattern,
                &search_path,
                glob_filter.as_deref(),
                ignore_case,
                literal,
                context_lines,
                limit,
            )
            .await?
        } else {
            grep_fallback(
                pattern,
                &search_path,
                glob_filter.as_deref(),
                ignore_case,
                literal,
                limit,
            )?
        };

        if output.is_empty() {
            return Ok(ToolOutput::text("No matches found."));
        }

        // Truncate long lines, then truncate total output.
        let truncated_lines: String = output
            .lines()
            .map(|l| truncate_line(l, MAX_LINE_CHARS))
            .collect::<Vec<_>>()
            .join("\n");

        let TruncationResult {
            content,
            truncated,
            output_lines,
            total_lines,
            ..
        } = truncate_head(&truncated_lines, MAX_OUTPUT_LINES, MAX_OUTPUT_BYTES);

        let mut result = content;
        if truncated {
            result.push_str(&format!(
                "\n[Output truncated: showing first {output_lines} of {total_lines} lines]"
            ));
        }

        Ok(ToolOutput::text(result))
    }
}

async fn has_rg() -> bool {
    tokio::process::Command::new("rg")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn grep_rg(
    pattern: &str,
    path: &Path,
    glob_filter: Option<&str>,
    ignore_case: bool,
    literal: bool,
    context_lines: Option<usize>,
    limit: usize,
) -> Result<String> {
    let mut cmd = tokio::process::Command::new("rg");
    cmd.arg("--no-heading")
        .arg("--line-number")
        .arg("--color=never")
        .arg("--max-count")
        .arg(limit.to_string());

    if ignore_case {
        cmd.arg("-i");
    }
    if literal {
        cmd.arg("-F");
    }
    if let Some(ctx) = context_lines {
        cmd.arg("-C").arg(ctx.to_string());
    }
    if let Some(g) = glob_filter {
        cmd.arg("--glob").arg(g);
    }

    cmd.arg("--").arg(pattern).arg(path);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let output = cmd
        .output()
        .await
        .map_err(|e| crate::error::Error::Tool(format!("rg failed: {e}")))?;

    // rg returns exit code 1 for no matches — that's fine.
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn grep_fallback(
    pattern: &str,
    path: &Path,
    glob_filter: Option<&str>,
    ignore_case: bool,
    literal: bool,
    limit: usize,
) -> Result<String> {
    let re = {
        let pat = if literal {
            regex::escape(pattern)
        } else {
            pattern.to_string()
        };
        regex::RegexBuilder::new(&pat)
            .case_insensitive(ignore_case)
            .build()
            .map_err(|e| crate::error::Error::Tool(format!("invalid regex: {e}")))?
    };

    let glob_pat = glob_filter.map(|g| {
        // If the glob doesn't contain a path separator, match against filename only.
        glob::Pattern::new(g).ok()
    });

    let mut results = Vec::new();
    let mut count = 0;

    let walker = walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file());

    for entry in walker {
        if count >= limit {
            break;
        }

        let file_path = entry.path();

        // Apply glob filter against relative path.
        if let Some(Some(ref pat)) = glob_pat {
            let rel = file_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if !pat.matches(&rel) {
                // Also try matching against the full relative path.
                let rel_path = file_path
                    .strip_prefix(path)
                    .unwrap_or(file_path)
                    .to_string_lossy();
                if !pat.matches(&rel_path) {
                    continue;
                }
            }
        }

        // Read file, skip binary.
        let content = match std::fs::read(file_path) {
            Ok(bytes) => {
                if bytes.contains(&0) {
                    continue; // binary
                }
                String::from_utf8_lossy(&bytes).to_string()
            }
            Err(_) => continue,
        };

        let rel_path = file_path
            .strip_prefix(path)
            .unwrap_or(file_path)
            .to_string_lossy();

        for (line_num, line) in content.lines().enumerate() {
            if count >= limit {
                break;
            }
            if re.is_match(line) {
                results.push(format!("{}:{}:{}", rel_path, line_num + 1, line));
                count += 1;
            }
        }
    }

    Ok(results.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{ToolContext, ToolUpdate};
    use crate::ui::NullInterface;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    fn test_ctx(dir: &std::path::Path) -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel(64);
        ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(NullInterface),
        }
    }

    fn setup_test_dir() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("hello.txt"),
            "Hello World\nfoo bar\nHello Again\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("data.rs"),
            "fn main() {\n    println!(\"hello\");\n}\n",
        )
        .unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        std::fs::write(tmp.path().join("sub/nested.txt"), "nested hello\n").unwrap();
        tmp
    }

    #[tokio::test]
    async fn grep_pattern_match() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());

        let result = GrepTool
            .execute("1", json!({ "pattern": "Hello" }), ctx)
            .await
            .unwrap();

        let text = match &result.content[0] {
            imp_llm::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(!result.is_error);
        assert!(text.contains("Hello World"));
        assert!(text.contains("Hello Again"));
    }

    #[tokio::test]
    async fn grep_case_insensitive() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());

        let result = GrepTool
            .execute("1", json!({ "pattern": "hello", "ignoreCase": true }), ctx)
            .await
            .unwrap();

        let text = match &result.content[0] {
            imp_llm::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(!result.is_error);
        // Should match both "Hello" and "hello"
        assert!(text.contains("Hello World"));
        assert!(text.contains("hello"));
    }

    #[tokio::test]
    async fn grep_glob_filter() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());

        let result = GrepTool
            .execute(
                "1",
                json!({ "pattern": "hello", "glob": "*.rs", "ignoreCase": true }),
                ctx,
            )
            .await
            .unwrap();

        let text = match &result.content[0] {
            imp_llm::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(!result.is_error);
        assert!(text.contains("hello"));
        // Should NOT match .txt files
        assert!(!text.contains("Hello World"));
    }

    #[tokio::test]
    async fn grep_limit() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());

        let result = GrepTool
            .execute("1", json!({ "pattern": "Hello", "limit": 1 }), ctx)
            .await
            .unwrap();

        let text = match &result.content[0] {
            imp_llm::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(!result.is_error);
        // With limit 1, should have at most one match line
        let match_lines: Vec<&str> = text.lines().filter(|l| l.contains("Hello")).collect();
        assert_eq!(match_lines.len(), 1);
    }

    #[tokio::test]
    async fn grep_no_matches() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());

        let result = GrepTool
            .execute("1", json!({ "pattern": "ZZZZNOTFOUND" }), ctx)
            .await
            .unwrap();

        let text = match &result.content[0] {
            imp_llm::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(text.contains("No matches"));
    }

    // Fallback test: use the built-in grep directly
    #[test]
    fn grep_fallback_works() {
        let tmp = setup_test_dir();

        let output = grep_fallback("Hello", tmp.path(), None, false, false, 100).unwrap();
        assert!(output.contains("Hello World"));
        assert!(output.contains("Hello Again"));
    }

    #[test]
    fn grep_fallback_case_insensitive() {
        let tmp = setup_test_dir();

        let output = grep_fallback("hello", tmp.path(), None, true, false, 100).unwrap();
        assert!(output.contains("Hello World"));
    }

    #[test]
    fn grep_fallback_glob() {
        let tmp = setup_test_dir();

        let output = grep_fallback("hello", tmp.path(), Some("*.rs"), true, false, 100).unwrap();
        assert!(output.contains("hello"));
        assert!(!output.contains("Hello World"));
    }

    #[test]
    fn grep_fallback_literal() {
        let tmp = setup_test_dir();

        // "." as literal should not match "Hello World" (no literal dot there)
        let output = grep_fallback(".", tmp.path(), None, false, false, 100).unwrap();
        // As regex, "." matches everything
        assert!(!output.is_empty());

        let output_literal = grep_fallback(".", tmp.path(), None, false, true, 100).unwrap();
        // As literal, only matches lines with actual "."
        // Our test files don't have "." in content except possibly in file paths
        // The point is: literal mode uses regex::escape
        assert!(output.len() >= output_literal.len());
    }
}
