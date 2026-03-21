use std::path::{Path, PathBuf};
use std::process::Stdio;

use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use tokio::process::Command;

use super::{truncate_head, truncate_line, Tool, ToolContext, ToolOutput, TruncationResult};
use crate::error::{Error, Result};

const DEFAULT_MAX_RESULTS: usize = 10;
const MAX_OUTPUT_LINES: usize = 2000;
const MAX_OUTPUT_BYTES: usize = 50 * 1024;
const MAX_LINE_CHARS: usize = 500;

pub struct ProbeSearchTool;

#[async_trait]
impl Tool for ProbeSearchTool {
    fn name(&self) -> &str {
        "probe_search"
    }

    fn label(&self) -> &str {
        "Semantic Code Search"
    }

    fn description(&self) -> &str {
        "Semantic code search using ripgrep + tree-sitter AST parsing."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" },
                "path": { "type": "string", "description": "Directory or file to search" },
                "language": { "type": "string", "description": "Limit to language" },
                "maxResults": { "type": "number", "description": "Maximum number of results" }
            },
            "required": ["query"]
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
                    "probe_search requires the 'probe' CLI. Install probe: cargo install probe-search",
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
}

pub struct ProbeExtractTool;

#[async_trait]
impl Tool for ProbeExtractTool {
    fn name(&self) -> &str {
        "probe_extract"
    }

    fn label(&self) -> &str {
        "Extract Code Block"
    }

    fn description(&self) -> &str {
        "Extract complete code blocks from files using tree-sitter AST parsing."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "targets": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "File:line or file#symbol targets"
                },
                "context": { "type": "number", "description": "Context lines" }
            },
            "required": ["targets"]
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
                    "probe_extract requires the 'probe' CLI. Install probe: cargo install probe-search",
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
}

pub struct ScanTool;

#[async_trait]
impl Tool for ScanTool {
    fn name(&self) -> &str {
        "scan"
    }

    fn label(&self) -> &str {
        "Scan Code Structure"
    }

    fn description(&self) -> &str {
        "Extract code structure (types, functions, imports) from source files."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["extract", "build", "scan"] },
                "files": { "type": "array", "items": { "type": "string" } },
                "directory": { "type": "string" },
                "task": { "type": "string" }
            },
            "required": ["action"]
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
        let action = match params["action"].as_str() {
            Some(action) => action,
            None => return Ok(ToolOutput::error("missing 'action' parameter")),
        };

        let mut files = match action {
            "extract" | "build" => {
                let files = match params["files"].as_array() {
                    Some(files) if !files.is_empty() => files,
                    _ => {
                        return Ok(ToolOutput::error(
                            "scan requires a non-empty 'files' array for 'extract' and 'build' actions",
                        ));
                    }
                };

                let mut resolved = Vec::with_capacity(files.len());
                for file in files {
                    match file.as_str() {
                        Some(file) => resolved.push(ctx.cwd.join(file)),
                        None => return Ok(ToolOutput::error("'files' must contain only strings")),
                    }
                }
                resolved
            }
            "scan" => collect_scan_files(&resolve_path(&ctx, params["directory"].as_str()))?,
            _ => {
                return Ok(ToolOutput::error(format!(
                    "unsupported scan action: {action}"
                )))
            }
        };

        files.sort();
        files.dedup();

        if files.is_empty() {
            return Ok(ToolOutput::text("No supported source files found."));
        }

        let task = params["task"].as_str();
        let report = format_scan_report(action, task, &files, &ctx.cwd)?;
        Ok(ToolOutput::text(truncate_output(report)))
    }
}

pub struct AstGrepTool;

#[async_trait]
impl Tool for AstGrepTool {
    fn name(&self) -> &str {
        "ast_grep"
    }

    fn label(&self) -> &str {
        "AST Pattern Search"
    }

