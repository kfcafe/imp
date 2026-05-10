//! Scan tool — extract code structure using tree-sitter AST parsing.
//!
//! Dispatches to language-specific parsers based on file extension.
//! Produces rich output: visibility, signatures, fields, variants, trait impls.

pub mod go;
pub mod kotlin;
pub mod python;
pub mod rust;
pub mod types;
pub mod typescript;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::json;

use super::{truncate_head, truncate_line, Tool, ToolContext, ToolOutput, TruncationResult};
use crate::error::{Error, Result};
use types::*;

const MAX_OUTPUT_LINES: usize = 2000;
const MAX_OUTPUT_BYTES: usize = 50 * 1024;
const MAX_LINE_CHARS: usize = 500;

/// Node kinds that represent enclosing blocks we want to extract around a line or symbol.
const BLOCK_KINDS: &[&str] = &[
    // Rust
    "function_item",
    "impl_item",
    "struct_item",
    "enum_item",
    "trait_item",
    "mod_item",
    "const_item",
    "static_item",
    "type_item",
    "macro_definition",
    // TypeScript / JavaScript
    "function_declaration",
    "method_definition",
    "class_declaration",
    "interface_declaration",
    "type_alias_declaration",
    "enum_declaration",
    "export_statement",
    "lexical_declaration",
    "variable_declaration",
    "arrow_function",
    // Python
    "function_definition",
    "class_definition",
    "decorated_definition",
    // Kotlin
    "class_declaration",
    "object_declaration",
    "function_declaration",
    "property_declaration",
    // Go
    "function_declaration",
    "method_declaration",
    "type_declaration",
    "type_spec",
];

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
        "Code structure search/extraction with tree-sitter."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["directory", "files", "extract", "search", "tests", "related", "references", "impact"],
                    "description": "Scan operation"
                },
                "directory": {
                    "type": "string",
                    "description": "Directory; defaults to cwd"
                },
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Files for action=files"
                },
                "targets": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Targets: file#symbol, file:start-end, or file:line"
                },
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "mode": {
                    "type": "string",
                    "enum": ["symbol", "text", "concept"],
                    "description": "Search mode; default concept"
                },
                "preset": {
                    "type": "string",
                    "enum": ["definition", "edit_context", "module_context", "test_context"],
                    "description": "Extraction preset"
                },
                "target": {
                    "type": "string",
                    "description": "Single target"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Max results"
                }
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
            Some(a) => a,
            None => return Ok(ToolOutput::error("missing 'action' parameter")),
        };

        let mut files = match action {
            "extract" => {
                let target_values = params
                    .get("targets")
                    .or_else(|| params.get("files"))
                    .and_then(|value| value.as_array());
                let targets = match parse_string_array(target_values, "targets") {
                    Ok(targets) if !targets.is_empty() => targets,
                    Ok(_) => return Ok(ToolOutput::error("scan extract requires targets")),
                    Err(message) => return Ok(ToolOutput::error(message)),
                };
                let preset = params["preset"].as_str();
                return Ok(execute_extract_with_preset(&targets, preset, &ctx));
            }
            "search" => {
                let query = match params["query"]
                    .as_str()
                    .map(str::trim)
                    .filter(|q| !q.is_empty())
                {
                    Some(query) => query,
                    None => return Ok(ToolOutput::error("scan search requires query")),
                };
                let mode = params["mode"].as_str().unwrap_or("concept");
                let max_results = params["max_results"].as_u64().unwrap_or(10) as usize;
                let files = files_from_params_or_directory(&params, &ctx)?;
                return Ok(execute_search(
                    files,
                    &ctx.cwd,
                    query,
                    mode,
                    max_results.max(1),
                ));
            }
            "tests" => {
                let targets =
                    parse_string_array(params["targets"].as_array(), "targets").unwrap_or_default();
                let target = params["target"]
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| targets.first().cloned())
                    .or_else(|| params["query"].as_str().map(str::to_string));
                let Some(target) = target else {
                    return Ok(ToolOutput::error(
                        "scan tests requires target, targets, or query",
                    ));
                };
                let max_results = params["max_results"].as_u64().unwrap_or(10) as usize;
                let files = files_from_params_or_directory(&params, &ctx)?;
                return Ok(execute_tests(files, &ctx.cwd, &target, max_results.max(1)));
            }
            "related" => {
                let targets =
                    parse_string_array(params["targets"].as_array(), "targets").unwrap_or_default();
                let target = params["target"]
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| targets.first().cloned());
                let Some(target) = target else {
                    return Ok(ToolOutput::error("scan related requires target or targets"));
                };
                let files = files_from_params_or_directory(&params, &ctx)?;
                return Ok(execute_related(files, &ctx.cwd, &target));
            }
            "references" | "impact" => {
                let target = params["target"]
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| params["query"].as_str().map(str::to_string));
                let Some(target) = target else {
                    return Ok(ToolOutput::error(format!(
                        "scan {action} requires target or query"
                    )));
                };
                let max_results = params["max_results"].as_u64().unwrap_or(25) as usize;
                let files = files_from_params_or_directory(&params, &ctx)?;
                if action == "references" {
                    return Ok(execute_references(
                        files,
                        &ctx.cwd,
                        &target,
                        max_results.max(1),
                    ));
                }
                return Ok(execute_impact(files, &ctx.cwd, &target, max_results.max(1)));
            }
            "files" | "build" => {
                let files = match parse_string_array(params["files"].as_array(), "files") {
                    Ok(files) if !files.is_empty() => files,
                    Ok(_) => return Ok(ToolOutput::error("scan files requires files")),
                    Err(message) => return Ok(ToolOutput::error(message)),
                };
                files
                    .into_iter()
                    .map(|file| crate::tools::resolve_path(&ctx.cwd, &file))
                    .collect()
            }
            "directory" | "scan" => {
                let dir = params["directory"]
                    .as_str()
                    .map(|d| crate::tools::resolve_path(&ctx.cwd, d))
                    .unwrap_or_else(|| ctx.cwd.clone());
                collect_source_files(&dir)?
            }
            _ => return Ok(ToolOutput::error(format!("unknown action: {action}"))),
        };

        files.sort();
        files.dedup();

        if files.is_empty() {
            return Ok(ToolOutput::text("No supported source files found."));
        }

        let result = extract_files(&files, &ctx.cwd);
        let action_name = canonical_action(action);
        let output = format_result(&result, &files, &ctx.cwd, action_name, None);

        Ok(ToolOutput {
            content: vec![imp_llm::ContentBlock::Text {
                text: truncate_output(output),
            }],
            details: json!({
                "action": action_name,
                "files_analyzed": files.len(),
                "supported_languages": ["rust", "typescript", "javascript", "python", "go", "kotlin"],
                "types_count": result.types.len(),
                "functions_count": result.functions.len(),
            }),
            is_error: false,
        })
    }
}

// ── parameter helpers ───────────────────────────────────────────────

fn canonical_action(action: &str) -> &str {
    match action {
        "scan" => "directory",
        "build" => "files",
        other => other,
    }
}

fn parse_string_array(
    values: Option<&Vec<serde_json::Value>>,
    field: &str,
) -> std::result::Result<Vec<String>, String> {
    let Some(values) = values else {
        return Ok(Vec::new());
    };
    let mut strings = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let Some(text) = value
            .as_str()
            .map(str::trim)
            .filter(|text| !text.is_empty())
        else {
            return Err(format!("{field}[{index}] must be a non-empty string"));
        };
        strings.push(text.to_string());
    }
    Ok(strings)
}

