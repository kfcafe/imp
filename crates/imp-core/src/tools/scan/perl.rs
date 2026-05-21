//! Perl tree-sitter extraction — packages and subroutines.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_perl::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    extract_perl(&tree.root_node(), source, file, None, result);
    extract_perl_lines(source, file, result);
}

fn extract_perl_lines(source: &str, file: &str, result: &mut ScanResult) {
    let mut package: Option<String> = None;
    for (idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("package ") {
            if let Some(name) = rest
                .split(';')
                .next()
                .map(str::trim)
                .filter(|name| !name.is_empty())
            {
                package = Some(name.to_string());
                result.types.entry(name.to_string()).or_insert(TypeInfo {
                    name: name.to_string(),
                    source: format!("{}:{}", file, idx + 1),
                    kind: TypeKind::Class,
                    visibility: Visibility::Public,
                    ..Default::default()
                });
            }
        }
        if let Some(rest) = trimmed.strip_prefix("sub ") {
            if let Some(name) = rest
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .next()
                .filter(|name| !name.is_empty())
            {
                let qualified = package
                    .as_ref()
                    .map(|package| format!("{package}::{name}"))
                    .unwrap_or_else(|| name.to_string());
                result.functions.entry(qualified).or_insert(FunctionInfo {
                    name: name.to_string(),
                    source: format!("{}:{}", file, idx + 1),
                    signature: trimmed.to_string(),
                    visibility: Visibility::Public,
                    ..Default::default()
                });
                if let Some(package) = package.as_deref() {
                    if let Some(typedef) = result.types.get_mut(package) {
                        let method = name.to_string();
                        if !typedef.methods.contains(&method) {
                            typedef.methods.push(method);
                        }
                    }
                }
            }
        }
    }
}

fn extract_perl(
    node: &Node,
    source: &str,
    file: &str,
    package: Option<&str>,
    result: &mut ScanResult,
) {
    let mut current_package = package.map(str::to_string);
    match node.kind() {
        "package_statement" | "package_declaration" | "package" => {
            if let Some(name) = package_name(node, source) {
                current_package = Some(name.clone());
                result.types.entry(name.clone()).or_insert(TypeInfo {
                    name,
                    source: source_loc(file, node),
                    kind: TypeKind::Class,
                    visibility: Visibility::Public,
                    ..Default::default()
                });
            }
        }
        "subroutine_declaration"
        | "subroutine_definition"
        | "sub_declaration"
        | "sub_definition" => {
            extract_sub(node, source, file, package, result);
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_perl(&child, source, file, current_package.as_deref(), result);
    }
}

fn extract_sub(
    node: &Node,
    source: &str,
    file: &str,
    package: Option<&str>,
    result: &mut ScanResult,
) {
    let Some(name) =
        child_text_by_field(node, "name", source).or_else(|| first_identifier(node, source))
    else {
        return;
    };
    let package = package
        .map(str::to_string)
        .or_else(|| package_before(source, node.start_byte()));
    let qualified = package
        .as_ref()
        .map(|package| format!("{package}::{name}"))
        .unwrap_or_else(|| name.clone());
    result.functions.entry(qualified).or_insert(FunctionInfo {
        name: name.clone(),
        source: source_loc(file, node),
        signature: first_line(node_text(node, source)),
        visibility: Visibility::Public,
        ..Default::default()
    });
    if let Some(package) = package.as_deref() {
        if let Some(typedef) = result.types.get_mut(package) {
            if !typedef.methods.contains(&name) {
                typedef.methods.push(name);
            }
        }
    }
}

fn package_before(source: &str, byte: usize) -> Option<String> {
    source
        .get(..byte)?
        .lines()
        .filter_map(|line| line.trim().strip_prefix("package "))
        .filter_map(|rest| rest.split(';').next())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .last()
        .map(str::to_string)
}

fn package_name(node: &Node, source: &str) -> Option<String> {
    child_text_by_field(node, "name", source).or_else(|| first_identifier(node, source))
}

fn child_text_by_field(node: &Node, field: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field)
        .map(|child| {
            node_text(&child, source)
                .trim()
                .trim_end_matches(';')
                .to_string()
        })
        .filter(|text| !text.is_empty())
}

fn first_identifier(node: &Node, source: &str) -> Option<String> {
    if matches!(node.kind(), "identifier" | "package_name" | "bareword") {
        return Some(node_text(node, source).trim_end_matches(';').to_string());
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
