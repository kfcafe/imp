//! OCaml tree-sitter extraction — modules, type declarations, and let bindings.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_ocaml::LANGUAGE_OCAML.into())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    extract_ocaml(&tree.root_node(), source, file, None, result);
}

fn extract_ocaml(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    match node.kind() {
        "module_definition" => {
            extract_module(node, source, file, owner, result);
            return;
        }
        "type_definition" => extract_type(node, source, file, owner, result),
        "value_definition" | "let_binding" => extract_let(node, source, file, owner, result),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_ocaml(&child, source, file, owner, result);
    }
}

fn extract_module(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    let Some(name) = first_kind(
        node,
        source,
        &["module_name", "module_identifier", "capitalized_identifier"],
    ) else {
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

    let body_owner = Some(qualified.as_str());
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "module_binding" => {
                let mut binding_cursor = child.walk();
                for binding_child in child.named_children(&mut binding_cursor) {
                    extract_ocaml(&binding_child, source, file, body_owner, result);
                }
            }
            _ => extract_ocaml(&child, source, file, body_owner, result),
        }
    }
}

fn extract_type(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    let Some(name) = first_kind(
        node,
        source,
        &["type_constructor", "type_identifier", "value_name"],
    ) else {
        return;
    };
    let qualified = qualify(owner, &name);
    result.types.entry(qualified.clone()).or_insert(TypeInfo {
        name: qualified,
        source: source_loc(file, node),
        kind: TypeKind::TypeAlias,
        visibility: Visibility::Public,
        ..Default::default()
    });
}

fn extract_let(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    let Some(name) = first_kind(
        node,
        source,
        &[
            "value_name",
            "identifier",
            "lowercase_identifier",
            "value_pattern",
        ],
    ) else {
        return;
    };
    let qualified = qualify(owner, &name);
    result.functions.insert(
        qualified,
        FunctionInfo {
            name: name.clone(),
            source: source_loc(file, node),
            signature: first_line(node_text(node, source)),
            visibility: Visibility::Public,
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

fn first_kind(node: &Node, source: &str, kinds: &[&str]) -> Option<String> {
    if kinds.contains(&node.kind()) {
        return Some(node_text(node, source).to_string());
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = first_kind(&child, source, kinds) {
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