fn files_from_params_or_directory(
    params: &serde_json::Value,
    ctx: &ToolContext,
) -> Result<Vec<PathBuf>> {
    let explicit_files =
        parse_string_array(params["files"].as_array(), "files").map_err(Error::Tool)?;
    if !explicit_files.is_empty() {
        return Ok(explicit_files
            .into_iter()
            .map(|file| crate::tools::resolve_path(&ctx.cwd, &file))
            .collect());
    }

    let dir = params["directory"]
        .as_str()
        .map(|d| crate::tools::resolve_path(&ctx.cwd, d))
        .unwrap_or_else(|| ctx.cwd.clone());
    collect_source_files(&dir)
}

// ── extraction dispatch ─────────────────────────────────────────────

fn extract_files(files: &[PathBuf], cwd: &Path) -> ScanResult {
    let mut result = ScanResult::default();

    for file in files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Skip binary files
        if source.as_bytes().contains(&0) {
            continue;
        }

        let rel = file
            .strip_prefix(cwd)
            .unwrap_or(file)
            .to_string_lossy()
            .to_string();

        let ext = file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();

        match ext {
            "rs" => rust::parse(&source, &rel, &mut result),
            "ts" => {
                if !rel.ends_with(".d.ts") {
                    typescript::parse(&source, &rel, false, &mut result);
                }
            }
            "tsx" => typescript::parse(&source, &rel, true, &mut result),
            "py" => python::parse(&source, &rel, &mut result),
            "go" => go::parse(&source, &rel, &mut result),
            "kt" | "kts" => kotlin::parse(&source, &rel, &mut result),
            "js" | "jsx" => typescript::parse(&source, &rel, ext == "jsx", &mut result),
            // TODO: add more languages as tree-sitter grammars are added
            _ => {}
        }
    }

    result
}

// ── file collection ─────────────────────────────────────────────────

fn collect_source_files(root: &Path) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(if is_supported(root) {
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
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| !is_skip_dir(e.path()))
    {
        if is_supported(entry.path()) {
            files.push(entry.path().to_path_buf());
        }
    }

    Ok(files)
}

fn is_supported(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "kt" | "kts")
    )
}

fn is_skip_dir(path: &Path) -> bool {
    const SKIP: &[&str] = &[
        "target",
        "node_modules",
        ".git",
        "__pycache__",
        ".venv",
        "venv",
        "vendor",
        "dist",
        "build",
        ".next",
        "coverage",
    ];
    path.components().any(|c| {
        if let std::path::Component::Normal(name) = c {
            SKIP.contains(&name.to_string_lossy().as_ref())
        } else {
            false
        }
    })
}

// ── search and test discovery ───────────────────────────────────────

#[derive(Debug, Clone)]
struct IndexedSymbol {
    file: String,
    name: String,
    kind: String,
    line: usize,
    text: String,
    is_test: bool,
}

#[derive(Debug, Clone)]
struct SearchHit {
    file: String,
    symbol: Option<String>,
    kind: String,
    line: usize,
    score: i32,
    why: Vec<String>,
}

fn execute_search(
    mut files: Vec<PathBuf>,
    cwd: &Path,
    query: &str,
    mode: &str,
    max_results: usize,
) -> ToolOutput {
    files.sort();
    files.dedup();
    let index = build_symbol_index(&files, cwd);
    let hits = search_index(&index, query, mode, max_results);
    let mut lines = vec![
        format!("Action: search"),
        format!("Query: {query}"),
        format!("Mode: {mode}"),
        format!("Files analyzed: {}", files.len()),
    ];
    if hits.is_empty() {
        lines.push("No matching symbols found.".to_string());
    } else {
        lines.push("Results:".to_string());
        for hit in &hits {
            let symbol = hit.symbol.as_deref().unwrap_or("<file>");
            lines.push(format!(
                "- {}:{} [{}] {} score={} — {}",
                hit.file,
                hit.line,
                hit.kind,
                symbol,
                hit.score,
                hit.why.join(", ")
            ));
        }
    }

    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text {
            text: truncate_output(lines.join("\n")),
        }],
        details: json!({
            "action": "search",
            "query": query,
            "mode": mode,
            "files_analyzed": files.len(),
            "results": hits.iter().map(|hit| json!({
                "file": hit.file,
                "symbol": hit.symbol,
                "kind": hit.kind,
                "line": hit.line,
                "score": hit.score,
                "why": hit.why,
            })).collect::<Vec<_>>(),
        }),
        is_error: false,
    }
}

fn execute_tests(
    mut files: Vec<PathBuf>,
    cwd: &Path,
    target: &str,
    max_results: usize,
) -> ToolOutput {
    files.sort();
    files.dedup();
    let index = build_symbol_index(&files, cwd);
    let tests = discover_tests(&index, target, cwd, max_results);
    let mut lines = vec![
        format!("Action: tests"),
        format!("Target: {target}"),
        format!("Files analyzed: {}", files.len()),
    ];
    if tests.is_empty() {
        lines.push("No likely tests found.".to_string());
    } else {
        lines.push("Likely tests:".to_string());
        for test in &tests {
            lines.push(format!(
                "- {}:{} {} — {}",
                test.file,
                test.line,
                test.name,
                test.command.as_deref().unwrap_or("no command inferred")
            ));
        }
    }

    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text {
            text: truncate_output(lines.join("\n")),
        }],
        details: json!({
            "action": "tests",
            "target": target,
            "files_analyzed": files.len(),
            "tests": tests.iter().map(|test| json!({
                "file": test.file,
                "symbol": test.name,
                "line": test.line,
                "command": test.command,
                "why": test.why,
            })).collect::<Vec<_>>(),
        }),
        is_error: false,
    }
}

fn build_symbol_index(files: &[PathBuf], cwd: &Path) -> Vec<IndexedSymbol> {
    let result = extract_files(files, cwd);
    let mut symbols = Vec::new();
    for t in result.types.values() {
        symbols.push(IndexedSymbol {
            file: source_file(&t.source).to_string(),
            name: t.name.clone(),
            kind: format!("{:?}", t.kind).to_lowercase(),
            line: source_line(&t.source),
            text: format!(
                "{} {:?} {:?} {:?}",
                t.name, t.fields, t.variants, t.implements
            ),
            is_test: false,
        });
    }
    for f in result.functions.values() {
        symbols.push(IndexedSymbol {
            file: source_file(&f.source).to_string(),
            name: f.name.clone(),
            kind: "function".to_string(),
            line: source_line(&f.source),
            text: f.signature.clone(),
            is_test: f.is_test,
        });
    }
    symbols
}

fn search_index(
    index: &[IndexedSymbol],
    query: &str,
    mode: &str,
    max_results: usize,
) -> Vec<SearchHit> {
    let terms = query_terms(query);
    if terms.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for symbol in index {
        let mut score = 0;
        let mut why = Vec::new();
        let path = symbol.file.to_lowercase();
        let name = symbol.name.to_lowercase();
        let text = symbol.text.to_lowercase();
        for term in &terms {
            if name == *term {
                score += 100;
                why.push(format!("symbol exactly matches {term}"));
            } else if name.contains(term) {
                score += 60;
                why.push(format!("symbol contains {term}"));
            }
            if mode != "symbol" {
                if path.contains(term) {
                    score += 25;
                    why.push(format!("path contains {term}"));
                }
                if text.contains(term) {
                    score += if mode == "concept" { 20 } else { 15 };
                    why.push(format!("signature/metadata contains {term}"));
                }
            }
        }
        if score > 0 {
            hits.push(SearchHit {
                file: symbol.file.clone(),
                symbol: Some(symbol.name.clone()),
                kind: symbol.kind.clone(),
                line: symbol.line,
                score,
                why,
            });
        }
    }
    hits.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });
    hits.truncate(max_results);
    hits
}

