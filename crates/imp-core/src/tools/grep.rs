//! Grep tool — native code search with optional tree-sitter block extraction.
//!
//! Replaces both `rg` (ripgrep) and `probe` with a single native tool:
//! - Default: line matches (like ripgrep)
//! - `blocks: true`: return complete enclosing code blocks via tree-sitter
//! - `extract`: get blocks by file:line or file#symbol
//!
//! Uses the `ignore` crate for .gitignore-aware parallel file walking.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::json;

use super::{truncate_head, truncate_line, Tool, ToolContext, ToolOutput, TruncationResult};
use crate::error::Result;

const DEFAULT_LIMIT: usize = 100;
const DEFAULT_BLOCK_LIMIT: usize = 10;
const MAX_OUTPUT_LINES: usize = 2000;
const MAX_OUTPUT_BYTES: usize = 50 * 1024;
const MAX_LINE_CHARS: usize = 500;

/// Node kinds that represent "enclosing blocks" we want to extract.
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
    "lexical_declaration", // const/let at top level
    "variable_declaration",
    "arrow_function",
    // Python
    "function_definition",
    "class_definition",
    "decorated_definition",
    // Go
    "function_declaration",
    "method_declaration",
    "type_declaration",
    "type_spec",
];

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
        "Search file contents or extract code blocks. Supports boolean queries (AND/OR/NOT), phrases, and stemming. Set blocks=true for complete code blocks via tree-sitter. Use extract for blocks at file:line or file#symbol."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Search query. Supports AND/OR/NOT and \"phrases\"." },
                "path": { "type": "string", "description": "Directory or file to search" },
                "glob": { "type": "string", "description": "Filter files by glob, e.g. '*.rs'" },
                "language": { "type": "string", "description": "Filter by language: rust, typescript, python, go, etc." },
                "ignoreCase": { "type": "boolean", "description": "Case-insensitive search" },
                "exact": { "type": "boolean", "description": "Exact match without stemming (default: false)" },
                "literal": { "type": "boolean", "description": "Treat pattern as literal string (no regex)" },
                "context": { "type": "number", "description": "Context lines before/after match" },
                "limit": { "type": "number", "description": "Max matches (default: 100, or 10 for blocks)" },
                "blocks": { "type": "boolean", "description": "Return complete code blocks enclosing matches instead of lines" },
                "allowTests": { "type": "boolean", "description": "Include test files (default: false)" },
                "extract": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Extract blocks at locations: 'file:line', 'file:start-end', or 'file#symbol'"
                }
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
        // Extract mode: get blocks at specific locations
        if let Some(targets) = params["extract"].as_array() {
            if !targets.is_empty() {
                return execute_extract(targets, &ctx).await;
            }
        }

        // Search mode: need a pattern
        let pattern = match params["pattern"].as_str() {
            Some(p) if !p.is_empty() => p,
            _ => {
                return Ok(ToolOutput::error(
                    "Missing 'pattern' parameter (or use 'extract' for block extraction)",
                ))
            }
        };

        let search_path = params["path"]
            .as_str()
            .map(|p| super::resolve_path(&ctx.cwd, p))
            .unwrap_or_else(|| ctx.cwd.clone());

        let glob_filter = params["glob"].as_str();
        let language = params["language"].as_str();
        let ignore_case = params["ignoreCase"].as_bool().unwrap_or(false);
        let exact = params["exact"].as_bool().unwrap_or(false);
        let literal = params["literal"].as_bool().unwrap_or(false);
        let context_lines = params["context"].as_u64().map(|n| n as usize);
        let blocks = params["blocks"].as_bool().unwrap_or(false);
        let allow_tests = params["allowTests"].as_bool().unwrap_or(false);

        let default_limit = if blocks {
            DEFAULT_BLOCK_LIMIT
        } else {
            DEFAULT_LIMIT
        };
        let limit = params["limit"]
            .as_u64()
            .map(|n| n as usize)
            .unwrap_or(default_limit);

        // Build the search query — supports boolean operators, phrases, stemming
        let query = if literal {
            // Literal mode: single exact term, no parsing
            super::query::parse(&format!("\"{}\"", pattern), true, ignore_case)
                .map_err(crate::error::Error::Tool)?
        } else {
            super::query::parse(pattern, exact, ignore_case).map_err(crate::error::Error::Tool)?
        };

        let file_filter = FileFilter {
            glob: glob_filter.map(String::from),
            language: language.map(String::from),
            allow_tests,
        };

        if blocks {
            execute_block_search(&query, &search_path, &file_filter, limit, &ctx.cwd)
        } else {
            execute_line_search(
                &query,
                &search_path,
                &file_filter,
                context_lines,
                limit,
                &ctx.cwd,
            )
        }
    }
}

