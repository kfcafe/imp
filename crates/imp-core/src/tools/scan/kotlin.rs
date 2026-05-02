//! Kotlin tree-sitter extraction — classes, objects, interfaces, enums, functions, and properties.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_kotlin_ng::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return,
    };
    extract_kotlin(&tree.root_node(), source, file, result);
}

fn extract_kotlin(root: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        walk_declaration(&child, source, file, None, result);
    }
}

fn walk_declaration(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    match node.kind() {
        "class_declaration" | "object_declaration" => {
            extract_type(node, source, file, owner, result);
        }
        "function_declaration" => extract_function(node, source, file, owner, result),
        "property_declaration" => extract_property(node, source, file, owner, result),
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_declaration(&child, source, file, owner, result);
            }
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
    let Some(name_node) = find_child_kind(node, "type_identifier", source) else {
        return;
    };
    let name = node_text(&name_node, source).to_string();
    let qualified = qualify(owner, &name);
    let kind = type_kind(node, source);
    let visibility = visibility(node, source);
    let fields = extract_constructor_fields(node, source);
    let implements = extract_delegation_specifiers(node, source);

    result.types.insert(
        qualified.clone(),
        TypeInfo {
            name: qualified.clone(),
            source: source_loc(file, node),
            kind,
            fields,
            visibility,
            implements,
            ..Default::default()
        },
    );

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_declaration(&child, source, file, Some(&qualified), result);
    }
}

fn extract_function(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    let Some(name_node) = find_child_kind(node, "simple_identifier", source) else {
        return;
    };
    let name = node_text(&name_node, source).to_string();
    let qualified = qualify(owner, &name);
    let signature = first_line(node_text(node, source));
    let is_test = has_test_annotation(node, source) || name.starts_with("test");

    result.functions.insert(
        qualified,
        FunctionInfo {
            name: name.clone(),
            source: source_loc(file, node),
            signature,
            visibility: visibility(node, source),
            is_async: node_text(node, source).contains("suspend"),
            is_test,
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

fn extract_property(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    let Some(name_node) = find_child_kind(node, "variable_identifier", source) else {
        return;
    };
    let name = node_text(&name_node, source).to_string();

    if let Some(owner) = owner {
        if let Some(typedef) = result.types.get_mut(owner) {
            if !typedef.fields.iter().any(|field| field.name == name) {
                typedef.fields.push(Field {
                    name,
                    type_name: find_type_text(node, source).unwrap_or_default(),
                    optional: node_text(node, source).contains('?'),
                });
            }
        }
    } else {
        let qualified = qualify(None, &name);
        result.functions.insert(
            qualified,
            FunctionInfo {
                name,
                source: source_loc(file, node),
                signature: first_line(node_text(node, source)),
                visibility: visibility(node, source),
                is_async: false,
                is_test: false,
            },
        );
    }
}

fn extract_constructor_fields(node: &Node, source: &str) -> Vec<Field> {
    let mut fields = Vec::new();
    collect_constructor_fields(node, source, &mut fields);
    fields
}

fn collect_constructor_fields(node: &Node, source: &str, fields: &mut Vec<Field>) {
    if matches!(node.kind(), "class_parameter" | "parameter") {
        let text = node_text(node, source);
        if text.contains("val ") || text.contains("var ") {
            if let Some(name_node) = find_child_kind(node, "simple_identifier", source) {
                fields.push(Field {
                    name: node_text(&name_node, source).to_string(),
                    type_name: find_type_text(node, source).unwrap_or_default(),
                    optional: text.contains('?'),
                });
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_constructor_fields(&child, source, fields);
    }
}

fn extract_delegation_specifiers(node: &Node, source: &str) -> Vec<String> {
    let mut implements = Vec::new();
    collect_delegation_specifiers(node, source, &mut implements);
    implements
}

fn collect_delegation_specifiers(node: &Node, source: &str, implements: &mut Vec<String>) {
    if node.kind() == "delegation_specifier" {
        let text = node_text(node, source).trim();
        if !text.is_empty() {
            implements.push(text.to_string());
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_delegation_specifiers(&child, source, implements);
    }
}

fn type_kind(node: &Node, source: &str) -> TypeKind {
    let text = node_text(node, source);
    if text.trim_start().starts_with("interface ") || text.contains(" interface ") {
        TypeKind::Interface
    } else if text.trim_start().starts_with("enum class ") || text.contains(" enum class ") {
        TypeKind::Enum
    } else {
        TypeKind::Class
    }
}

fn visibility(node: &Node, source: &str) -> Visibility {
    let text = node_text(node, source);
    if text.contains("private ") {
        Visibility::Private
    } else if text.contains("internal ") {
        Visibility::Internal
    } else {
        Visibility::Public
    }
}

fn find_type_text(node: &Node, source: &str) -> Option<String> {
    find_child_kind(node, "type", source).map(|node| node_text(&node, source).trim().to_string())
}

fn find_child_kind<'a>(node: &Node<'a>, kind: &str, source: &str) -> Option<Node<'a>> {
    if node.kind() == kind && !node_text(node, source).trim().is_empty() {
        return Some(*node);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(found) = find_child_kind(&child, kind, source) {
            return Some(found);
        }
    }
    None
}

fn has_test_annotation(node: &Node, source: &str) -> bool {
    let text = node_text(node, source);
    text.contains("@Test") || text.contains("@ParameterizedTest")
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