#[derive(Debug, Clone)]
struct TestHit {
    file: String,
    name: String,
    line: usize,
    command: Option<String>,
    why: String,
}

fn discover_tests(
    index: &[IndexedSymbol],
    target: &str,
    cwd: &Path,
    max_results: usize,
) -> Vec<TestHit> {
    let (target_file, target_symbol) = split_target(target);
    let target_terms = query_terms(target_symbol.as_deref().unwrap_or(target));
    let cargo = cwd.join("Cargo.toml").exists();
    let mut hits = Vec::new();
    for symbol in index
        .iter()
        .filter(|symbol| symbol.is_test || looks_like_test_file(&symbol.file))
    {
        let mut score = 0;
        if let Some(file) = &target_file {
            if symbol.file == *file {
                score += 50;
            } else if same_stem_or_test_neighbor(&symbol.file, file) {
                score += 35;
            }
        }
        for term in &target_terms {
            if symbol.name.to_lowercase().contains(term) {
                score += 20;
            }
        }
        if score > 0 || target_file.is_none() {
            let test_name = symbol.name.rsplit("::").next().unwrap_or(&symbol.name);
            hits.push((
                score,
                TestHit {
                    file: symbol.file.clone(),
                    name: symbol.name.clone(),
                    line: symbol.line,
                    command: if cargo && symbol.file.ends_with(".rs") {
                        Some(format!("cargo test {test_name}"))
                    } else {
                        None
                    },
                    why: if symbol.is_test {
                        "indexed test symbol"
                    } else {
                        "test file naming"
                    }
                    .to_string(),
                },
            ));
        }
    }
    hits.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.file.cmp(&b.1.file)));
    hits.into_iter()
        .take(max_results)
        .map(|(_, hit)| hit)
        .collect()
}

fn query_terms(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(|term| term.to_lowercase())
        .collect()
}

fn source_line(source: &str) -> usize {
    source
        .rsplit_once(':')
        .and_then(|(_, line)| line.parse().ok())
        .unwrap_or(1)
}

fn split_target(target: &str) -> (Option<String>, Option<String>) {
    if let Some((file, symbol)) = target.split_once('#') {
        return (Some(file.to_string()), Some(symbol.to_string()));
    }
    if let Some((file, _line)) = target.rsplit_once(':') {
        return (Some(file.to_string()), None);
    }
    if target.contains('/') || target.contains('\\') {
        return (Some(target.to_string()), None);
    }
    (None, Some(target.to_string()))
}

fn looks_like_test_file(file: &str) -> bool {
    let name = Path::new(file)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(file);
    name.contains("test")
        || name.ends_with(".spec.ts")
        || name.ends_with(".spec.tsx")
        || name.ends_with("_test.go")
        || name.ends_with("_test.exs")
}

fn same_stem_or_test_neighbor(test_file: &str, target_file: &str) -> bool {
    let test_stem = Path::new(test_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .replace("_test", "")
        .replace(".test", "")
        .replace(".spec", "");
    let target_stem = Path::new(target_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    !target_stem.is_empty() && test_stem.contains(target_stem)
}

fn related_symbols<'a>(
    index: &'a [IndexedSymbol],
    target_file: Option<&str>,
    target_symbol: Option<&str>,
    max_results: usize,
) -> Vec<&'a IndexedSymbol> {
    let target_terms = target_symbol.map(query_terms).unwrap_or_default();
    let mut scored = Vec::new();
    for symbol in index {
        if target_symbol == Some(symbol.name.as_str()) {
            continue;
        }
        let mut score = 0;
        if target_file == Some(symbol.file.as_str()) {
            score += 50;
        }
        for term in &target_terms {
            if symbol.name.to_lowercase().contains(term)
                || symbol.text.to_lowercase().contains(term)
            {
                score += 15;
            }
        }
        if score > 0 {
            scored.push((score, symbol));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.line.cmp(&b.1.line)));
    scored
        .into_iter()
        .take(max_results)
        .map(|(_, symbol)| symbol)
        .collect()
}

fn collect_target_files(targets: &[String], cwd: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for target in targets {
        if let Some((file, _locator)) = parse_extract_target(target) {
            files.push(crate::tools::resolve_path(cwd, &file));
        }
    }
    files.sort();
    files.dedup();
    files
}

fn execute_related(mut files: Vec<PathBuf>, cwd: &Path, target: &str) -> ToolOutput {
    files.sort();
    files.dedup();
    let index = build_symbol_index(&files, cwd);
    let (target_file, target_symbol) = split_target(target);
    let related = related_symbols(&index, target_file.as_deref(), target_symbol.as_deref(), 12);
    let tests = discover_tests(&index, target, cwd, 5);
    let definition = target_symbol.as_ref().and_then(|name| {
        index.iter().find(|symbol| {
            symbol.name == *name && target_file.as_ref().is_none_or(|file| symbol.file == *file)
        })
    });

    let mut lines = vec![
        format!("Action: related"),
        format!("Target: {target}"),
        format!("Files analyzed: {}", files.len()),
    ];
    if let Some(symbol) = definition {
        lines.push(format!(
            "Definition: {}:{} [{}] {} (extract: {}#{})",
            symbol.file, symbol.line, symbol.kind, symbol.name, symbol.file, symbol.name
        ));
    }
    if !related.is_empty() {
        lines.push("Related symbols:".to_string());
        for symbol in &related {
            lines.push(format!(
                "- {}:{} [{}] {} (extract: {}#{})",
                symbol.file, symbol.line, symbol.kind, symbol.name, symbol.file, symbol.name
            ));
        }
    }
    if !tests.is_empty() {
        lines.push("Likely tests:".to_string());
        for test in &tests {
            lines.push(format!(
                "- {}:{} {} — {}",
                test.file,
                test.line,
                test.name,
                test.command.as_deref().unwrap_or("no command inferred")
            ));
        }
    }
    if definition.is_none() && related.is_empty() && tests.is_empty() {
        lines.push("No related context found.".to_string());
    }

    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text {
            text: truncate_output(lines.join("\n")),
        }],
        details: json!({
            "action": "related",
            "target": target,
            "files_analyzed": files.len(),
            "definition": definition.map(|symbol| json!({
                "file": symbol.file,
                "symbol": symbol.name,
                "kind": symbol.kind,
                "line": symbol.line,
                "extract_target": format!("{}#{}", symbol.file, symbol.name),
            })),
            "related": related.iter().map(|symbol| json!({
                "file": symbol.file,
                "symbol": symbol.name,
                "kind": symbol.kind,
                "line": symbol.line,
                "extract_target": format!("{}#{}", symbol.file, symbol.name),
            })).collect::<Vec<_>>(),
            "tests": tests.iter().map(|test| json!({
                "file": test.file,
                "symbol": test.name,
                "line": test.line,
                "command": test.command,
                "why": test.why,
            })).collect::<Vec<_>>(),
        }),
        is_error: false,
    }
}