use super::query::Query;

struct FileFilter {
    glob: Option<String>,
    language: Option<String>,
    allow_tests: bool,
}

impl FileFilter {
    fn accepts(&self, path: &Path) -> bool {
        // Language filter
        if let Some(ref lang) = self.language {
            if let Some(exts) = super::query::language_extensions(lang) {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if !exts.contains(&ext) {
                    return false;
                }
            }
        }

        // Test file exclusion
        if !self.allow_tests && super::query::is_test_file(path) {
            return false;
        }

        true
    }
}

// ── line search (classic grep) ──────────────────────────────────────

fn execute_line_search(
    query: &Query,
    search_path: &Path,
    file_filter: &FileFilter,
    context_lines: Option<usize>,
    limit: usize,
    cwd: &Path,
) -> Result<ToolOutput> {
    let mut results = Vec::new();
    let mut match_count = 0;

    for entry in walk_files(search_path, file_filter) {
        if match_count >= limit {
            break;
        }

        let content = match read_text_file(&entry) {
            Some(c) => c,
            None => continue,
        };

        // For boolean AND queries, check file-level match first
        if !query.must.is_empty() && !query.matches_file(&content) {
            continue;
        }

        let rel_path = entry.strip_prefix(cwd).unwrap_or(&entry);
        let lines: Vec<&str> = content.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            if match_count >= limit {
                break;
            }
            if !query.matches_line(line) {
                continue;
            }

            match_count += 1;

            if let Some(ctx) = context_lines {
                let start = line_idx.saturating_sub(ctx);
                let end = (line_idx + ctx + 1).min(lines.len());

                if !results.is_empty() {
                    results.push("--".to_string());
                }
                for (i, &ctx_line) in lines.iter().enumerate().take(end).skip(start) {
                    let sep = if i == line_idx { ':' } else { '-' };
                    results.push(format!(
                        "{}{}{}{}",
                        rel_path.display(),
                        sep,
                        i + 1,
                        format_args!("{sep}{}", ctx_line)
                    ));
                }
            } else {
                results.push(format!("{}:{}:{}", rel_path.display(), line_idx + 1, line));
            }
        }
    }

    if results.is_empty() {
        return Ok(ToolOutput::text("No matches found."));
    }

    Ok(ToolOutput::text(truncate_text(results.join("\n"))))
}

// ── block search (grep + tree-sitter) ───────────────────────────────

fn execute_block_search(
    query: &Query,
    search_path: &Path,
    file_filter: &FileFilter,
    limit: usize,
    cwd: &Path,
) -> Result<ToolOutput> {
    use rayon::prelude::*;

    let files = walk_files(search_path, file_filter);

    // Parallel: process each file independently, then merge results
    let file_blocks: Vec<Vec<CodeBlock>> = files
        .par_iter()
        .filter_map(|entry| {
            let content = read_text_file(entry)?;

            // For AND queries, check file-level match first
            if !query.must.is_empty() && !query.matches_file(&content) {
                return None;
            }

            let match_lines = query.matching_lines(&content);
            if match_lines.is_empty() {
                return None;
            }

            let rel_path = entry.strip_prefix(cwd).unwrap_or(entry);

            if let Some(parser_blocks) = extract_blocks_at_lines(&content, entry, &match_lines) {
                let blocks: Vec<CodeBlock> = parser_blocks
                    .into_iter()
                    .map(|block| CodeBlock {
                        file: rel_path.to_path_buf(),
                        start_line: block.start_line,
                        end_line: block.end_line,
                        kind: block.kind,
                        code: block.code,
                    })
                    .collect();
                if blocks.is_empty() {
                    None
                } else {
                    Some(blocks)
                }
            } else {
                let lines: Vec<&str> = content.lines().collect();
                let blocks: Vec<CodeBlock> = match_lines
                    .iter()
                    .map(|&line_idx| {
                        let start = line_idx.saturating_sub(5);
                        let end = (line_idx + 6).min(lines.len());
                        CodeBlock {
                            file: rel_path.to_path_buf(),
                            start_line: start + 1,
                            end_line: end,
                            kind: None,
                            code: lines[start..end].join("\n"),
                        }
                    })
                    .collect();
                if blocks.is_empty() {
                    None
                } else {
                    Some(blocks)
                }
            }
        })
        .collect();

    // Merge and deduplicate
    let mut blocks = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for file_result in file_blocks {
        for block in file_result {
            if blocks.len() >= limit {
                break;
            }
            let key = (block.file.clone(), block.start_line, block.end_line);
            if seen.insert(key) {
                blocks.push(block);
            }
        }
        if blocks.len() >= limit {
            break;
        }
    }

    if blocks.is_empty() {
        return Ok(ToolOutput::text("No matches found."));
    }

    Ok(ToolOutput::text(truncate_text(format_blocks(&blocks))))
}

