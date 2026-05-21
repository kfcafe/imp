//! Elixir tree-sitter extraction — modules and def/defp functions.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_elixir::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    extract_elixir(&tree.root_node(), source, file, None, result);
}

fn extract_elixir(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    if node.kind() == "call" {
        match callee(node, source).as_deref() {
            Some("defmodule") => {
                extract_module(node, source, file, owner, result);
                return;
            }
            Some("def") | Some("defp") => extract_function(node, source, file, owner, result),
            _ => {}
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_elixir(&child, source, file, owner, result);
    }
}

fn extract_module(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    let Some(name) = first_kind(node, source, "alias") else {
        return;
    };
    let qualified = qualify(owner, &name);
    result.types.insert(
        qualified.clone(),
        TypeInfo {
            name: qualified.clone(),
            source: source_loc(file, node),
            kind: TypeKind::Class,
            visibility: Visibility::Public,
            ..Default::default()
        },
    );

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_elixir(&child, source, file, Some(&qualified), result);
    }
}

fn extract_function(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    let Some(name) = def_name(node, source) else {
        return;
    };
    let qualified = qualify(owner, &name);
    result.functions.insert(
        qualified,
        FunctionInfo {
            name: name.clone(),
            source: source_loc(file, node),
            signature: first_line(node_text(node, source)),
            visibility: if callee(node, source).as_deref() == Some("def") {
                Visibility::Public
            } else {
                Visibility::Private
            },
            ..Default::default()
        },
    );
    if let Some(owner) = owner {
        if let Some(typedef) = result.types.get_mut(owner) {
            if !typedef.methods.contains(&name) {
                typedef.methods.push(name);
            }
        }
    }
}

fn def_name(node: &Node, source: &str) -> Option<String> {
    let arguments = first_child_kind(node, "arguments")?;
    let call = first_child_kind(&arguments, "call")?;
    callee(&call, source)
}

fn callee(node: &Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let child = node.named_children(&mut cursor).next()?;
    if child.kind() == "identifier" {
        Some(node_text(&child, source).to_string())
    } else {
        None
    }
}

fn first_child_kind<'a>(node: &Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
    }
    None
}

fn first_kind(node: &Node, source: &str, kind: &str) -> Option<String> {
    if node.kind() == kind {
        return Some(node_text(node, source).to_string());
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = first_kind(&child, source, kind) {
            return Some(name);
        }
    }
    None
}

fn qualify(owner: Option<&str>, name: &str) -> String {
    owner
        .map(|owner| format!("{owner}::{name}"))
        .unwrap_or_else(|| name.to_string())
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