#[derive(Debug, Clone)]
struct ReferenceHit {
    file: String,
    line: usize,
    kind: String,
    snippet: String,
    confidence: &'static str,
    why: String,
}

fn execute_references(
    mut files: Vec<PathBuf>,
    cwd: &Path,
    target: &str,
    max_results: usize,
) -> ToolOutput {
    files.sort();
    files.dedup();
    let index = build_symbol_index(&files, cwd);
    let hits = find_references(&files, cwd, &index, target, max_results);
    let mut lines = vec![
        "Action: references".to_string(),
        format!("Target: {target}"),
        "Accuracy: lexical/structural reference search, not a complete LSP call graph.".to_string(),
        format!("Files analyzed: {}", files.len()),
    ];
    if hits.is_empty() {
        lines.push("No references found.".to_string());
    } else {
        lines.push("References:".to_string());
        for hit in &hits {
            lines.push(format!(
                "- {}:{} [{} {}] {} — {}",
                hit.file, hit.line, hit.kind, hit.confidence, hit.snippet, hit.why
            ));
        }
    }

    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text {
            text: truncate_output(lines.join("\n")),
        }],
        details: json!({
            "action": "references",
            "target": target,
            "accuracy": "lexical_structural_not_lsp_complete",
            "files_analyzed": files.len(),
            "references": hits.iter().map(|hit| json!({
                "file": hit.file,
                "line": hit.line,
                "kind": hit.kind,
                "snippet": hit.snippet,
                "confidence": hit.confidence,
                "why": hit.why,
            })).collect::<Vec<_>>(),
        }),
        is_error: false,
    }
}

fn execute_impact(
    mut files: Vec<PathBuf>,
    cwd: &Path,
    target: &str,
    max_results: usize,
) -> ToolOutput {
    files.sort();
    files.dedup();
    let index = build_symbol_index(&files, cwd);
    let references = find_references(&files, cwd, &index, target, max_results);
    let tests = discover_tests(&index, target, cwd, 10);
    let (_target_file, target_symbol) = split_target(target);
    let public_status = target_symbol.as_ref().and_then(|name| {
        index
            .iter()
            .find(|symbol| symbol.name == *name)
            .map(|symbol| {
                if symbol.kind == "function" {
                    "unknown"
                } else {
                    "indexed symbol"
                }
            })
    });
    let affected_files: BTreeSet<String> = references.iter().map(|hit| hit.file.clone()).collect();
    let verify_commands: Vec<String> = tests
        .iter()
        .filter_map(|test| test.command.clone())
        .collect();
    let mut lines = vec![
        "Action: impact".to_string(),
        format!("Target: {target}"),
        "Accuracy: lexical/structural impact analysis, not a complete LSP call graph.".to_string(),
        format!("Files analyzed: {}", files.len()),
        format!("References found: {}", references.len()),
    ];
    if !affected_files.is_empty() {
        lines.push("Likely affected files:".to_string());
        for file in &affected_files {
            lines.push(format!("- {file}"));
        }
    }
    if !tests.is_empty() {
        lines.push("Relevant tests:".to_string());
        for test in &tests {
            lines.push(format!(
                "- {}:{} {} — {}",
                test.file,
                test.line,
                test.name,
                test.command.as_deref().unwrap_or("no command inferred")
            ));
        }
    }
    if !verify_commands.is_empty() {
        lines.push("Suggested verification:".to_string());
        for command in &verify_commands {
            lines.push(format!("- {command}"));
        }
    }

    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text {
            text: truncate_output(lines.join("\n")),
        }],
        details: json!({
            "action": "impact",
            "target": target,
            "accuracy": "lexical_structural_not_lsp_complete",
            "files_analyzed": files.len(),
            "public_status": public_status,
            "affected_files": affected_files.into_iter().collect::<Vec<_>>(),
            "references": references.iter().map(|hit| json!({
                "file": hit.file,
                "line": hit.line,
                "kind": hit.kind,
                "snippet": hit.snippet,
                "confidence": hit.confidence,
                "why": hit.why,
            })).collect::<Vec<_>>(),
            "tests": tests.iter().map(|test| json!({
                "file": test.file,
                "symbol": test.name,
                "line": test.line,
                "command": test.command,
                "why": test.why,
            })).collect::<Vec<_>>(),
            "verify_commands": verify_commands,
        }),
        is_error: false,
    }
}

fn find_references(
    files: &[PathBuf],
    cwd: &Path,
    index: &[IndexedSymbol],
    target: &str,
    max_results: usize,
) -> Vec<ReferenceHit> {
    let (_target_file, target_symbol) = split_target(target);
    let needle = target_symbol.unwrap_or_else(|| target.to_string());
    let needle = needle.trim();
    if needle.is_empty() {
        return Vec::new();
    }
    let pattern = format!(r"\b{}\b", regex::escape(needle));
    let Ok(symbol_regex) = regex::Regex::new(&pattern) else {
        return Vec::new();
    };
    let mut hits = Vec::new();
    for file in files {
        let Some(content) = read_text_file(file) else {
            continue;
        };
        let rel = file
            .strip_prefix(cwd)
            .unwrap_or(file)
            .to_string_lossy()
            .to_string();
        for (idx, line) in content.lines().enumerate() {
            if !symbol_regex.is_match(line) {
                continue;
            }
            let line_no = idx + 1;
            let (kind, confidence, why) = classify_reference(index, &rel, line_no, line, &needle);
            hits.push(ReferenceHit {
                file: rel.clone(),
                line: line_no,
                kind,
                snippet: truncate_line(line.trim(), 180),
                confidence,
                why,
            });
        }
    }
    hits.sort_by(|a, b| {
        reference_kind_rank(&a.kind)
            .cmp(&reference_kind_rank(&b.kind))
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });
    hits.truncate(max_results);
    hits
}