    fn description(&self) -> &str {
        "Structural code search using AST patterns."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "AST pattern to search for" },
                "path": { "type": "string", "description": "File or directory to search" },
                "lang": { "type": "string", "description": "Language" },
                "replace": { "type": "string", "description": "Replacement pattern" }
            },
            "required": ["pattern"]
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
        let pattern = match params["pattern"].as_str() {
            Some(pattern) => pattern,
            None => return Ok(ToolOutput::error("missing 'pattern' parameter")),
        };

        let search_path = resolve_path(&ctx, params["path"].as_str());
        let replace = params["replace"].as_str();

        let mut search_args = vec![
            "run".to_string(),
            "--pattern".to_string(),
            pattern.to_string(),
            search_path.display().to_string(),
            "--json=pretty".to_string(),
            "--color".to_string(),
            "never".to_string(),
        ];

        if let Some(lang) = params["lang"]
            .as_str()
            .or_else(|| infer_sg_lang(&search_path))
        {
            search_args.push("--lang".to_string());
            search_args.push(lang.to_string());
        }

        let search_output = match run_command_candidates(
            &["sg", "ast-grep"],
            &search_args,
            &ctx.cwd,
        )
        .await
        {
            Ok(output) => output,
            Err(CommandFailure::MissingBinary) => {
                return Ok(ToolOutput::error(
                    "ast_grep requires the 'sg' (ast-grep) CLI. Install ast-grep: cargo install ast-grep",
                ));
            }
            Err(CommandFailure::Spawn(error)) => {
                return Ok(ToolOutput::error(format!(
                    "failed to run ast-grep: {error}"
                )));
            }
        };

        if !search_output.status.success() {
            return Ok(ToolOutput::error(command_error_message(
                "ast-grep search failed",
                &search_output,
            )));
        }

        let parsed = match parse_json_output(&search_output.stdout) {
            Some(value) => value,
            None => {
                return Ok(ToolOutput::error(format!(
                    "ast-grep returned invalid JSON: {}",
                    summarize_command_output(&search_output)
                )));
            }
        };

        let formatted_matches = format_ast_grep_results(&parsed, &ctx.cwd);
        if replace.is_none() {
            return Ok(ToolOutput::text(truncate_output(formatted_matches)));
        }

        if is_empty_result_set(&parsed) {
            return Ok(ToolOutput::text("No matches found."));
        }

        let mut apply_args = vec![
            "run".to_string(),
            "--pattern".to_string(),
            pattern.to_string(),
            "--rewrite".to_string(),
            replace.unwrap().to_string(),
            search_path.display().to_string(),
            "-U".to_string(),
            "--color".to_string(),
            "never".to_string(),
        ];

        if let Some(lang) = params["lang"]
            .as_str()
            .or_else(|| infer_sg_lang(&search_path))
        {
            apply_args.push("--lang".to_string());
            apply_args.push(lang.to_string());
        }

        let apply_output = match run_command_candidates(&["sg", "ast-grep"], &apply_args, &ctx.cwd)
            .await
        {
            Ok(output) => output,
            Err(CommandFailure::MissingBinary) => {
                return Ok(ToolOutput::error(
                    "ast_grep requires the 'sg' (ast-grep) CLI. Install ast-grep: cargo install ast-grep",
                ));
            }
            Err(CommandFailure::Spawn(error)) => {
                return Ok(ToolOutput::error(format!(
                    "failed to run ast-grep: {error}"
                )));
            }
        };

        if !apply_output.status.success() {
            return Ok(ToolOutput::error(command_error_message(
                "ast-grep rewrite failed",
                &apply_output,
            )));
        }

        let mut response = formatted_matches;
        let apply_summary = summarize_command_output(&apply_output);
        if !apply_summary.is_empty() {
            response.push_str("\n\n");
            response.push_str(&apply_summary);
        }

        Ok(ToolOutput::text(truncate_output(response)))
    }
}

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

fn format_ast_grep_results(value: &Value, cwd: &Path) -> String {
    let Some(results) = value.as_array() else {
        return "No matches found.".to_string();
    };

    if results.is_empty() {
        return "No matches found.".to_string();
    }

    let mut sections = Vec::with_capacity(results.len());
    for result in results {
        let file = result
            .get("file")
            .and_then(Value::as_str)
            .map(|file| display_path(file, cwd))
            .unwrap_or_else(|| "<unknown file>".to_string());
        let (start_line, end_line) = ast_grep_line_span(result);
        let code = result
            .get("lines")
            .and_then(Value::as_str)
            .or_else(|| result.get("text").and_then(Value::as_str))
            .unwrap_or_default()
            .trim_end();
        let replacement = result.get("replacement").and_then(Value::as_str);

        let mut section = format!("{file}:{start_line}-{end_line}");
        if !code.is_empty() {
            let fence = fence_language(result.get("file").and_then(Value::as_str));
            section.push_str(&format!("\n```{fence}\n{code}\n```"));
        }
        if let Some(replacement) = replacement {
            section.push_str("\n=>\n");
            section.push_str(replacement);
        }
        sections.push(section);
    }

    sections.join("\n\n")
}

