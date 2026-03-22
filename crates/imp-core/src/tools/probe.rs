//! Probe tools — code search and extraction via the `probe` CLI.
//!
//! ProbeTool is the unified entry point with action=search/extract.
//! Shells out to `probe search` and `probe extract` commands.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;

use super::{truncate_head, truncate_line, Tool, ToolContext, ToolOutput, TruncationResult};
use crate::error::Result;

const DEFAULT_MAX_RESULTS: usize = 10;
const MAX_OUTPUT_LINES: usize = 2000;
const MAX_OUTPUT_BYTES: usize = 50 * 1024;
const MAX_LINE_CHARS: usize = 500;

// ── unified probe tool ──────────────────────────────────────────────

pub struct ProbeTool;

#[async_trait]
impl Tool for ProbeTool {
    fn name(&self) -> &str {
        "probe"
    }
    fn label(&self) -> &str {
        "Probe"
    }
    fn description(&self) -> &str {
        "Code search and extraction via tree-sitter AST. action=search for semantic search, action=extract to get complete code blocks by file:line or file#symbol."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["search", "extract"], "description": "search: find code matching query. extract: get blocks by location." },
                "query": { "type": "string", "description": "Search query with AND/OR/NOT (search)" },
                "targets": { "type": "array", "items": { "type": "string" }, "description": "file:line or file#symbol targets (extract)" },
                "path": { "type": "string", "description": "Directory or file to search" },
                "language": { "type": "string", "description": "Filter by language" },
                "maxResults": { "type": "number", "description": "Max results (default: 10)" },
                "exact": { "type": "boolean", "description": "Exact match without stemming" },
                "context": { "type": "number", "description": "Context lines (extract)" }
            },
            "required": ["action"]
        })
    }
    fn is_readonly(&self) -> bool {
        true
    }
    async fn execute(
        &self,
        call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        match params["action"].as_str() {
            Some("search") => execute_search(call_id, params, ctx).await,
            Some("extract") => execute_extract(call_id, params, ctx).await,
            Some(other) => Ok(ToolOutput::error(format!("Unknown probe action: {other}"))),
            None => Ok(ToolOutput::error("Missing 'action' parameter")),
        }
    }
}

// ── search ──────────────────────────────────────────────────────────

async fn execute_search(
    _call_id: &str,
    params: serde_json::Value,
    ctx: ToolContext,
) -> Result<ToolOutput> {
    let query = match params["query"].as_str() {
        Some(query) => query,
        None => return Ok(ToolOutput::error("missing 'query' parameter")),
    };

    let search_path = resolve_path(&ctx, params["path"].as_str());
    let max_results = params["maxResults"]
        .as_u64()
        .unwrap_or(DEFAULT_MAX_RESULTS as u64);

    let mut args = vec![
        "search".to_string(),
        query.to_string(),
        search_path.display().to_string(),
        "--max-results".to_string(),
        max_results.to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];

    if let Some(language) = params["language"].as_str() {
        args.push("--language".to_string());
        args.push(language.to_string());
    }
    if params["exact"].as_bool().unwrap_or(false) {
        args.push("--exact".to_string());
    }
    if params["allowTests"].as_bool().unwrap_or(false) {
        args.push("--allow-tests".to_string());
    }
    if let Some(max_tokens) = params["maxTokens"].as_u64() {
        args.push("--max-tokens".to_string());
        args.push(max_tokens.to_string());
    }

    let output = match run_command_candidates(&["probe"], &args, &ctx.cwd).await {
        Ok(output) => output,
        Err(CommandFailure::MissingBinary) => {
            return Ok(ToolOutput::error(
                "probe requires the 'probe' CLI. Install: cargo install probe-search",
            ));
        }
        Err(CommandFailure::Spawn(error)) => {
            return Ok(ToolOutput::error(format!("failed to run probe: {error}")));
        }
    };

    if !output.status.success() {
        return Ok(ToolOutput::error(command_error_message(
            "probe search failed",
            &output,
        )));
    }

    let parsed = match parse_json_output(&output.stdout) {
        Some(value) => value,
        None => {
            return Ok(ToolOutput::error(format!(
                "probe search returned invalid JSON: {}",
                summarize_command_output(&output)
            )));
        }
    };

    Ok(ToolOutput::text(truncate_output(format_probe_results(
        &parsed,
        &ctx.cwd,
        "No matches found.",
    ))))
}