fn classify_reference(
    index: &[IndexedSymbol],
    file: &str,
    line_no: usize,
    line: &str,
    needle: &str,
) -> (String, &'static str, String) {
    if index
        .iter()
        .any(|symbol| symbol.file == file && symbol.line == line_no && symbol.name == needle)
    {
        return (
            "definition".to_string(),
            "high",
            "matches indexed symbol definition".to_string(),
        );
    }
    let trimmed = line.trim_start();
    if trimmed.starts_with("use ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("from ")
        || trimmed.contains("require(")
    {
        return (
            "import".to_string(),
            "medium",
            "line looks like import/use".to_string(),
        );
    }
    if looks_like_test_file(file)
        || index
            .iter()
            .any(|symbol| symbol.file == file && symbol.is_test && line_no >= symbol.line)
    {
        return (
            "test".to_string(),
            "medium",
            "reference appears in test context".to_string(),
        );
    }
    if line.contains(&format!("{needle}(")) || line.contains(&format!(".{needle}")) {
        return (
            "call".to_string(),
            "medium",
            "line looks like call/member access".to_string(),
        );
    }
    ("reference".to_string(), "low", "lexical match".to_string())
}

fn reference_kind_rank(kind: &str) -> usize {
    match kind {
        "definition" => 0,
        "call" => 1,
        "reference" => 2,
        "import" => 3,
        "test" => 4,
        _ => 5,
    }
}

// ── formatting ──────────────────────────────────────────────────────

fn format_result(
    result: &ScanResult,
    files: &[PathBuf],
    cwd: &Path,
    action: &str,
    task: Option<&str>,
) -> String {
    let mut sections = Vec::new();
    sections.push(format!("Action: {action}"));
    if let Some(task) = task {
        sections.push(format!("Task: {task}"));
    }
    sections.push(format!("Files analyzed: {}", files.len()));
    sections.push("Output: compact code skeleton with symbol kind and line ranges. Use `scan extract` targets like file#symbol, file:start-end, or file:line for exact code.".to_string());

    // Group types and functions by source file
    let mut file_types: BTreeMap<&str, Vec<&TypeInfo>> = BTreeMap::new();
    let mut file_functions: BTreeMap<&str, Vec<&FunctionInfo>> = BTreeMap::new();

    for t in result.types.values() {
        let file = source_file(&t.source);
        file_types.entry(file).or_default().push(t);
    }

    for f in result.functions.values() {
        let file = source_file(&f.source);
        file_functions.entry(file).or_default().push(f);
    }

    let all_files: BTreeSet<&str> = file_types
        .keys()
        .chain(file_functions.keys())
        .copied()
        .collect();

    for file in &all_files {
        let rel = display_path(file, cwd);
        let mut lines = vec![rel];

        if let Some(types) = file_types.get(file) {
            lines.push(format!("  Types ({}):", types.len()));
            for t in types {
                lines.push(format!("    - {}", format_type(t)));
            }
        }

        if let Some(funcs) = file_functions.get(file) {
            // Standalone functions only (not Type::method — those show under Types)
            let standalone: Vec<_> = funcs
                .iter()
                .filter(|f| !f.name.contains("::") && !is_qualified_name(&f.name))
                .filter(|f| !f.is_test)
                .collect();
            if !standalone.is_empty() {
                lines.push(format!("  Functions ({}):", standalone.len()));
                for f in standalone {
                    lines.push(format!("    - {}", format_function(f)));
                }
            }
        }

        if lines.len() > 1 {
            sections.push(lines.join("\n"));
        }
    }

    sections.join("\n\n")
}

fn format_type(t: &TypeInfo) -> String {
    let vis = format_visibility(&t.visibility);
    let kind = match t.kind {
        TypeKind::Struct => "struct",
        TypeKind::Enum => "enum",
        TypeKind::Trait => "trait",
        TypeKind::Interface => "interface",
        TypeKind::Class => "class",
        TypeKind::TypeAlias => "type",
        TypeKind::Union => "union",
        TypeKind::Protocol => "protocol",
    };

    let mut out = format!("{vis}{kind} {}", t.name);

    match t.kind {
        TypeKind::Struct | TypeKind::Class => {
            if !t.fields.is_empty() {
                let names: Vec<&str> = t.fields.iter().map(|f| f.name.as_str()).collect();
                if names.len() <= 6 {
                    out.push_str(&format!(" {{ {} }}", names.join(", ")));
                } else {
                    let shown = &names[..5];
                    out.push_str(&format!(
                        " {{ {}, ... +{} }}",
                        shown.join(", "),
                        names.len() - 5
                    ));
                }
            }
        }
        TypeKind::Enum => {
            if !t.variants.is_empty() {
                if t.variants.len() <= 6 {
                    out.push_str(&format!(" {{ {} }}", t.variants.join(", ")));
                } else {
                    let shown: Vec<&str> = t.variants[..5].iter().map(|s| s.as_str()).collect();
                    out.push_str(&format!(
                        " {{ {}, ... +{} }}",
                        shown.join(", "),
                        t.variants.len() - 5
                    ));
                }
            }
        }
        TypeKind::Trait | TypeKind::Interface | TypeKind::Protocol => {
            if !t.methods.is_empty() {
                if t.methods.len() <= 6 {
                    out.push_str(&format!(" {{ {} }}", t.methods.join(", ")));
                } else {
                    let shown: Vec<&str> = t.methods[..5].iter().map(|s| s.as_str()).collect();
                    out.push_str(&format!(
                        " {{ {}, ... +{} }}",
                        shown.join(", "),
                        t.methods.len() - 5
                    ));
                }
            }
        }
        _ => {}
    }

    if !t.implements.is_empty() {
        out.push_str(&format!(" [{}]", t.implements.join(", ")));
    }

    out.push_str(&format!(" @ {}", t.source));

    out
}

fn format_function(f: &FunctionInfo) -> String {
    let vis = format_visibility(&f.visibility);
    let mut out = if !f.signature.is_empty() {
        format!("{vis}{}", f.signature)
    } else {
        format!("{vis}fn {}", f.name)
    };
    out.push_str(&format!(" @ {}", f.source));
    out
}

fn format_visibility(vis: &Visibility) -> &'static str {
    match vis {
        Visibility::Public => "pub ",
        Visibility::Internal => "pub(crate) ",
        Visibility::Private => "",
    }
}

fn source_file(source: &str) -> &str {
    // "src/lib.rs:42" → "src/lib.rs"
    source.rsplit_once(':').map(|(f, _)| f).unwrap_or(source)
}

fn display_path(path: &str, cwd: &Path) -> String {
    let cwd_str = cwd.to_string_lossy();
    path.strip_prefix(cwd_str.as_ref())
        .map(|p| p.strip_prefix('/').unwrap_or(p))
        .unwrap_or(path)
        .to_string()
}

fn is_qualified_name(name: &str) -> bool {
    // "Type::method" or "module.function" patterns
    name.contains("::")
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
            .map(|p| format!(". Full output saved to {}", p.display()))
            .unwrap_or_default()
    ));
    result
}

struct CodeBlock {
    file: PathBuf,
    start_line: usize,
    end_line: usize,
    kind: Option<String>,
    symbol: Option<String>,
    language: Option<String>,
    truncated: bool,
    code: String,
}

enum Locator {
    Line(usize),
    Range(usize, usize),
    Symbol(String),
}

fn execute_extract_with_preset(
    targets: &[String],
    preset: Option<&str>,
    ctx: &ToolContext,
) -> ToolOutput {
    let Some(preset) = preset else {
        return execute_extract(targets, ctx);
    };
    match preset {
        "definition" | "module_context" => execute_extract(targets, ctx),
        "edit_context" | "test_context" => execute_context_preset(targets, preset, ctx),
        other => ToolOutput::error(format!(
            "unknown extract preset: {other}. Use definition, edit_context, module_context, or test_context."
        )),
    }
}

fn execute_context_preset(targets: &[String], preset: &str, ctx: &ToolContext) -> ToolOutput {
    let mut output = execute_extract(targets, ctx);
    if output.is_error {
        return output;
    }
    let files = collect_target_files(targets, &ctx.cwd);
    let index = build_symbol_index(&files, &ctx.cwd);
    let mut extras = Vec::new();
    for target in targets {
        let tests = discover_tests(&index, target, &ctx.cwd, 5);
        if preset == "test_context" && !tests.is_empty() {
            extras.push(format!("Related tests for {target}:"));
            for test in tests {
                extras.push(format!(
                    "- {}:{} {} — {}",
                    test.file,
                    test.line,
                    test.name,
                    test.command
                        .unwrap_or_else(|| "no command inferred".to_string())
                ));
            }
        } else if preset == "edit_context" {
            let (target_file, target_symbol) = split_target(target);
            let related =
                related_symbols(&index, target_file.as_deref(), target_symbol.as_deref(), 6);
            if !related.is_empty() {
                extras.push(format!("Related symbols for {target}:"));
                for symbol in related {
                    extras.push(format!(
                        "- {}:{} [{}] {} (extract: {}#{})",
                        symbol.file,
                        symbol.line,
                        symbol.kind,
                        symbol.name,
                        symbol.file,
                        symbol.name
                    ));
                }
            }
        }
    }
    if !extras.is_empty() {
        if let Some(text) = output.content.iter_mut().find_map(|block| match block {
            imp_llm::ContentBlock::Text { text } => Some(text),
            _ => None,
        }) {
            text.push_str("\n\n");
            text.push_str(&extras.join("\n"));
            *text = truncate_output(text.clone());
        }
        output.details["preset"] = json!(preset);
        output.details["context"] = json!(extras);
    }
    output
}

