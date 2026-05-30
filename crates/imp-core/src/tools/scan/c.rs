//! C tree-sitter extraction — structs, enums, typedefs, and functions.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_c::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    extract_c(&tree.root_node(), source, file, result);
}

fn extract_c(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    match node.kind() {
        "struct_specifier" => extract_type(node, source, file, TypeKind::Struct, result),
        "enum_specifier" => extract_type(node, source, file, TypeKind::Enum, result),
        "type_definition" => extract_typedef(node, source, file, result),
        "function_definition" => extract_function(node, source, file, result),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_c(&child, source, file, result);
    }
}

fn extract_type(node: &Node, source: &str, file: &str, kind: TypeKind, result: &mut ScanResult) {
    let Some(name) =
        child_text_by_field(node, "name", source).or_else(|| first_identifier(node, source))
    else {
        return;
    };
    let fields = if kind == TypeKind::Struct {
        collect_fields(node, source)
    } else {
        Vec::new()
    };
    let variants = if kind == TypeKind::Enum {
        collect_kinds(node, source, &["enumerator"])
    } else {
        Vec::new()
    };

    result.types.entry(name.clone()).or_insert(TypeInfo {
        name,
        source: source_loc(file, node),
        kind,
        fields,
        variants,
        visibility: Visibility::Internal,
        ..Default::default()
    });
}

fn extract_typedef(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let Some(name) = last_identifier(node, source) else {
        return;
    };
    let kind = if has_kind(node, "struct_specifier") {
        TypeKind::Struct
    } else if has_kind(node, "enum_specifier") {
        TypeKind::Enum
    } else {
        TypeKind::TypeAlias
    };
    result.types.entry(name.clone()).or_insert(TypeInfo {
        name,
        source: source_loc(file, node),
        kind,
        visibility: Visibility::Internal,
        ..Default::default()
    });
}

fn extract_function(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let Some(name) = child_text_by_field(node, "declarator", source)
        .and_then(|text| identifier_from_signature(&text))
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
            visibility: Visibility::Internal,
            ..Default::default()
        });
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

fn child_text_by_field(node: &Node, field: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field)
        .map(|child| node_text(&child, source).trim().to_string())
        .filter(|text| !text.is_empty())
}

fn has_kind(node: &Node, kind: &str) -> bool {
    if node.kind() == kind {
        return true;
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|child| has_kind(&child, kind));
    found
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
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .rfind(|part| !part.is_empty())
        })
        .map(str::to_string)
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