// ── extract ─────────────────────────────────────────────────────────

async fn execute_extract(
    _call_id: &str,
    params: serde_json::Value,
    ctx: ToolContext,
) -> Result<ToolOutput> {
    let targets = match params["targets"].as_array() {
        Some(targets) if !targets.is_empty() => targets,
        _ => return Ok(ToolOutput::error("missing 'targets' parameter")),
    };

    let context = params["context"].as_u64().unwrap_or(0);

    let mut args = vec![
        "extract".to_string(),
        "--context".to_string(),
        context.to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];

    for target in targets {
        match target.as_str() {
            Some(target) => args.push(resolve_extract_target(&ctx.cwd, target)),
            None => return Ok(ToolOutput::error("'targets' must contain only strings")),
        }
    }

    if params["allowTests"].as_bool().unwrap_or(false) {
        args.push("--allow-tests".to_string());
    }

    let output = match run_command_candidates(&["probe"], &args, &ctx.cwd).await {
        Ok(output) => output,
        Err(CommandFailure::MissingBinary) => {
            return Ok(ToolOutput::error(
                "probe requires the 'probe' CLI. Install: cargo install probe-search",
            ));
        }
        Err(CommandFailure::Spawn(error)) => {
            return Ok(ToolOutput::error(format!("failed to run probe: {error}")));
        }
    };

    if !output.status.success() {
        return Ok(ToolOutput::error(command_error_message(
            "probe extract failed",
            &output,
        )));
    }

    let parsed = match parse_json_output(&output.stdout) {
        Some(value) => value,
        None => {
            return Ok(ToolOutput::error(format!(
                "probe extract returned invalid JSON: {}",
                summarize_command_output(&output)
            )));
        }
    };

    Ok(ToolOutput::text(truncate_output(format_probe_results(
        &parsed,
        &ctx.cwd,
        "No code blocks found.",
    ))))
}

// ── command helpers ─────────────────────────────────────────────────