fn execute_extract(targets: &[String], ctx: &ToolContext) -> ToolOutput {
    let mut blocks = Vec::new();
    let mut errors = Vec::new();

    for target in targets {
        let Some((file, locator)) = parse_extract_target(target) else {
            errors.push(format!(
                "Invalid target `{target}`. Use file#symbol, file:start-end, or file:line."
            ));
            continue;
        };

        let path = crate::tools::resolve_path(&ctx.cwd, &file);
        let Some(content) = read_text_file(&path) else {
            blocks.push(CodeBlock {
                file: PathBuf::from(&file),
                start_line: 0,
                end_line: 0,
                kind: None,
                symbol: None,
                language: language_for_path(Path::new(&file)).map(str::to_string),
                truncated: false,
                code: format!("Error: could not read {file}"),
            });
            continue;
        };

        let rel_path = path.strip_prefix(&ctx.cwd).unwrap_or(&path).to_path_buf();

        match locator {
            Locator::Line(line) => {
                let line_idx = line.saturating_sub(1);
                if let Some(extracted) = extract_blocks_at_lines(&content, &path, &[line_idx]) {
                    for mut block in extracted {
                        block.file = rel_path.clone();
                        blocks.push(block);
                    }
                } else {
                    let lines: Vec<&str> = content.lines().collect();
                    let start = line_idx.saturating_sub(5);
                    let end = (line_idx + 6).min(lines.len());
                    blocks.push(CodeBlock {
                        file: rel_path.clone(),
                        start_line: start + 1,
                        end_line: end,
                        kind: None,
                        symbol: None,
                        language: language_for_path(&path).map(str::to_string),
                        truncated: false,
                        code: lines[start..end].join("\n"),
                    });
                }
            }
            Locator::Range(start, end) => {
                let lines: Vec<&str> = content.lines().collect();
                let s = start.saturating_sub(1).min(lines.len());
                let e = end.min(lines.len());
                blocks.push(CodeBlock {
                    file: rel_path.clone(),
                    start_line: s + 1,
                    end_line: e,
                    kind: None,
                    symbol: None,
                    language: language_for_path(&path).map(str::to_string),
                    truncated: false,
                    code: lines[s..e].join("\n"),
                });
            }
            Locator::Symbol(name) => {
                if let Some(found) = extract_symbol(&content, &path, &name) {
                    blocks.push(CodeBlock {
                        file: rel_path.clone(),
                        ..found
                    });
                } else {
                    blocks.push(CodeBlock {
                        file: rel_path.clone(),
                        start_line: 0,
                        end_line: 0,
                        kind: None,
                        symbol: Some(name.clone()),
                        language: language_for_path(&path).map(str::to_string),
                        truncated: false,
                        code: format!("Symbol '{name}' not found in {file}"),
                    });
                }
            }
        }
    }

    if blocks.is_empty() && errors.is_empty() {
        return ToolOutput::text("No code blocks found.");
    }

    let mut output = String::new();
    if !blocks.is_empty() {
        output.push_str(&format_blocks(&blocks));
    }
    if !errors.is_empty() {
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        output.push_str("Errors:\n");
        for error in &errors {
            output.push_str(&format!("- {error}\n"));
        }
    }

    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text {
            text: truncate_output(output),
        }],
        details: json!({
            "action": "extract",
            "targets_count": targets.len(),
            "blocks_count": blocks.len(),
            "errors": errors,
            "blocks": blocks.iter().map(block_details).collect::<Vec<_>>(),
        }),
        is_error: blocks.is_empty(),
    }
}

fn parse_extract_target(target: &str) -> Option<(String, Locator)> {
    if let Some(hash_pos) = target.rfind('#') {
        let file = target[..hash_pos].to_string();
        let symbol = target[hash_pos + 1..].to_string();
        if !file.is_empty() && !symbol.is_empty() {
            return Some((file, Locator::Symbol(symbol)));
        }
    }

    if let Some(colon_pos) = target.rfind(':') {
        let file = target[..colon_pos].to_string();
        let suffix = &target[colon_pos + 1..];
        if !file.is_empty() && !suffix.is_empty() {
            if let Some(dash_pos) = suffix.find('-') {
                let start = suffix[..dash_pos].parse::<usize>().ok()?;
                let end = suffix[dash_pos + 1..].parse::<usize>().ok()?;
                if start == 0 || end == 0 || start > end {
                    return None;
                }
                return Some((file, Locator::Range(start, end)));
            } else if let Ok(line) = suffix.parse::<usize>() {
                if line == 0 {
                    return None;
                }
                return Some((file, Locator::Line(line)));
            }
        }
    }

    None
}

fn block_details(block: &CodeBlock) -> serde_json::Value {
    json!({
        "path": block.file.to_string_lossy(),
        "symbol": block.symbol,
        "kind": block.kind,
        "language": block.language,
        "start_line": block.start_line,
        "end_line": block.end_line,
        "truncated": block.truncated,
    })
}

fn read_text_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.contains(&0) {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn get_parser(path: &Path) -> Option<tree_sitter::Parser> {
    let ext = path.extension()?.to_str()?;
    let language = match ext {
        "rs" => tree_sitter_rust::LANGUAGE.into(),
        "ts" | "tsx" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "js" | "jsx" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "py" => tree_sitter_python::LANGUAGE.into(),
        "go" => tree_sitter_go::LANGUAGE.into(),
        "kt" | "kts" => tree_sitter_kotlin_ng::LANGUAGE.into(),
        _ => return None,
    };
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).ok()?;
    Some(parser)
}

fn extract_blocks_at_lines(
    source: &str,
    path: &Path,
    match_lines: &[usize],
) -> Option<Vec<CodeBlock>> {
    let mut parser = get_parser(path)?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();
    let lines: Vec<&str> = source.lines().collect();

    let mut blocks = Vec::new();
    let mut seen_ranges = std::collections::HashSet::new();

    for &line_idx in match_lines {
        if let Some(node) = find_enclosing_block(root, line_idx) {
            let start = node.start_position().row;
            let end = node.end_position().row;
            let range = (start, end);
            if seen_ranges.insert(range) {
                let s = start.min(lines.len());
                let e = (end + 1).min(lines.len());
                blocks.push(CodeBlock {
                    file: PathBuf::new(),
                    start_line: start + 1,
                    end_line: end + 1,
                    kind: Some(node.kind().to_string()),
                    symbol: None,
                    language: language_for_path(path).map(str::to_string),
                    truncated: false,
                    code: lines[s..e].join("\n"),
                });
            }
        }
    }

    Some(blocks)
}

fn find_enclosing_block(root: tree_sitter::Node, target_line: usize) -> Option<tree_sitter::Node> {
    let mut best: Option<tree_sitter::Node> = None;
    find_enclosing_block_recursive(root, target_line, &mut best);
    best
}