// ── extract mode ────────────────────────────────────────────────────

async fn execute_extract(targets: &[serde_json::Value], ctx: &ToolContext) -> Result<ToolOutput> {
    let mut blocks = Vec::new();

    for target in targets {
        let target_str = match target.as_str() {
            Some(s) => s,
            None => continue,
        };

        if let Some((file, locator)) = parse_extract_target(target_str) {
            let path = super::resolve_path(&ctx.cwd, &file);

            let content = match read_text_file(&path) {
                Some(c) => c,
                None => {
                    blocks.push(CodeBlock {
                        file: PathBuf::from(&file),
                        start_line: 0,
                        end_line: 0,
                        kind: None,
                        code: format!("Error: could not read {file}"),
                    });
                    continue;
                }
            };

            let rel_path = path.strip_prefix(&ctx.cwd).unwrap_or(&path);

            match locator {
                Locator::Line(line) => {
                    let line_idx = line.saturating_sub(1);
                    if let Some(extracted) = extract_blocks_at_lines(&content, &path, &[line_idx]) {
                        for mut block in extracted {
                            block.file = rel_path.to_path_buf();
                            blocks.push(block);
                        }
                    } else {
                        // No parser — return raw lines around the target
                        let lines: Vec<&str> = content.lines().collect();
                        let start = line_idx.saturating_sub(5);
                        let end = (line_idx + 6).min(lines.len());
                        blocks.push(CodeBlock {
                            file: rel_path.to_path_buf(),
                            start_line: start + 1,
                            end_line: end,
                            kind: None,
                            code: lines[start..end].join("\n"),
                        });
                    }
                }
                Locator::Range(start, end) => {
                    let lines: Vec<&str> = content.lines().collect();
                    let s = start.saturating_sub(1).min(lines.len());
                    let e = end.min(lines.len());
                    blocks.push(CodeBlock {
                        file: rel_path.to_path_buf(),
                        start_line: s + 1,
                        end_line: e,
                        kind: None,
                        code: lines[s..e].join("\n"),
                    });
                }
                Locator::Symbol(name) => {
                    if let Some(found) = extract_symbol(&content, &path, &name) {
                        blocks.push(CodeBlock {
                            file: rel_path.to_path_buf(),
                            ..found
                        });
                    } else {
                        blocks.push(CodeBlock {
                            file: rel_path.to_path_buf(),
                            start_line: 0,
                            end_line: 0,
                            kind: None,
                            code: format!("Symbol '{name}' not found in {file}"),
                        });
                    }
                }
            }
        }
    }

    if blocks.is_empty() {
        return Ok(ToolOutput::text("No code blocks found."));
    }

    Ok(ToolOutput::text(truncate_text(format_blocks(&blocks))))
}

// ── tree-sitter block extraction ────────────────────────────────────

struct CodeBlock {
    file: PathBuf,
    start_line: usize,
    end_line: usize,
    kind: Option<String>,
    code: String,
}

fn get_parser(path: &Path) -> Option<tree_sitter::Parser> {
    let ext = path.extension()?.to_str()?;
    let language = match ext {
        "rs" => tree_sitter_rust::LANGUAGE.into(),
        "ts" | "tsx" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "js" | "jsx" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "py" => tree_sitter_python::LANGUAGE.into(),
        "go" => tree_sitter_go::LANGUAGE.into(),
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
        code: lines[s..e].join("\n"),
    })
}

fn find_symbol_node<'a>(
    node: tree_sitter::Node<'a>,
    source: &str,
    name: &str,
) -> Option<tree_sitter::Node<'a>> {
    // Check if this node is a block kind and contains an identifier matching the name
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
        {
            let text = &source[child.byte_range()];
            if text == name {
                return true;
            }
        }
        // Don't recurse into nested blocks — only check direct children
        if BLOCK_KINDS.contains(&kind) {
            continue;
        }
        // Check one level deeper for patterns like `fn name(...)`
        let mut inner_cursor = child.walk();
        let inner_children: Vec<_> = child.children(&mut inner_cursor).collect();
        for inner in inner_children {
            let ik = inner.kind();
            if ik == "identifier" || ik == "type_identifier" || ik == "name" {
                let text = &source[inner.byte_range()];
                if text == name {
                    return true;
                }
            }
        }
    }
    false
}

