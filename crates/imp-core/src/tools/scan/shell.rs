//! Shell tree-sitter extraction — functions.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_bash::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    extract_shell(&tree.root_node(), source, file, result);
}

fn extract_shell(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    if node.kind() == "function_definition" {
        extract_function(node, source, file, result);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_shell(&child, source, file, result);
    }
}

fn extract_function(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let Some(name) = child_text_by_field(node, "name", source).or_else(|| first_word(node, source))
    else {
        return;
    };
    result
        .functions
        .entry(name.clone())
        .or_insert(FunctionInfo {
            name,
            source: source_loc(file, node),
            signature: first_line(node_text(node, source)),
            visibility: Visibility::Public,
            ..Default::default()
        });
}

fn child_text_by_field(node: &Node, field: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field)
        .map(|child| node_text(&child, source).trim().to_string())
        .filter(|text| !text.is_empty())
}

fn first_word(node: &Node, source: &str) -> Option<String> {
    if matches!(node.kind(), "word" | "identifier") {
        return Some(node_text(node, source).to_string());
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = first_word(&child, source) {
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
