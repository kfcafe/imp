//! Generic tree-sitter extraction for languages without a dedicated adapter.
//!
//! This intentionally extracts only broad, stable symbols: named type-like
//! declarations and function-like declarations. Language-specific adapters can
//! replace this when richer fields, variants, visibility, or signatures matter.

use tree_sitter::{Language, Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, language: Language, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    extract_node(&tree.root_node(), source, file, None, result);
}

fn extract_node(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    if extract_elixir_call(node, source, file, result)
        || extract_ocaml_module(node, source, file, owner, result)
        || extract_zig_variable_type(node, source, file, result)
    {
        // Keep walking below so nested function-like declarations are still indexed.
    }

    match node.kind() {
        "class" | "class_declaration" | "class_definition" | "class_specifier" => {
            extract_type(node, source, file, TypeKind::Class, result);
        }
        "interface_declaration" => {
            extract_type(node, source, file, TypeKind::Interface, result);
        }
        "struct_declaration" | "struct_specifier" | "struct_item" => {
            extract_type(node, source, file, TypeKind::Struct, result);
        }
        "enum_declaration" | "enum_specifier" | "enum_item" => {
            extract_type(node, source, file, TypeKind::Enum, result);
        }
        "protocol_declaration" => {
            extract_type(node, source, file, TypeKind::Protocol, result);
        }
        "type_alias_declaration" | "type_declaration" => {
            extract_type(node, source, file, TypeKind::TypeAlias, result);
        }
        "function_declaration"
        | "function_definition"
        | "method"
        | "function_item"
        | "method_declaration"
        | "method_definition"
        | "procedure_declaration"
        | "value_definition"
        | "let_binding"
        | "constructor_declaration" => {
            extract_function(node, source, file, owner, result);
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let child_owner = ocaml_module_name(node, source).or(owner.map(str::to_string));
        extract_node(&child, source, file, child_owner.as_deref(), result);
    }
}

fn extract_elixir_call(node: &Node, source: &str, file: &str, result: &mut ScanResult) -> bool {
    if node.kind() != "call" {
        return false;
    }
    let Some(callee) = first_named_child(node) else {
        return false;
    };
    if callee.kind() != "identifier" {
        return false;
    }

    match node_text(&callee, source).as_str() {
        "defmodule" => {
            let Some(name) = first_identifier_in_kind(node, source, "alias") else {
                return false;
            };
            result.types.entry(name.clone()).or_insert(TypeInfo {
                name,
                source: source_loc(file, node),
                kind: TypeKind::Class,
                visibility: Visibility::Private,
                ..Default::default()
            });
            true
        }
        "def" | "defp" => {
            let Some(name) = elixir_def_name(node, source) else {
                return false;
            };
            result
                .functions
                .entry(name.clone())
                .or_insert(FunctionInfo {
                    name,
                    source: source_loc(file, node),
                    signature: signature_line(node, source),
                    visibility: Visibility::Private,
                    ..Default::default()
                });
            true
        }
        _ => false,
    }
}

fn elixir_def_name(node: &Node, source: &str) -> Option<String> {
    let arguments = named_child_kind(node, "arguments")?;
    let call = named_child_kind(&arguments, "call")?;
    let identifier = first_named_child(&call)?;
    if identifier.kind() == "identifier" {
        Some(node_text(&identifier, source))
    } else {
        None
    }
}

fn extract_ocaml_module(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) -> bool {
    if node.kind() != "module_definition" {
        return false;
    }
    let Some(name) = first_identifier_in_kind(node, source, "module_name") else {
        return false;
    };
    let qualified = qualify(owner, &name);
    result.types.entry(qualified.clone()).or_insert(TypeInfo {
        name: qualified,
        source: source_loc(file, node),
        kind: TypeKind::Class,
        visibility: Visibility::Private,
        ..Default::default()
    });
    true
}

fn extract_zig_variable_type(
    node: &Node,
    source: &str,
    file: &str,
    result: &mut ScanResult,
) -> bool {
    if node.kind() != "variable_declaration" {
        return false;
    }
    let has_type_value = named_child_kind(node, "struct_declaration").is_some()
        || named_child_kind(node, "enum_declaration").is_some();
    if !has_type_value {
        return false;
    }
    let Some(name) = first_identifier_in_kind(node, source, "identifier") else {
        return false;
    };
    result.types.entry(name.clone()).or_insert(TypeInfo {
        name,
        source: source_loc(file, node),
        kind: TypeKind::Struct,
        visibility: Visibility::Private,
        ..Default::default()
    });
    true
}

fn extract_type(node: &Node, source: &str, file: &str, kind: TypeKind, result: &mut ScanResult) {
    let Some(name) = node_name(node, source) else {
        return;
    };

    result.types.entry(name.clone()).or_insert(TypeInfo {
        name,
        source: source_loc(file, node),
        kind,
        visibility: Visibility::Private,
        ..Default::default()
    });
}

fn extract_function(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    let Some(name) = node_name(node, source) else {
        return;
    };
    let signature = signature_line(node, source);

    let qualified = qualify(owner, &name);

    result
        .functions
        .entry(qualified.clone())
        .or_insert(FunctionInfo {
            name: qualified,
            source: source_loc(file, node),
            signature,
            visibility: Visibility::Private,
            is_async: has_child_kind(node, "async") || has_child_kind(node, "async_modifier"),
            ..Default::default()
        });
}

fn node_name(node: &Node, source: &str) -> Option<String> {
    for field in ["name", "declarator", "declaration"] {
        if let Some(child) = node.child_by_field_name(field) {
            if let Some(name) = identifier_text(&child, source) {
                return Some(name);
            }
        }
    }
    identifier_text(node, source)
}

fn ocaml_module_name(node: &Node, source: &str) -> Option<String> {
    if node.kind() == "module_definition" {
        first_identifier_in_kind(node, source, "module_name")
    } else {
        None
    }
}

fn qualify(owner: Option<&str>, name: &str) -> String {
    owner
        .map(|owner| format!("{owner}::{name}"))
        .unwrap_or_else(|| name.to_string())
}

fn first_named_child<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let child = node.named_children(&mut cursor).next();
    child
}

fn named_child_kind<'a>(node: &Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let child = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == kind);
    child
}

fn first_identifier_in_kind(node: &Node, source: &str, kind: &str) -> Option<String> {
    if node.kind() == kind {
        return Some(node_text(node, source));
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = first_identifier_in_kind(&child, source, kind) {
            return Some(name);
        }
    }
    None
}

fn identifier_text(node: &Node, source: &str) -> Option<String> {
    if is_identifier_kind(node.kind()) {
        return Some(node_text(node, source));
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = identifier_text(&child, source) {
            return Some(name);
        }
    }
    None
}

fn is_identifier_kind(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "word"
            | "package_name"
            | "alias"
            | "type_identifier"
            | "field_identifier"
            | "property_identifier"
            | "value_name"
            | "value_pattern"
            | "constant"
            | "simple_identifier"
            | "variable_identifier"
            | "name"
    )
}

fn has_child_kind(node: &Node, kind: &str) -> bool {
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).any(|child| child.kind() == kind);
    found
}

fn signature_line(node: &Node, source: &str) -> String {
    node_text(node, source)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn node_text(node: &Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

fn source_loc(file: &str, node: &Node) -> String {
    format!("{}:{}", file, node.start_position().row + 1)
}
