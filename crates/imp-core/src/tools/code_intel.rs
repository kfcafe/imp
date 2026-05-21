use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::json;

const BLOCK_KINDS: &[&str] = &[
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
    "function_definition",
    "class_definition",
    "decorated_definition",
    "object_declaration",
    "property_declaration",
    "method_declaration",
    "type_declaration",
    "type_spec",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    pub file: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub kind: Option<String>,
    pub symbol: Option<String>,
    pub language: Option<String>,
    pub truncated: bool,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    pub start_line: usize,
    pub end_line: usize,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxValidation {
    pub language: Option<&'static str>,
    pub supported: bool,
    pub valid: bool,
    pub errors: Vec<SyntaxError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolDiff {
    pub before: BTreeSet<String>,
    pub after: BTreeSet<String>,
    pub added: BTreeSet<String>,
    pub removed: BTreeSet<String>,
}

pub fn parser_for_path(path: &Path) -> Option<tree_sitter::Parser> {
    let language = language_for_path_tree_sitter(path)?;
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).ok()?;
    Some(parser)
}

pub fn language_for_path(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|e| e.to_str())? {
        "sh" | "bash" | "zsh" | "fish" => Some("shell"),
        "py" | "pyw" => Some("python"),
        "rs" => Some("rust"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        "ts" | "tsx" => Some("typescript"),
        "go" => Some("go"),
        "ex" | "exs" => Some("elixir"),
        "rb" => Some("ruby"),
        "pl" | "pm" | "t" => Some("perl"),
        "lua" | "luau" => Some("lua"),
        "ml" | "mli" => Some("ocaml"),
        "zig" | "zon" => Some("zig"),
        "odin" => Some("odin"),
        "swift" => Some("swift"),
        "kt" | "kts" => Some("kotlin"),
        "java" => Some("java"),
        "c" | "h" => Some("c"),
        "cs" => Some("csharp"),
        "cc" | "cpp" | "cxx" | "c++" | "hpp" | "hh" | "hxx" | "h++" => Some("cpp"),
        "php" => Some("php"),
        "scala" | "sc" => Some("scala"),
        "dart" => Some("dart"),
        _ => None,
    }
}

fn language_for_path_tree_sitter(path: &Path) -> Option<tree_sitter::Language> {
    match path.extension()?.to_str()? {
        "sh" | "bash" | "zsh" | "fish" => Some(tree_sitter_bash::LANGUAGE.into()),
        "py" | "pyw" => Some(tree_sitter_python::LANGUAGE.into()),
        "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "js" | "jsx" | "mjs" | "cjs" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "ts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "go" => Some(tree_sitter_go::LANGUAGE.into()),
        "ex" | "exs" => Some(tree_sitter_elixir::LANGUAGE.into()),
        "rb" => Some(tree_sitter_ruby::LANGUAGE.into()),
        "pl" | "pm" | "t" => Some(tree_sitter_perl::LANGUAGE.into()),
        "lua" | "luau" => Some(tree_sitter_lua::LANGUAGE.into()),
        "ml" | "mli" => Some(tree_sitter_ocaml::LANGUAGE_OCAML.into()),
        "zig" | "zon" => Some(tree_sitter_zig::LANGUAGE.into()),
        "odin" => Some(tree_sitter_odin::LANGUAGE.into()),
        "swift" => Some(tree_sitter_swift::LANGUAGE.into()),
        "kt" | "kts" => Some(tree_sitter_kotlin_ng::LANGUAGE.into()),
        "java" => Some(tree_sitter_java::LANGUAGE.into()),
        "c" | "h" => Some(tree_sitter_c::LANGUAGE.into()),
        "cs" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        "cc" | "cpp" | "cxx" | "c++" | "hpp" | "hh" | "hxx" | "h++" => {
            Some(tree_sitter_cpp::LANGUAGE.into())
        }
        "php" => Some(tree_sitter_php::LANGUAGE_PHP.into()),
        "scala" | "sc" => Some(tree_sitter_scala::LANGUAGE.into()),
        "dart" => Some(tree_sitter_dart::LANGUAGE.into()),
        _ => None,
    }
}

pub fn validate_syntax(source: &str, path: &Path) -> SyntaxValidation {
    let language = language_for_path(path);
    let Some(mut parser) = parser_for_path(path) else {
        return SyntaxValidation {
            language,
            supported: false,
            valid: true,
            errors: Vec::new(),
        };
    };
    let Some(tree) = parser.parse(source, None) else {
        return SyntaxValidation {
            language,
            supported: true,
            valid: false,
            errors: vec![SyntaxError {
                start_line: 1,
                end_line: 1,
                kind: "parse_failed".into(),
            }],
        };
    };
    let mut errors = Vec::new();
    collect_error_nodes(tree.root_node(), &mut errors);
    SyntaxValidation {
        language,
        supported: true,
        valid: errors.is_empty(),
        errors,
    }
}

fn collect_error_nodes(node: tree_sitter::Node, errors: &mut Vec<SyntaxError>) {
    if node.is_error() || node.is_missing() {
        errors.push(SyntaxError {
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            kind: node.kind().to_string(),
        });
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_error_nodes(child, errors);
    }
}

pub fn extract_blocks_at_lines(
    source: &str,
    path: &Path,
    match_lines: &[usize],
) -> Option<Vec<CodeBlock>> {
    let mut parser = parser_for_path(path)?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();
    let lines: Vec<&str> = source.lines().collect();
    let mut blocks = Vec::new();
    let mut seen_ranges = std::collections::HashSet::new();
    for &line_idx in match_lines {
        if let Some(node) = find_enclosing_node(root, line_idx) {
            let start = node.start_position().row;
            let end = node.end_position().row;
            if seen_ranges.insert((start, end)) {
                let s = start.min(lines.len());
                let e = (end + 1).min(lines.len());
                blocks.push(CodeBlock {
                    file: PathBuf::new(),
                    start_line: start + 1,
                    end_line: end + 1,
                    kind: Some(node.kind().to_string()),
                    symbol: node_name(node, source),
                    language: language_for_path(path).map(str::to_string),
                    truncated: false,
                    code: lines[s..e].join("\n"),
                });
            }
        }
    }
    Some(blocks)
}

pub fn extract_symbol(source: &str, path: &Path, name: &str) -> Option<CodeBlock> {
    let mut parser = parser_for_path(path)?;
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

pub fn top_level_symbols(source: &str, path: &Path) -> BTreeSet<String> {
    let mut symbols = BTreeSet::new();
    let Some(mut parser) = parser_for_path(path) else {
        return symbols;
    };
    let Some(tree) = parser.parse(source, None) else {
        return symbols;
    };
    let root = tree.root_node();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if BLOCK_KINDS.contains(&child.kind()) {
            if let Some(name) = node_name(child, source) {
                symbols.insert(name);
            }
        }
    }
    symbols
}

pub fn diff_top_level_symbols(before_source: &str, after_source: &str, path: &Path) -> SymbolDiff {
    let before = top_level_symbols(before_source, path);
    let after = top_level_symbols(after_source, path);
    let added = after.difference(&before).cloned().collect();
    let removed = before.difference(&after).cloned().collect();
    SymbolDiff {
        before,
        after,
        added,
        removed,
    }
}

pub fn block_details(block: &CodeBlock) -> serde_json::Value {
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

pub fn format_blocks(blocks: &[CodeBlock]) -> String {
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
        let fence = language_for_path(&block.file).unwrap_or("text");
        sections.push(format!(
            "{header}\nDetails: {details}\n```{fence}\n{}\n```",
            block.code
        ));
    }
    sections.join("\n\n")
}

fn find_enclosing_node(root: tree_sitter::Node, target_line: usize) -> Option<tree_sitter::Node> {
    let mut best = None;
    find_enclosing_node_recursive(root, target_line, &mut best);
    best
}

fn find_enclosing_node_recursive<'a>(
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
    for child in node.children(&mut cursor) {
        find_enclosing_node_recursive(child, target_line, best);
    }
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
    for child in node.children(&mut cursor) {
        if let Some(found) = find_symbol_node(child, source, name) {
            return Some(found);
        }
    }
    None
}

