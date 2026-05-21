//! C++ tree-sitter extraction — classes, structs, enums, typedefs, and functions.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_cpp::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    extract_cpp(&tree.root_node(), source, file, None, result);
}

fn extract_cpp(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    match node.kind() {
        "class_specifier" | "struct_specifier" => {
            extract_type(node, source, file, owner, result);
            return;
        }
        "enum_specifier" => extract_enum(node, source, file, result),
        "type_definition" | "alias_declaration" => extract_typedef(node, source, file, result),
        "function_definition" | "declaration" => {
            extract_function(node, source, file, owner, result)
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_cpp(&child, source, file, owner, result);
    }
}

fn extract_type(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    let Some(name) =
        child_text_by_field(node, "name", source).or_else(|| first_identifier(node, source))
    else {
        return;
    };
    let qualified = qualify(owner, &name);
    let kind = if node.kind() == "class_specifier" {
        TypeKind::Class
    } else {
        TypeKind::Struct
    };

    result.types.insert(
        qualified.clone(),
        TypeInfo {
            name: qualified.clone(),
            source: source_loc(file, node),
            kind,
            fields: collect_fields(node, source),
            visibility: Visibility::Internal,
            implements: base_classes(node, source, &name),
            ..Default::default()
        },
    );

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_cpp(&child, source, file, Some(&qualified), result);
    }
}

fn extract_enum(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let Some(name) =
        child_text_by_field(node, "name", source).or_else(|| first_identifier(node, source))
    else {
        return;
    };
    result.types.entry(name.clone()).or_insert(TypeInfo {
        name,
        source: source_loc(file, node),
        kind: TypeKind::Enum,
        variants: collect_kinds(node, source, &["enumerator"]),
        visibility: Visibility::Internal,
        ..Default::default()
    });
}

fn extract_typedef(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let Some(name) = last_identifier(node, source) else {
        return;
    };
    result.types.entry(name.clone()).or_insert(TypeInfo {
        name,
        source: source_loc(file, node),
        kind: TypeKind::TypeAlias,
        visibility: Visibility::Internal,
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
    if !node_text(node, source).contains('(') {
        return;
    }
    let Some(name) = child_text_by_field(node, "declarator", source)
        .and_then(|text| identifier_from_signature(&text))
        .or_else(|| child_text_by_field(node, "name", source))
    else {
        return;
    };
    let qualified = qualify(owner, &name);
    result.functions.entry(qualified).or_insert(FunctionInfo {
        name: name.clone(),
        source: source_loc(file, node),
        signature: first_line(node_text(node, source)),
        visibility: Visibility::Internal,
        ..Default::default()
    });

    if let Some(owner) = owner {
        if let Some(typedef) = result.types.get_mut(owner) {
            if !typedef.methods.contains(&name) {
                typedef.methods.push(name);
            }
        }
    }
}

fn collect_fields(node: &Node, source: &str) -> Vec<Field> {
    let mut fields = Vec::new();
    collect_fields_inner(node, source, &mut fields);
    fields
}

fn collect_fields_inner(node: &Node, source: &str, fields: &mut Vec<Field>) {
    if node.kind() == "field_declaration" {
        if let Some(name) = last_identifier(node, source) {
            fields.push(Field {
                name,
                type_name: node_text(node, source)
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_string(),
                optional: false,
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_fields_inner(&child, source, fields);
    }
}

fn collect_kinds(node: &Node, source: &str, kinds: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    collect_kinds_inner(node, source, kinds, &mut out);
    out
}

fn collect_kinds_inner(node: &Node, source: &str, kinds: &[&str], out: &mut Vec<String>) {
    if kinds.contains(&node.kind()) {
        if let Some(name) = first_identifier(node, source) {
            out.push(name);
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_kinds_inner(&child, source, kinds, out);
    }
}

fn base_classes(node: &Node, source: &str, own_name: &str) -> Vec<String> {
    collect_kinds(node, source, &["base_class_clause", "type_identifier"])
        .into_iter()
        .filter(|name| name != own_name)
        .collect()
}

fn child_text_by_field(node: &Node, field: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field)
        .map(|child| node_text(&child, source).trim().to_string())
        .filter(|text| !text.is_empty())
}

fn first_identifier(node: &Node, source: &str) -> Option<String> {
    if matches!(
        node.kind(),
        "identifier" | "type_identifier" | "field_identifier"
    ) {
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

fn last_identifier(node: &Node, source: &str) -> Option<String> {
    let mut last = None;
    collect_last_identifier(node, source, &mut last);
    last
}

fn collect_last_identifier(node: &Node, source: &str, last: &mut Option<String>) {
    if matches!(
        node.kind(),
        "identifier" | "type_identifier" | "field_identifier"
    ) {
        *last = Some(node_text(node, source).to_string());
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_last_identifier(&child, source, last);
    }
}

fn identifier_from_signature(text: &str) -> Option<String> {
    text.split('(')
        .next()
        .and_then(|prefix| {
            prefix
                .split(|c: char| !c.is_alphanumeric() && c != '_' && c != ':')
                .filter(|part| !part.is_empty())
                .next_back()
        })
        .map(|name| name.rsplit("::").next().unwrap_or(name).to_string())
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
