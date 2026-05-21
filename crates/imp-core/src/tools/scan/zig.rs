//! Zig tree-sitter extraction — const type declarations and functions.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_zig::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    extract_zig(&tree.root_node(), source, file, result);
}

fn extract_zig(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    match node.kind() {
        "variable_declaration" => extract_variable_type(node, source, file, result),
        "function_declaration" | "fn_proto" => extract_function(node, source, file, result),
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_zig(&child, source, file, result);
    }
}

fn extract_variable_type(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let text = node_text(node, source);
    let kind = if text.contains("struct") {
        TypeKind::Struct
    } else if text.contains("enum") {
        TypeKind::Enum
    } else {
        return;
    };
    let Some(name) = first_identifier(node, source) else {
        return;
    };
    result.types.entry(name.clone()).or_insert(TypeInfo {
        name,
        source: source_loc(file, node),
        kind,
        visibility: if text.trim_start().starts_with("pub ") {
            Visibility::Public
        } else {
            Visibility::Private
        },
        ..Default::default()
    });
}

fn extract_function(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let Some(name) =
        child_text_by_field(node, "name", source).or_else(|| first_identifier(node, source))
    else {
        return;
    };
    let text = node_text(node, source);
    result
        .functions
        .entry(name.clone())
        .or_insert(FunctionInfo {
            name,
            source: source_loc(file, node),
            signature: first_line(text),
            visibility: if text.trim_start().starts_with("pub ") {
                Visibility::Public
            } else {
                Visibility::Private
            },
            ..Default::default()
        });
}

fn child_text_by_field(node: &Node, field: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field)
        .map(|child| node_text(&child, source).trim().to_string())
        .filter(|text| !text.is_empty())
}

fn first_identifier(node: &Node, source: &str) -> Option<String> {
    if matches!(node.kind(), "identifier" | "IDENTIFIER") {
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