fn find_enclosing_block_recursive<'a>(
    node: tree_sitter::Node<'a>,
    target_line: usize,
    best: &mut Option<tree_sitter::Node<'a>>,
) {
    let start = node.start_position().row;
    let end = node.end_position().row;

    if target_line < start || target_line > end {
        return;
    }

    if BLOCK_KINDS.contains(&node.kind()) {
        *best = Some(node);
    }

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    for child in children {
        find_enclosing_block_recursive(child, target_line, best);
    }
}

fn extract_symbol(source: &str, path: &Path, name: &str) -> Option<CodeBlock> {
    let mut parser = get_parser(path)?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();
    let lines: Vec<&str> = source.lines().collect();

    let node = find_symbol_node(root, source, name)?;
    let start = node.start_position().row;
    let end = node.end_position().row;
    let s = start.min(lines.len());
    let e = (end + 1).min(lines.len());

    Some(CodeBlock {
        file: PathBuf::new(),
        start_line: start + 1,
        end_line: end + 1,
        kind: Some(node.kind().to_string()),
        symbol: Some(name.to_string()),
        language: language_for_path(path).map(str::to_string),
        truncated: false,
        code: lines[s..e].join("\n"),
    })
}

fn find_symbol_node<'a>(
    node: tree_sitter::Node<'a>,
    source: &str,
    name: &str,
) -> Option<tree_sitter::Node<'a>> {
    if BLOCK_KINDS.contains(&node.kind()) && node_has_name(node, source, name) {
        return Some(node);
    }

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    for child in children {
        if let Some(found) = find_symbol_node(child, source, name) {
            return Some(found);
        }
    }

    None
}

fn node_has_name(node: tree_sitter::Node, source: &str, name: &str) -> bool {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    for child in children {
        let kind = child.kind();
        if kind == "identifier"
            || kind == "type_identifier"
            || kind == "name"
            || kind == "property_identifier"
            || kind == "simple_identifier"
            || kind == "variable_identifier"
        {
            let text = &source[child.byte_range()];
            if text == name {
                return true;
            }
        }
        if BLOCK_KINDS.contains(&kind) {
            continue;
        }
        let mut inner_cursor = child.walk();
        let inner_children: Vec<_> = child.children(&mut inner_cursor).collect();
        for inner in inner_children {
            let ik = inner.kind();
            if ik == "identifier"
                || ik == "type_identifier"
                || ik == "name"
                || ik == "simple_identifier"
                || ik == "variable_identifier"
            {
                let text = &source[inner.byte_range()];
                if text == name {
                    return true;
                }
            }
        }
    }
    false
}

fn language_for_path(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|e| e.to_str())? {
        "rs" => Some("rust"),
        "ts" | "tsx" => Some("typescript"),
        "js" | "jsx" => Some("javascript"),
        "py" => Some("python"),
        "go" => Some("go"),
        "kt" | "kts" => Some("kotlin"),
        _ => None,
    }
}

