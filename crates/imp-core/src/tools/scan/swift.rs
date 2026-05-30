//! Swift tree-sitter extraction — nominal types, extensions, and functions.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_swift::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    extract_swift(&tree.root_node(), source, file, None, result);
    extract_swift_lines(source, file, result);
}

fn extract_swift(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    match node.kind() {
        "class_declaration"
        | "struct_declaration"
        | "enum_declaration"
        | "protocol_declaration" => {
            extract_type(node, source, file, owner, result);
            return;
        }
        "extension_declaration" => {
            extract_extension(node, source, file, result);
            return;
        }
        "function_declaration" => extract_function(node, source, file, owner, result),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_swift(&child, source, file, owner, result);
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
    result.types.entry(qualified.clone()).or_insert(TypeInfo {
        name: qualified.clone(),
        source: source_loc(file, node),
        kind: type_kind(node.kind()),
        visibility: visibility(node_text(node, source)),
        implements: inheritance(node, source, &name),
        variants: if node.kind() == "enum_declaration" {
            collect_case_names(node, source)
        } else {
            Vec::new()
        },
        ..Default::default()
    });

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_swift(&child, source, file, Some(&qualified), result);
    }
}

fn extract_extension(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let Some(name) =
        child_text_by_field(node, "name", source).or_else(|| first_identifier(node, source))
    else {
        return;
    };

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_swift(&child, source, file, Some(&name), result);
    }
}

fn extract_function(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    let Some(name) = child_text_by_field(node, "name", source)
        .or_else(|| function_name(node_text(node, source)))
    else {
        return;
    };
    insert_function(
        name,
        node_text(node, source),
        source_loc(file, node),
        owner,
        result,
    );
}

fn extract_swift_lines(source: &str, file: &str, result: &mut ScanResult) {
    let mut stack: Vec<(String, usize)> = Vec::new();
    for (idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        while stack
            .last()
            .is_some_and(|(_, depth)| brace_depth_before(source, idx) < *depth)
        {
            stack.pop();
        }
        if let Some((kind, name)) = line_type(trimmed) {
            let loc = format!("{}:{}", file, idx + 1);
            let visibility = visibility(trimmed);
            let implements = line_inheritance(trimmed);
            let qualified_name = stack
                .last()
                .map(|(owner, _)| format!("{owner}::{name}"))
                .unwrap_or_else(|| name.clone());
            let entry = result
                .types
                .entry(qualified_name.clone())
                .or_insert(TypeInfo {
                    name: qualified_name.clone(),
                    source: loc.clone(),
                    kind: kind.clone(),
                    visibility: visibility.clone(),
                    implements: implements.clone(),
                    ..Default::default()
                });
            if !trimmed.contains("extension ") {
                entry.kind = kind;
                entry.visibility = visibility;
                if !implements.is_empty() {
                    entry.implements = implements;
                }
                if entry.source.is_empty() {
                    entry.source = loc;
                }
            }
            stack.push((qualified_name, brace_depth_before(source, idx) + 1));
            continue;
        }
        if let Some(name) = function_name(trimmed) {
            let owner = stack.last().map(|(name, _)| name.as_str());
            insert_function(
                name,
                trimmed,
                format!("{}:{}", file, idx + 1),
                owner,
                result,
            );
        }
    }
}

fn insert_function(
    name: String,
    text: &str,
    source: String,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    let qualified = qualify(owner, &name);
    result.functions.entry(qualified).or_insert(FunctionInfo {
        name: name.clone(),
        source,
        signature: first_line(text),
        visibility: visibility(text),
        is_async: text.contains(" async") || text.contains(" async "),
        is_test: name.starts_with("test"),
    });
    if let Some(owner) = owner {
        if let Some(typedef) = result.types.get_mut(owner) {
            if !typedef.methods.contains(&name) {
                typedef.methods.push(name);
            }
        }
    }
}

fn line_type(line: &str) -> Option<(TypeKind, String)> {
    if let Some(name) = name_after_keyword(line, "extension") {
        return Some((TypeKind::Protocol, name));
    }

    for (keyword, kind) in [
        ("struct", TypeKind::Struct),
        ("class", TypeKind::Class),
        ("enum", TypeKind::Enum),
        ("protocol", TypeKind::Protocol),
    ] {
        if let Some(name) = name_after_keyword(line, keyword) {
            return Some((kind, name));
        }
    }
    None
}

fn name_after_keyword(line: &str, keyword: &str) -> Option<String> {
    let rest = line.split_once(keyword)?.1.trim_start();
    rest.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
        .next()
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

fn function_name(text: &str) -> Option<String> {
    name_after_keyword(text.trim_start(), "func")
}

fn line_inheritance(line: &str) -> Vec<String> {
    line.split_once(':')
        .map(|(_, rest)| {
            rest.split(['{', ','])
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn inheritance(node: &Node, source: &str, own_name: &str) -> Vec<String> {
    collect_texts(
        node,
        source,
        &["inheritance_specifier", "user_type", "type_identifier"],
    )
    .into_iter()
    .filter(|item| item != own_name)
    .collect()
}

fn collect_case_names(node: &Node, source: &str) -> Vec<String> {
    collect_texts(
        node,
        source,
        &["enum_case", "enum_entry", "simple_identifier"],
    )
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

fn first_identifier(node: &Node, source: &str) -> Option<String> {
    if matches!(
        node.kind(),
        "identifier" | "simple_identifier" | "type_identifier"
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

fn type_kind(kind: &str) -> TypeKind {
    match kind {
        "struct_declaration" => TypeKind::Struct,
        "enum_declaration" => TypeKind::Enum,
        "protocol_declaration" => TypeKind::Protocol,
        _ => TypeKind::Class,
    }
}

fn visibility(text: &str) -> Visibility {
    let trimmed = text.trim_start();
    if trimmed.starts_with("private ") || trimmed.starts_with("fileprivate ") {
        Visibility::Private
    } else if trimmed.starts_with("public ") || trimmed.starts_with("open ") {
        Visibility::Public
    } else {
        Visibility::Internal
    }
}

fn brace_depth_before(source: &str, line_idx: usize) -> usize {
    source
        .lines()
        .take(line_idx)
        .map(|line| {
            line.matches('{')
                .count()
                .saturating_sub(line.matches('}').count())
        })
        .sum()
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