struct CommandOutput {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

enum CommandFailure {
    MissingBinary,
    Spawn(std::io::Error),
}

async fn run_command_candidates(
    candidates: &[&str],
    args: &[String],
    cwd: &Path,
) -> std::result::Result<CommandOutput, CommandFailure> {
    let mut last_not_found = true;

    for candidate in candidates {
        let mut command = Command::new(candidate);
        command
            .args(args)
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        match command.output().await {
            Ok(output) => {
                return Ok(CommandOutput {
                    status: output.status,
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                last_not_found = true;
                continue;
            }
            Err(error) => return Err(CommandFailure::Spawn(error)),
        }
    }

    if last_not_found {
        Err(CommandFailure::MissingBinary)
    } else {
        Err(CommandFailure::Spawn(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no command candidates available",
        )))
    }
}

fn resolve_path(ctx: &ToolContext, maybe_path: Option<&str>) -> PathBuf {
    maybe_path
        .map(|path| ctx.cwd.join(path))
        .unwrap_or_else(|| ctx.cwd.clone())
}

fn resolve_extract_target(cwd: &Path, target: &str) -> String {
    match target.find('#') {
        Some(index) => {
            let (file, suffix) = target.split_at(index);
            if file.is_empty() {
                target.to_string()
            } else {
                format!("{}{}", cwd.join(file).display(), suffix)
            }
        }
        None => {
            if let Some((file, suffix)) = split_line_target(target) {
                format!("{}{}", cwd.join(file).display(), suffix)
            } else {
                cwd.join(target).display().to_string()
            }
        }
    }
}

fn split_line_target(target: &str) -> Option<(&str, &str)> {
    let bytes = target.as_bytes();
    for index in 0..target.len() {
        if bytes.get(index) != Some(&b':') {
            continue;
        }
        let suffix = &target[index + 1..];
        if suffix.is_empty() {
            continue;
        }
        if suffix
            .chars()
            .all(|ch| ch.is_ascii_digit() || ch == '-' || ch == ':')
        {
            return Some((&target[..index], &target[index..]));
        }
    }
    None
}

// ── output formatting ───────────────────────────────────────────────

fn parse_json_output(output: &str) -> Option<Value> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str(trimmed) {
        return Some(value);
    }
    let start = trimmed.find(['{', '['])?;
    let end = trimmed.rfind(['}', ']'])?;
    serde_json::from_str(&trimmed[start..=end]).ok()
}

fn format_probe_results(value: &Value, cwd: &Path, empty_message: &str) -> String {
    let results = match value.get("results").and_then(Value::as_array) {
        Some(results) if !results.is_empty() => results,
        _ => return empty_message.to_string(),
    };

    let mut sections = Vec::with_capacity(results.len());
    for result in results {
        let file = result
            .get("file")
            .and_then(Value::as_str)
            .map(|file| display_path(file, cwd))
            .unwrap_or_else(|| "<unknown file>".to_string());
        let lines = format_probe_line_span(result.get("lines"));
        let node_type = result.get("node_type").and_then(Value::as_str);
        let code = result
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim_end();

        let mut header = file;
        if !lines.is_empty() {
            header.push(':');
            header.push_str(&lines);
        }
        if let Some(node_type) = node_type {
            header.push_str(&format!(" ({node_type})"));
        }

        let fence = fence_language(result.get("file").and_then(Value::as_str));
        if code.is_empty() {
            sections.push(header);
        } else {
            sections.push(format!("{header}\n```{fence}\n{code}\n```"));
        }
    }

    sections.join("\n\n")
}

fn format_probe_line_span(lines: Option<&Value>) -> String {
    let Some(lines) = lines.and_then(Value::as_array) else {
        return String::new();
    };
    let start = lines.first().and_then(Value::as_u64).unwrap_or_default();
    let end = lines.get(1).and_then(Value::as_u64).unwrap_or(start);
    if start == 0 {
        String::new()
    } else if start == end {
        start.to_string()
    } else {
        format!("{start}-{end}")
    }
}

fn display_path(path: &str, cwd: &Path) -> String {
    let path_buf = PathBuf::from(path);
    path_buf
        .strip_prefix(cwd)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.to_string())
}

fn fence_language(path: Option<&str>) -> &'static str {
    match Path::new(path.unwrap_or_default())
        .extension()
        .and_then(|ext| ext.to_str())
    {
        Some("rs") => "rust",
        Some("ts") => "typescript",
        Some("tsx") => "tsx",
        Some("js") => "javascript",
        Some("jsx") => "jsx",
        Some("py") => "python",
        Some("go") => "go",
        Some("java") => "java",
        Some("rb") => "ruby",
        Some("php") => "php",
        Some("swift") => "swift",
        Some("c") => "c",
        Some("h") => "c",
        Some("cpp") | Some("cc") | Some("cxx") | Some("hpp") | Some("hxx") => "cpp",
        _ => "text",
    }
}

fn summarize_command_output(output: &CommandOutput) -> String {
    let stderr = output.stderr.trim();
    let stdout = output.stdout.trim();
    if !stderr.is_empty() && !stdout.is_empty() {
        format!("{stderr}\n{stdout}")
    } else if !stderr.is_empty() {
        stderr.to_string()
    } else {
        stdout.to_string()
    }
}

fn command_error_message(prefix: &str, output: &CommandOutput) -> String {
    let summary = summarize_command_output(output);
    if summary.is_empty() {
        format!(
            "{prefix} with exit code {}",
            output.status.code().unwrap_or(-1)
        )
    } else {
        format!("{prefix}: {summary}")
    }
}