// ── file walking ────────────────────────────────────────────────────

fn walk_files(search_path: &Path, filter: &FileFilter) -> Vec<PathBuf> {
    let mut builder = ignore::WalkBuilder::new(search_path);
    builder
        .hidden(true) // skip hidden files
        .git_ignore(true) // respect .gitignore
        .git_global(true)
        .git_exclude(true);

    if let Some(ref glob) = filter.glob {
        let mut overrides = ignore::overrides::OverrideBuilder::new(search_path);
        if overrides.add(glob).is_ok() {
            if let Ok(built) = overrides.build() {
                builder.overrides(built);
            }
        }
    }

    let mut files = Vec::new();
    for entry in builder.build().flatten() {
        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            let path = entry.into_path();
            if filter.accepts(&path) {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn read_text_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    // Skip binary files
    if bytes.contains(&0) {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

// ── target parsing ──────────────────────────────────────────────────

enum Locator {
    Line(usize),
    Range(usize, usize),
    Symbol(String),
}

fn parse_extract_target(target: &str) -> Option<(String, Locator)> {
    // file#symbol
    if let Some(hash_pos) = target.rfind('#') {
        let file = target[..hash_pos].to_string();
        let symbol = target[hash_pos + 1..].to_string();
        if !file.is_empty() && !symbol.is_empty() {
            return Some((file, Locator::Symbol(symbol)));
        }
    }

    // file:start-end or file:line
    if let Some(colon_pos) = target.rfind(':') {
        let file = target[..colon_pos].to_string();
        let suffix = &target[colon_pos + 1..];
        if !file.is_empty() && !suffix.is_empty() {
            if let Some(dash_pos) = suffix.find('-') {
                let start = suffix[..dash_pos].parse::<usize>().ok()?;
                let end = suffix[dash_pos + 1..].parse::<usize>().ok()?;
                return Some((file, Locator::Range(start, end)));
            } else if let Ok(line) = suffix.parse::<usize>() {
                return Some((file, Locator::Line(line)));
            }
        }
    }

    None
}

// ── formatting ──────────────────────────────────────────────────────

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

        let fence = fence_language(&block.file);
        sections.push(format!("{header}\n```{fence}\n{}\n```", block.code));
    }

    sections.join("\n\n")
}

fn fence_language(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("py") => "python",
        Some("go") => "go",
        Some("java") => "java",
        Some("rb") => "ruby",
        Some("c") | Some("h") => "c",
        Some("cpp") | Some("cc") | Some("hpp") => "cpp",
        Some("swift") => "swift",
        _ => "text",
    }
}

fn truncate_text(text: String) -> String {
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

// ── tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{ToolContext, ToolUpdate};
    use crate::ui::NullInterface;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    fn test_ctx(dir: &Path) -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel::<ToolUpdate>(64);
        ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(crate::tools::FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(crate::tools::FileTracker::new())),
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
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
            tmp.path().join("main.rs"),
            "fn helper() {\n    println!(\"hello\");\n}\n\npub fn main() {\n    helper();\n    println!(\"world\");\n}\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("lib.py"),
            "def greet(name):\n    print(f\"hello {name}\")\n\ndef farewell():\n    print(\"bye\")\n",
        )
        .unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        std::fs::write(tmp.path().join("sub/nested.txt"), "nested hello\n").unwrap();
        // Create .gitignore to exclude a file
        std::fs::write(tmp.path().join(".gitignore"), "ignored.txt\n").unwrap();
        std::fs::write(tmp.path().join("ignored.txt"), "should not appear\n").unwrap();
        tmp
    }

    fn run_async<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

    #[test]
    fn line_search_basic() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());
        let result = run_async(GrepTool.execute("1", json!({"pattern": "Hello"}), ctx)).unwrap();
        let text = result.text_content().unwrap();
        assert!(!result.is_error);
        assert!(text.contains("Hello World"));
        assert!(text.contains("Hello Again"));
    }

    #[test]
    fn line_search_case_insensitive() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());
        let result =
            run_async(GrepTool.execute("1", json!({"pattern": "hello", "ignoreCase": true}), ctx))
                .unwrap();
        let text = result.text_content().unwrap();
        assert!(text.contains("Hello World"));
    }

    #[test]
    fn line_search_glob_filter() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());
        let result = run_async(GrepTool.execute(
            "1",
            json!({"pattern": "hello", "glob": "*.rs", "ignoreCase": true}),
            ctx,
        ))
        .unwrap();
        let text = result.text_content().unwrap();
        assert!(text.contains("hello")); // matches in main.rs
        assert!(!text.contains("Hello World")); // not in .txt files
    }

    #[test]
    fn line_search_respects_gitignore() {
        let tmp = setup_test_dir();
        // Init git so .gitignore is respected
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .output()
            .ok();
        let ctx = test_ctx(tmp.path());
        let result =
            run_async(GrepTool.execute("1", json!({"pattern": "should not appear"}), ctx)).unwrap();
        let text = result.text_content().unwrap();
        assert!(text.contains("No matches"));
    }

    #[test]
    fn line_search_no_matches() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());
        let result =
            run_async(GrepTool.execute("1", json!({"pattern": "ZZZZNOTFOUND"}), ctx)).unwrap();
        let text = result.text_content().unwrap();
        assert!(text.contains("No matches"));
    }

    #[test]
    fn line_search_literal() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());
        // "." as regex matches everything; as literal, only actual dots
        let re_result = run_async(GrepTool.execute("1", json!({"pattern": "."}), ctx)).unwrap();
        let ctx2 = test_ctx(tmp.path());
        let lit_result =
            run_async(GrepTool.execute("1", json!({"pattern": ".", "literal": true}), ctx2))
                .unwrap();
        // Regex should match more than literal
        let re_text = re_result.text_content().unwrap();
        let lit_text = lit_result.text_content().unwrap();
        assert!(re_text.len() >= lit_text.len());
    }

    #[test]
    fn block_search_returns_functions() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());
        let result = run_async(GrepTool.execute(
            "1",
            json!({"pattern": "hello", "blocks": true, "glob": "*.rs"}),
            ctx,
        ))
        .unwrap();
        let text = result.text_content().unwrap();
        // Should return the complete helper() function
        assert!(text.contains("fn helper()"));
        assert!(text.contains("function_item"));
    }

    #[test]
    fn block_search_deduplicates() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());
        // "print" matches both lines in helper() — should only get one block
        let result = run_async(GrepTool.execute(
            "1",
            json!({"pattern": "println", "blocks": true, "glob": "*.rs"}),
            ctx,
        ))
        .unwrap();
        let text = result.text_content().unwrap();
        let block_count = text.matches("```rust").count();
        // helper() has println, main() has println — should get 2 blocks, not 3
        assert_eq!(block_count, 2, "Expected 2 blocks, got: {text}");
    }

    #[test]
    fn extract_by_line() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());
        let result =
            run_async(GrepTool.execute("1", json!({"extract": ["main.rs:2"]}), ctx)).unwrap();
        let text = result.text_content().unwrap();
        // Line 2 is inside helper() — should get the whole function
        assert!(text.contains("fn helper()"));
    }

    #[test]
    fn extract_by_symbol() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());
        let result =
            run_async(GrepTool.execute("1", json!({"extract": ["main.rs#main"]}), ctx)).unwrap();
        let text = result.text_content().unwrap();
        assert!(text.contains("pub fn main()"));
    }

    #[test]
    fn extract_by_range() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());
        let result =
            run_async(GrepTool.execute("1", json!({"extract": ["main.rs:1-3"]}), ctx)).unwrap();
        let text = result.text_content().unwrap();
        assert!(text.contains("fn helper()"));
        assert!(text.contains("println"));
    }

    #[test]
    fn extract_python_symbol() {
        let tmp = setup_test_dir();
        let ctx = test_ctx(tmp.path());
        let result =
            run_async(GrepTool.execute("1", json!({"extract": ["lib.py#greet"]}), ctx)).unwrap();
        let text = result.text_content().unwrap();
        assert!(text.contains("def greet"));
    }

    #[test]
    fn parse_target_line() {
        let (file, loc) = parse_extract_target("src/main.rs:42").unwrap();
        assert_eq!(file, "src/main.rs");
        assert!(matches!(loc, Locator::Line(42)));
    }

    #[test]
    fn parse_target_range() {
        let (file, loc) = parse_extract_target("src/main.rs:10-50").unwrap();
        assert_eq!(file, "src/main.rs");
        assert!(matches!(loc, Locator::Range(10, 50)));
    }

    #[test]
    fn parse_target_symbol() {
        let (file, loc) = parse_extract_target("src/main.rs#authenticate").unwrap();
        assert_eq!(file, "src/main.rs");
        assert!(matches!(loc, Locator::Symbol(ref s) if s == "authenticate"));
    }
}