fn ast_grep_line_span(result: &Value) -> (u64, u64) {
    let start = result
        .get("range")
        .and_then(|range| range.get("start"))
        .and_then(|start| start.get("line"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
        + 1;
    let end = result
        .get("range")
        .and_then(|range| range.get("end"))
        .and_then(|end| end.get("line"))
        .and_then(Value::as_u64)
        .unwrap_or(start.saturating_sub(1))
        + 1;
    (start, end.max(start))
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

fn is_empty_result_set(value: &Value) -> bool {
    value
        .as_array()
        .map(|results| results.is_empty())
        .unwrap_or(false)
}

fn infer_sg_lang(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => Some("rust"),
        Some("ts") => Some("typescript"),
        Some("tsx") => Some("tsx"),
        Some("js") => Some("javascript"),
        Some("jsx") => Some("jsx"),
        Some("py") => Some("python"),
        Some("go") => Some("go"),
        Some("java") => Some("java"),
        Some("rb") => Some("ruby"),
        Some("php") => Some("php"),
        Some("swift") => Some("swift"),
        Some("c") => Some("c"),
        Some("h") => Some("c"),
        Some("cpp") | Some("cc") | Some("cxx") | Some("hpp") | Some("hxx") => Some("cpp"),
        _ => None,
    }
}

fn collect_scan_files(root: &Path) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(if is_supported_scan_file(root) {
            vec![root.to_path_buf()]
        } else {
            Vec::new()
        });
    }

    if !root.exists() {
        return Err(Error::Tool(format!(
            "scan path not found: {}",
            root.display()
        )));
    }

    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        if is_supported_scan_file(entry.path()) {
            files.push(entry.path().to_path_buf());
        }
    }

    Ok(files)
}

fn is_supported_scan_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(
            "rs" | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "py"
                | "go"
                | "java"
                | "rb"
                | "php"
                | "swift"
                | "c"
                | "h"
                | "cpp"
                | "cc"
                | "cxx"
                | "hpp"
                | "hxx"
        )
    )
}

fn format_scan_report(
    action: &str,
    task: Option<&str>,
    files: &[PathBuf],
    cwd: &Path,
) -> Result<String> {
    let mut sections = Vec::new();
    sections.push(format!("Action: {action}"));
    if let Some(task) = task {
        sections.push(format!("Task: {task}"));
    }
    sections.push(format!("Files analyzed: {}", files.len()));

    for file in files {
        let structure = analyze_source_file(file)?;
        if structure.imports.is_empty()
            && structure.types.is_empty()
            && structure.functions.is_empty()
        {
            continue;
        }

        let mut section = vec![display_path(&file.display().to_string(), cwd)];
        if !structure.imports.is_empty() {
            section.push(format!("  Imports ({}):", structure.imports.len()));
            section.extend(structure.imports.iter().map(|item| format!("    - {item}")));
        }
        if !structure.types.is_empty() {
            section.push(format!("  Types ({}):", structure.types.len()));
            section.extend(structure.types.iter().map(|item| format!("    - {item}")));
        }
        if !structure.functions.is_empty() {
            section.push(format!("  Functions ({}):", structure.functions.len()));
            section.extend(
                structure
                    .functions
                    .iter()
                    .map(|item| format!("    - {item}")),
            );
        }
        sections.push(section.join("\n"));
    }

    Ok(sections.join("\n\n"))
}

struct FileStructure {
    imports: Vec<String>,
    types: Vec<String>,
    functions: Vec<String>,
}