fn truncate_output(text: String) -> String {
    if text.is_empty() {
        return text;
    }

    let truncated_lines = text
        .lines()
        .map(|line| truncate_line(line, MAX_LINE_CHARS))
        .collect::<Vec<_>>()
        .join("\n");

    let TruncationResult {
        content,
        truncated,
        output_lines,
        total_lines,
        temp_file,
        ..
    } = truncate_head(&truncated_lines, MAX_OUTPUT_LINES, MAX_OUTPUT_BYTES);

    if !truncated {
        return content;
    }

    let mut result = content;
    result.push_str(&format!(
        "\n[Output truncated: showing first {output_lines} of {total_lines} lines{}]",
        temp_file
            .as_ref()
            .map(|path| format!(". Full output saved to {}", path.display()))
            .unwrap_or_default()
    ));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{ToolContext, ToolUpdate};
    use crate::ui::NullInterface;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex, OnceLock};

    fn test_ctx(dir: &Path) -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel::<ToolUpdate>(64);
        ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(NullInterface),
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn with_test_path<F>(bins: &[(&str, &str)], include_system_path: bool, test: F)
    where
        F: FnOnce(&Path),
    {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let bin_dir = tmp.path().join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();

        for (name, script) in bins {
            let path = bin_dir.join(name);
            std::fs::write(&path, script).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut permissions = std::fs::metadata(&path).unwrap().permissions();
                permissions.set_mode(0o755);
                std::fs::set_permissions(&path, permissions).unwrap();
            }
        }

        let old_path = std::env::var_os("PATH");
        let path_value = if include_system_path {
            match old_path.as_ref() {
                Some(old_path) => {
                    let mut paths = vec![bin_dir.clone()];
                    paths.extend(std::env::split_paths(old_path));
                    std::env::join_paths(paths).unwrap()
                }
                None => bin_dir.as_os_str().to_os_string(),
            }
        } else {
            bin_dir.as_os_str().to_os_string()
        };

        std::env::set_var("PATH", path_value);
        test(tmp.path());
        match old_path {
            Some(old_path) => std::env::set_var("PATH", old_path),
            None => std::env::remove_var("PATH"),
        }
    }

    fn run_async<F>(future: F) -> F::Output
    where
        F: std::future::Future,
    {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn probe_search_shells_out() {
        let script = r#"#!/bin/sh
cat <<JSON
{"results":[{"file":"$PWD/sample.rs","lines":[1,1],"code":"fn helper() {}","node_type":"function_item"}],"summary":{"count":1}}
JSON
"#;
        with_test_path(&[("probe", script)], true, |tmp| {
            std::fs::write(tmp.join("sample.rs"), "fn helper() {}\n").unwrap();
            let ctx = test_ctx(tmp);
            let result = run_async(execute_search(
                "1",
                json!({ "query": "helper", "path": ".", "maxResults": 7 }),
                ctx,
            ))
            .unwrap();

            assert!(!result.is_error);
            let text = match &result.content[0] {
                imp_llm::ContentBlock::Text { text } => text.clone(),
                _ => panic!("expected text"),
            };
            assert!(text.contains("sample.rs:1 (function_item)"));
            assert!(text.contains("fn helper() {}"));
        });
    }

    #[test]
    fn probe_extract_passes_targets() {
        let script = r#"#!/bin/sh
cat <<JSON
{"results":[{"file":"$PWD/sample.rs","lines":[2,4],"code":"pub fn main() {\n    println!(\"hello\");\n}","node_type":"function_item"}],"summary":{"count":1}}
JSON
"#;
        with_test_path(&[("probe", script)], true, |tmp| {
            std::fs::write(
                tmp.join("sample.rs"),
                "fn helper() {}\n\npub fn main() {\n    println!(\"hello\");\n}\n",
            )
            .unwrap();
            let ctx = test_ctx(tmp);
            let result = run_async(execute_extract(
                "1",
                json!({ "targets": ["sample.rs:3"], "context": 2 }),
                ctx,
            ))
            .unwrap();

            assert!(!result.is_error);
            let text = match &result.content[0] {
                imp_llm::ContentBlock::Text { text } => text.clone(),
                _ => panic!("expected text"),
            };
            assert!(text.contains("sample.rs:2-4"));
        });
    }

    #[test]
    fn probe_missing_binary_error() {
        with_test_path(&[], false, |tmp| {
            let ctx = test_ctx(tmp);
            let result = run_async(execute_search("1", json!({ "query": "helper" }), ctx)).unwrap();
            assert!(result.is_error);
            let text = match &result.content[0] {
                imp_llm::ContentBlock::Text { text } => text.clone(),
                _ => panic!("expected text"),
            };
            assert!(text.contains("Install"));
        });
    }
}