fn node_name(node: tree_sitter::Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if is_name_node(child.kind()) {
            return Some(source[child.byte_range()].to_string());
        }
        if BLOCK_KINDS.contains(&child.kind()) {
            continue;
        }
        let mut inner_cursor = child.walk();
        for inner in child.children(&mut inner_cursor) {
            if is_name_node(inner.kind()) {
                return Some(source[inner.byte_range()].to_string());
            }
        }
    }
    None
}

fn node_has_name(node: tree_sitter::Node, source: &str, name: &str) -> bool {
    node_name(node, source).is_some_and(|found| found == name)
}

fn is_name_node(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "type_identifier"
            | "name"
            | "property_identifier"
            | "simple_identifier"
            | "variable_identifier"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_symbol_and_enclosing_block() {
        let source =
            "struct User { id: u64 }\n\nfn greet(name: &str) {\n    println!(\"hi {name}\");\n}\n";
        let path = Path::new("lib.rs");
        let symbol = extract_symbol(source, path, "greet").expect("symbol");
        assert_eq!(symbol.start_line, 3);
        assert_eq!(symbol.symbol.as_deref(), Some("greet"));
        let blocks = extract_blocks_at_lines(source, path, &[3]).expect("blocks");
        assert_eq!(blocks[0].symbol.as_deref(), Some("greet"));
    }

    #[test]
    fn validates_syntax_and_diffs_symbols() {
        let path = Path::new("lib.rs");
        let before = "fn old() {}\n";
        let after = "fn new() {}\n";
        let diff = diff_top_level_symbols(before, after, path);
        assert!(diff.added.contains("new"));
        assert!(diff.removed.contains("old"));
        assert!(validate_syntax(after, path).valid);
        assert!(!validate_syntax("fn broken( {", path).valid);
    }
}