fn analyze_source_file(path: &Path) -> Result<FileStructure> {
    let bytes = std::fs::read(path)?;
    if bytes.contains(&0) {
        return Ok(FileStructure {
            imports: Vec::new(),
            types: Vec::new(),
            functions: Vec::new(),
        });
    }

    let content = String::from_utf8_lossy(&bytes);
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();

    let imports = match extension {
        "rs" => collect_trimmed_lines(&content, &[r"^\s*use\s+.+;$"]),
        "ts" | "tsx" | "js" | "jsx" => collect_trimmed_lines(&content, &[r"^\s*import\s+.+;?$"]),
        "py" => collect_trimmed_lines(
            &content,
            &[r"^\s*import\s+.+$", r"^\s*from\s+\S+\s+import\s+.+$"],
        ),
        "go" => collect_trimmed_lines(&content, &[r"^\s*import\s+(?:\(|.+)$"]),
        _ => Vec::new(),
    };

    let types = match extension {
        "rs" => collect_named_matches(
            &content,
            &[
                r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|trait|type)\s+([A-Za-z_][A-Za-z0-9_]*)",
            ],
            |kind, name| format!("{kind} {name}"),
        ),
        "ts" | "tsx" => collect_named_matches(
            &content,
            &[
                r"^\s*export\s+(?:interface|type|class|enum)\s+([A-Za-z_$][A-Za-z0-9_$]*)",
                r"^\s*(?:interface|type|class|enum)\s+([A-Za-z_$][A-Za-z0-9_$]*)",
            ],
            |kind, name| format!("{kind} {name}"),
        ),
        "js" | "jsx" => collect_named_matches(
            &content,
            &[
                r"^\s*class\s+([A-Za-z_$][A-Za-z0-9_$]*)",
                r"^\s*export\s+class\s+([A-Za-z_$][A-Za-z0-9_$]*)",
            ],
            |kind, name| format!("{kind} {name}"),
        ),
        "py" => collect_named_matches(
            &content,
            &[r"^\s*class\s+([A-Za-z_][A-Za-z0-9_]*)"],
            |kind, name| format!("{kind} {name}"),
        ),
        "go" => collect_named_matches(
            &content,
            &[r"^\s*type\s+([A-Za-z_][A-Za-z0-9_]*)\s+"],
            |kind, name| format!("{kind} {name}"),
        ),
        _ => Vec::new(),
    };

    let functions = match extension {
        "rs" => collect_named_matches(
            &content,
            &[r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\("],
            |kind, name| format!("{kind} {name}"),
        ),
        "ts" | "tsx" | "js" | "jsx" => collect_named_matches(
            &content,
            &[
                r"^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*\(",
                r"^\s*(?:export\s+)?const\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*(?:async\s*)?\([^)]*\)\s*=>",
            ],
            |kind, name| format!("{kind} {name}"),
        ),
        "py" => collect_named_matches(
            &content,
            &[r"^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\("],
            |kind, name| format!("{kind} {name}"),
        ),
        "go" => collect_named_matches(
            &content,
            &[r"^\s*func\s+(?:\([^)]+\)\s*)?([A-Za-z_][A-Za-z0-9_]*)\s*\("],
            |kind, name| format!("{kind} {name}"),
        ),
        _ => Vec::new(),
    };

    Ok(FileStructure {
        imports,
        types,
        functions,
    })
}

fn collect_trimmed_lines(content: &str, patterns: &[&str]) -> Vec<String> {
    let regexes = patterns
        .iter()
        .map(|pattern| Regex::new(pattern).expect("valid regex"))
        .collect::<Vec<_>>();

    content
        .lines()
        .filter(|line| regexes.iter().any(|regex| regex.is_match(line)))
        .map(|line| line.trim().to_string())
        .collect()
}

fn collect_named_matches<F>(content: &str, patterns: &[&str], render: F) -> Vec<String>
where
    F: Fn(&str, &str) -> String,
{
    let regexes = patterns
        .iter()
        .map(|pattern| Regex::new(pattern).expect("valid regex"))
        .collect::<Vec<_>>();
    let kind_regex =
        Regex::new(r"\b(struct|enum|trait|type|interface|class|def|func|function|fn)\b")
            .expect("valid regex");

    let mut matches = Vec::new();
    for line in content.lines() {
        for regex in &regexes {
            if let Some(captures) = regex.captures(line) {
                let name = captures
                    .get(1)
                    .map(|capture| capture.as_str())
                    .unwrap_or_default();
                let kind = kind_regex
                    .captures(line)
                    .and_then(|captures| captures.get(1))
                    .map(|capture| capture.as_str())
                    .unwrap_or("item");
                matches.push(render(kind, name));
                break;
            }
        }
    }
    matches
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
    fn probe_search_shells_out_and_formats_results() {
        let script = r#"#!/bin/sh
printf '%s\n' "$@" > "$PWD/probe-args.txt"
cat <<JSON
{"results":[{"file":"$PWD/sample.rs","lines":[1,1],"code":"fn helper() {}","node_type":"function_item"}],"summary":{"count":1}}
JSON
"#;

        with_test_path(&[("probe", script)], true, |tmp| {
            std::fs::write(tmp.join("sample.rs"), "fn helper() {}\n").unwrap();
            let ctx = test_ctx(tmp);
            let result = run_async(ProbeSearchTool.execute(
                "1",
                json!({
                    "query": "helper",
                    "path": ".",
                    "language": "rust",
                    "maxResults": 7
                }),
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

            let args = std::fs::read_to_string(tmp.join("probe-args.txt")).unwrap();
            assert!(args.contains("search"));
            assert!(args.contains("helper"));
            assert!(args.contains("--language"));
            assert!(args.contains("rust"));
            assert!(args.contains("--max-results"));
            assert!(args.contains("7"));
        });
    }

    #[test]
    fn probe_extract_passes_context_and_targets() {
        let script = r#"#!/bin/sh
printf '%s\n' "$@" > "$PWD/probe-extract-args.txt"
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
            let result = run_async(ProbeExtractTool.execute(
                "1",
                json!({
                    "targets": ["sample.rs:3"],
                    "context": 2
                }),
                ctx,
            ))
            .unwrap();

            assert!(!result.is_error);
            let text = match &result.content[0] {
                imp_llm::ContentBlock::Text { text } => text.clone(),
                _ => panic!("expected text"),
            };
            assert!(text.contains("sample.rs:2-4"));
            assert!(text.contains("println!(\"hello\")"));

            let args = std::fs::read_to_string(tmp.join("probe-extract-args.txt")).unwrap();
            assert!(args.contains("extract"));
            assert!(args.contains("--context"));
            assert!(args.contains("2"));
            assert!(args.contains(&tmp.join("sample.rs").display().to_string()));
        });
    }

    #[test]
    fn probe_search_reports_missing_probe_binary() {
        with_test_path(&[], false, |tmp| {
            let ctx = test_ctx(tmp);
            let result =
                run_async(ProbeSearchTool.execute("1", json!({ "query": "helper" }), ctx)).unwrap();

            assert!(result.is_error);
            let text = match &result.content[0] {
                imp_llm::ContentBlock::Text { text } => text.clone(),
                _ => panic!("expected text"),
            };
            assert!(text.contains("Install probe"));
        });
    }

    #[test]
    fn scan_extract_reports_basic_structure_without_probe() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("sample.rs"),
            "use std::fmt;\n\npub struct Greeter;\n\npub fn greet() {}\n",
        )
        .unwrap();

        let ctx = test_ctx(tmp.path());
        let result = run_async(ScanTool.execute(
            "1",
            json!({
                "action": "extract",
                "files": ["sample.rs"],
                "task": "summarize structure"
            }),
            ctx,
        ))
        .unwrap();

        assert!(!result.is_error);
        let text = match &result.content[0] {
            imp_llm::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(text.contains("Action: extract"));
        assert!(text.contains("Task: summarize structure"));
        assert!(text.contains("sample.rs"));
        assert!(text.contains("use std::fmt;"));
        assert!(text.contains("struct Greeter"));
        assert!(text.contains("fn greet"));
    }

    #[test]
    fn ast_grep_search_and_replace_shells_out_to_sg() {
        let script = r#"#!/bin/sh
printf '%s\n' "$@" > "$PWD/sg-args.txt"
if printf '%s\n' "$@" | grep -q -- '--rewrite'; then
  printf 'fn renamed() {}\n' > "$PWD/sample.rs"
  echo 'Applied 1 changes'
  exit 0
fi
cat <<JSON
[{"file":"$PWD/sample.rs","range":{"start":{"line":0,"column":0},"end":{"line":0,"column":14}},"lines":"fn helper() {}","text":"fn helper() {}"}]
JSON
"#;

        with_test_path(&[("sg", script)], true, |tmp| {
            std::fs::write(tmp.join("sample.rs"), "fn helper() {}\n").unwrap();
            let ctx = test_ctx(tmp);
            let result = run_async(AstGrepTool.execute(
                "1",
                json!({
                    "pattern": "fn $NAME() {}",
                    "replace": "fn renamed() {}",
                    "path": "sample.rs",
                    "lang": "rust"
                }),
                ctx,
            ))
            .unwrap();

            assert!(!result.is_error);
            let text = match &result.content[0] {
                imp_llm::ContentBlock::Text { text } => text.clone(),
                _ => panic!("expected text"),
            };
            assert!(text.contains("sample.rs:1-1"));
            assert!(text.contains("Applied 1 changes"));
            assert_eq!(
                std::fs::read_to_string(tmp.join("sample.rs")).unwrap(),
                "fn renamed() {}\n"
            );

            let args = std::fs::read_to_string(tmp.join("sg-args.txt")).unwrap();
            assert!(args.contains("run"));
            assert!(args.contains("--rewrite"));
            assert!(args.contains("fn renamed() {}"));
        });
    }

    #[test]
    fn ast_grep_reports_missing_binary() {
        with_test_path(&[], false, |tmp| {
            let ctx = test_ctx(tmp);
            let result =
                run_async(AstGrepTool.execute("1", json!({ "pattern": "fn $NAME() {}" }), ctx))
                    .unwrap();

            assert!(result.is_error);
            let text = match &result.content[0] {
                imp_llm::ContentBlock::Text { text } => text.clone(),
                _ => panic!("expected text"),
            };
            assert!(text.contains("Install ast-grep"));
        });
    }
}
