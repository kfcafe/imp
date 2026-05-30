//! Java tree-sitter extraction — classes, interfaces, enums, methods, and constructors.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    extract_java(&tree.root_node(), source, file, None, result);
}

fn extract_java(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    match node.kind() {
        "class_declaration" | "interface_declaration" | "enum_declaration" => {
            extract_type(node, source, file, owner, result);
            return;
        }
        "method_declaration" | "constructor_declaration" => {
            extract_function(node, source, file, owner, result);
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_java(&child, source, file, owner, result);
    }
}

fn extract_type(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    let Some(name) = child_text_by_field(node, "name", source) else {
        return;
    };
    let qualified = qualify(owner, &name);
    let kind = match node.kind() {
        "interface_declaration" => TypeKind::Interface,
        "enum_declaration" => TypeKind::Enum,
        _ => TypeKind::Class,
    };
    let variants = if kind == TypeKind::Enum {
        enum_variants(node, source)
    } else {
        Vec::new()
    };

    result.types.insert(
        qualified.clone(),
        TypeInfo {
            name: qualified.clone(),
            source: source_loc(file, node),
            kind,
            variants,
            visibility: visibility(node, source),
            implements: heritage(node, source),
            ..Default::default()
        },
    );

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_java(&child, source, file, Some(&qualified), result);
    }
}

fn extract_function(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    let Some(name) = child_text_by_field(node, "name", source) else {
        return;
    };
    let qualified = qualify(owner, &name);

    result.functions.insert(
        qualified,
        FunctionInfo {
            name: name.clone(),
            source: source_loc(file, node),
            signature: first_line(node_text(node, source)),
            visibility: visibility(node, source),
            is_async: false,
            is_test: has_annotation(node, source, "Test"),
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

fn enum_variants(node: &Node, source: &str) -> Vec<String> {
    collect_texts(node, source, &["enum_constant"])
}

fn heritage(node: &Node, source: &str) -> Vec<String> {
    collect_texts(
        node,
        source,
        &[
            "superclass",
            "super_interfaces",
            "extends_interfaces",
            "type_identifier",
        ],
    )
    .into_iter()
    .filter(|item| {
        item != child_text_by_field(node, "name", source)
            .as_deref()
            .unwrap_or("")
    })
    .collect()
}

fn collect_texts(node: &Node, source: &str, kinds: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    collect_texts_inner(node, source, kinds, &mut out);
    out.sort();
    out.dedup();
    out
}

fn collect_texts_inner(node: &Node, source: &str, kinds: &[&str], out: &mut Vec<String>) {
    if kinds.contains(&node.kind()) {
        let text = node_text(node, source).trim();
        if !text.is_empty() {
            out.push(text.to_string());
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_texts_inner(&child, source, kinds, out);
    }
}

fn child_text_by_field(node: &Node, field: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field)
        .map(|child| node_text(&child, source).trim().to_string())
        .filter(|text| !text.is_empty())
}

fn visibility(node: &Node, source: &str) -> Visibility {
    let modifiers = modifier_text(node, source);
    if modifiers.contains("private") {
        Visibility::Private
    } else if modifiers.contains("public") {
        Visibility::Public
    } else {
        Visibility::Internal
    }
}

fn modifier_text(node: &Node, source: &str) -> String {
    let mut cursor = node.walk();
    let modifiers = node
        .named_children(&mut cursor)
        .find(|child| matches!(child.kind(), "modifiers" | "modifier"))
        .map(|child| node_text(&child, source).to_string())
        .unwrap_or_default();
    modifiers
}

fn has_annotation(node: &Node, source: &str, name: &str) -> bool {
    node_text(node, source).contains(&format!("@{name}"))
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
