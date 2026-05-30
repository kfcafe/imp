//! Odin tree-sitter extraction — struct declarations and procedures.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_odin::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    extract_odin(&tree.root_node(), source, file, result);
}

fn extract_odin(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    if node.kind() == "constant_declaration"
        || node.kind() == "variable_declaration"
        || node.kind() == "declaration"
    {
        let text = node_text(node, source);
        if text.contains("struct") || text.contains("enum") {
            extract_type(node, source, file, result);
        } else if text.contains("proc") {
            extract_proc(node, source, file, result);
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_odin(&child, source, file, result);
    }
}

fn extract_type(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let text = node_text(node, source);
    let Some(name) = name_before_colon_colon(text).or_else(|| first_identifier(node, source))
    else {
        return;
    };
    result.types.entry(name.clone()).or_insert(TypeInfo {
        name,
        source: source_loc(file, node),
        kind: if text.contains("enum") {
            TypeKind::Enum
        } else {
            TypeKind::Struct
        },
        visibility: Visibility::Public,
        ..Default::default()
    });
}

fn extract_proc(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let text = node_text(node, source);
    let Some(name) = name_before_colon_colon(text).or_else(|| first_identifier(node, source))
    else {
        return;
    };
    result
        .functions
        .entry(name.clone())
        .or_insert(FunctionInfo {
            name,
            source: source_loc(file, node),
            signature: first_line(text),
            visibility: Visibility::Public,
            ..Default::default()
        });
}

fn name_before_colon_colon(text: &str) -> Option<String> {
    text.split("::")
        .next()?
        .split_whitespace()
        .last()
        .map(str::to_string)
        .filter(|name| !name.is_empty())
}

fn first_identifier(node: &Node, source: &str) -> Option<String> {
    if node.kind() == "identifier" {
        return Some(node_text(node, source).to_string());
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = first_identifier(&child, source) {
            return Some(name);
        }
    }
    None
}

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().trim().to_string()
}
fn node_text<'a>(node: &Node, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}
fn source_loc(file: &str, node: &Node) -> String {
    format!("{}:{}", file, node.start_position().row + 1)
}