fn format_blocks(blocks: &[CodeBlock]) -> String {
    let mut sections = Vec::with_capacity(blocks.len());

    for block in blocks {
        let mut header = format!(
            "{}:{}-{}",
            block.file.display(),
            block.start_line,
            block.end_line
        );
        if let Some(kind) = &block.kind {
            header.push_str(&format!(" ({kind})"));
        }
        let details = block_details(block);

        let fence = match block.file.extension().and_then(|e| e.to_str()) {
            Some("rs") => "rust",
            Some("ts") | Some("tsx") => "typescript",
            Some("js") | Some("jsx") => "javascript",
            Some("py") => "python",
            Some("go") => "go",
            _ => "text",
        };
        sections.push(format!(
            "{header}\nDetails: {details}\n```{fence}\n{}\n```",
            block.code
        ));
    }

    sections.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_uses_directory_files_extract_and_targets() {
        let schema = ScanTool.parameters();
        let properties = schema["properties"].as_object().unwrap();
        let actions = properties["action"]["enum"].as_array().unwrap();
        assert!(actions.iter().any(|value| value == "directory"));
        assert!(actions.iter().any(|value| value == "files"));
        assert!(actions.iter().any(|value| value == "extract"));
        assert!(actions.iter().any(|value| value == "search"));
        assert!(actions.iter().any(|value| value == "tests"));
        assert!(actions.iter().any(|value| value == "related"));
        assert!(actions.iter().any(|value| value == "references"));
        assert!(actions.iter().any(|value| value == "impact"));
        assert!(properties.contains_key("targets"));
        assert!(properties.contains_key("query"));
        assert!(properties.contains_key("mode"));
        assert!(properties.contains_key("max_results"));
        assert!(properties.contains_key("preset"));
        assert!(properties.contains_key("target"));
        assert!(!properties.contains_key("task"));
    }

    #[test]
    fn search_index_returns_ranked_symbol_hits() {
        let index = vec![
            IndexedSymbol {
                file: "src/auth/session.rs".to_string(),
                name: "resolve_auth_fallback".to_string(),
                kind: "function".to_string(),
                line: 12,
                text: "fn resolve_auth_fallback()".to_string(),
                is_test: false,
            },
            IndexedSymbol {
                file: "src/cache.rs".to_string(),
                name: "load_cache".to_string(),
                kind: "function".to_string(),
                line: 4,
                text: "fn load_cache()".to_string(),
                is_test: false,
            },
        ];

        let hits = search_index(&index, "auth fallback", "concept", 5);

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].symbol.as_deref(), Some("resolve_auth_fallback"));
        assert!(hits[0]
            .why
            .iter()
            .any(|why| why.contains("symbol contains")));
    }

    #[test]
    fn discover_tests_suggests_cargo_test_for_rust_tests() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let index = vec![IndexedSymbol {
            file: "src/session.rs".to_string(),
            name: "falls_back_to_env_token".to_string(),
            kind: "function".to_string(),
            line: 42,
            text: "fn falls_back_to_env_token()".to_string(),
            is_test: true,
        }];

        let tests = discover_tests(
            &index,
            "src/session.rs#resolve_auth_fallback",
            tmp.path(),
            5,
        );

        assert_eq!(tests.len(), 1);
        assert_eq!(
            tests[0].command.as_deref(),
            Some("cargo test falls_back_to_env_token")
        );
    }

    #[test]
    fn related_symbols_returns_same_file_context() {
        let index = vec![
            IndexedSymbol {
                file: "src/session.rs".to_string(),
                name: "resolve_auth_fallback".to_string(),
                kind: "function".to_string(),
                line: 10,
                text: "fn resolve_auth_fallback()".to_string(),
                is_test: false,
            },
            IndexedSymbol {
                file: "src/session.rs".to_string(),
                name: "SessionConfig".to_string(),
                kind: "struct".to_string(),
                line: 2,
                text: "SessionConfig".to_string(),
                is_test: false,
            },
        ];

        let related = related_symbols(
            &index,
            Some("src/session.rs"),
            Some("resolve_auth_fallback"),
            5,
        );

        assert_eq!(related.len(), 1);
        assert_eq!(related[0].name, "SessionConfig");
    }

    #[test]
    fn collect_target_files_extracts_paths_from_targets() {
        let cwd = Path::new("/tmp/example");
        let files = collect_target_files(&["src/lib.rs#run".to_string()], cwd);

        assert_eq!(files, vec![PathBuf::from("/tmp/example/src/lib.rs")]);
    }

    #[test]
    fn references_classify_definitions_and_calls() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("lib.rs");
        std::fs::write(
            &file,
            "fn resolve_auth_fallback() {}\nfn caller() { resolve_auth_fallback(); }\n",
        )
        .unwrap();
        let files = vec![file];
        let index = build_symbol_index(&files, tmp.path());

        let references = find_references(&files, tmp.path(), &index, "resolve_auth_fallback", 10);

        assert!(references.iter().any(|hit| hit.kind == "definition"));
        assert!(references.iter().any(|hit| hit.kind == "call"));
    }

    #[test]
    fn impact_uses_tests_for_verification_commands() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let file = tmp.path().join("session.rs");
        std::fs::write(
            &file,
            "fn resolve_auth_fallback() {}\n#[test]\nfn resolve_auth_fallback_uses_env() { resolve_auth_fallback(); }\n",
        )
        .unwrap();

        let output = execute_impact(
            vec![file],
            tmp.path(),
            "session.rs#resolve_auth_fallback",
            10,
        );

        assert_eq!(output.details["action"], "impact");
        assert!(output.details["verify_commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("cargo test resolve_auth_fallback_uses_env")));
    }

    #[test]
    fn parse_extract_target_rejects_invalid_lines() {
        assert!(parse_extract_target("src/lib.rs:0").is_none());
        assert!(parse_extract_target("src/lib.rs:10-2").is_none());
        assert!(parse_extract_target("src/lib.rs:1-2").is_some());
    }

    #[test]
    fn execute_extract_reports_invalid_target_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(16);
        let ctx = ToolContext {
            cwd: tmp.path().to_path_buf(),
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            command_tx: cmd_tx,
            ui: std::sync::Arc::new(crate::ui::NullInterface),
            file_cache: std::sync::Arc::new(crate::tools::FileCache::new()),
            checkpoint_state: std::sync::Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: std::sync::Arc::new(std::sync::Mutex::new(
                crate::tools::FileTracker::new(),
            )),
            anchor_store: std::sync::Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
            turn_mana_review: std::sync::Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            run_policy: Default::default(),
            config: std::sync::Arc::new(crate::config::Config::default()),
        };

        let output = execute_extract(&["not-a-target".to_string()], &ctx);

        assert!(output.is_error);
        assert_eq!(output.details["action"], "extract");
        assert_eq!(output.details["errors"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn extract_rust_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("sample.rs");
        std::fs::write(
            &file,
            r#"
pub struct User {
    pub name: String,
    pub age: u32,
}

pub enum Status { Active, Inactive }

pub trait Validate {
    fn validate(&self) -> bool;
}

impl Validate for User {
    fn validate(&self) -> bool { true }
}

pub async fn load_user(id: &str) -> Result<User> { todo!() }
fn internal_helper() {}
"#,
        )
        .unwrap();

        let result = extract_files(&[file], tmp.path());

        // Types extracted
        assert!(result.types.contains_key("User"));
        assert!(result.types.contains_key("Status"));
        assert!(result.types.contains_key("Validate"));

        // User has fields
        let user = &result.types["User"];
        assert_eq!(user.fields.len(), 2);
        assert_eq!(user.visibility, Visibility::Public);

        // Status has variants
        let status = &result.types["Status"];
        assert_eq!(status.variants, vec!["Active", "Inactive"]);

        // Validate has methods
        let validate = &result.types["Validate"];
        assert!(validate.methods.contains(&"validate".to_string()));

        // User implements Validate
        assert!(user.implements.contains(&"Validate".to_string()));

        // Functions extracted with signatures
        let load = &result.functions["load_user"];
        assert!(load.is_async);
        assert!(load.signature.contains("-> Result<User>"));
        assert_eq!(load.visibility, Visibility::Public);

        let helper = &result.functions["internal_helper"];
        assert_eq!(helper.visibility, Visibility::Private);
    }

    #[test]
    fn extract_typescript_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("models.ts");
        std::fs::write(
            &file,
            r#"
export interface User {
    name: string;
    email: string;
}

export enum Status {
    Active = "active",
    Inactive = "inactive",
}

export async function fetchUser(id: string): Promise<User> {
    return {} as User;
}

function internalHelper(): void {}
"#,
        )
        .unwrap();

        let result = extract_files(&[file], tmp.path());
        assert!(result.types.contains_key("User"));
        assert!(result.types.contains_key("Status"));
        assert_eq!(result.types["User"].visibility, Visibility::Public);
        assert_eq!(result.types["Status"].variants, vec!["Active", "Inactive"]);
        assert!(result.functions["fetchUser"].is_async);
        assert_eq!(
            result.functions["internalHelper"].visibility,
            Visibility::Private
        );
    }

    #[test]
    fn format_output_shows_rich_info() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("lib.rs");
        std::fs::write(
            &file,
            r#"
pub struct Config { pub host: String, pub port: u16 }
pub enum Mode { Debug, Release }
pub fn start(config: &Config) -> Result<()> { todo!() }
"#,
        )
        .unwrap();

        let result = extract_files(std::slice::from_ref(&file), tmp.path());
        let output = format_result(&result, &[file], tmp.path(), "extract", None);

        assert!(output.contains("pub struct Config { host, port }"));
        assert!(output.contains("pub enum Mode { Debug, Release }"));
        assert!(output.contains("pub fn start"));
        assert!(output.contains("-> Result<()>"));
    }

    #[test]
    fn skeleton_output_includes_line_ranges_and_target_hint() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("lib.rs");
        std::fs::write(
            &file,
            r#"
pub struct Config { pub host: String, pub port: u16 }
pub fn start(config: &Config) -> Result<()> { todo!() }
"#,
        )
        .unwrap();

        let result = extract_files(std::slice::from_ref(&file), tmp.path());
        let output = format_result(&result, &[file], tmp.path(), "build", None);

        assert!(output.contains("compact code skeleton"));
        assert!(output.contains("file#symbol"));
        assert!(output.contains("pub struct Config"));
        assert!(output.contains(" @ lib.rs:2"));
        assert!(output.contains("pub fn start"));
        assert!(output.contains(" @ lib.rs:3"));
    }

    #[test]
    fn symbol_extract_includes_structured_details() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("lib.rs");
        std::fs::write(
            &file,
            r#"
pub struct Config {
    pub host: String,
}

pub fn start(config: &Config) -> Result<()> { todo!() }
"#,
        )
        .unwrap();

        let found =
            extract_symbol(&std::fs::read_to_string(&file).unwrap(), &file, "Config").unwrap();
        let output = format_blocks(&[CodeBlock {
            file: PathBuf::from("lib.rs"),
            ..found
        }]);

        assert!(output.contains("Details:"));
        assert!(output.contains("\"symbol\":\"Config\""));
        assert!(output.contains("\"language\":\"rust\""));
        assert!(output.contains("\"start_line\":2"));
        assert!(output.contains("pub struct Config"));
    }

    #[test]
    fn typescript_skeleton_output_includes_line_ranges() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("models.ts");
        std::fs::write(
            &file,
            r#"
export interface User { name: string; }
export async function fetchUser(id: string): Promise<User> { return {} as User; }
"#,
        )
        .unwrap();

        let result = extract_files(std::slice::from_ref(&file), tmp.path());
        let output = format_result(&result, &[file], tmp.path(), "build", None);

        assert!(output.contains("pub interface User @ models.ts:2"));
        assert!(output.contains("pub async function fetchUser"));
        assert!(output.contains(" @ models.ts:3"));
    }
}
