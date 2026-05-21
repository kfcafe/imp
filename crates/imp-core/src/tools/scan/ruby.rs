//! Ruby tree-sitter extraction — modules, classes, methods, and singleton methods.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_ruby::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    extract_ruby(&tree.root_node(), source, file, None, result);
}

fn extract_ruby(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    match node.kind() {
        "module" | "class" => {
            extract_type(node, source, file, owner, result);
            return;
        }
        "method" | "singleton_method" => extract_method(node, source, file, owner, result),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_ruby(&child, source, file, owner, result);
    }
}

fn extract_type(
    node: &Node,
    source: &str,
    file: &str,
    owner: Option<&str>,
    result: &mut ScanResult,
) {
    let Some(name) = first_named_const(node, source) else {
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
        extract_ruby(&child, source, file, Some(&qualified), result);
    }
}

fn extract_method(
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

fn first_named_const(node: &Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if matches!(child.kind(), "constant" | "scope_resolution") {
            return Some(node_text(&child, source).to_string());
        }
    }
    first_identifier(node, source)
}

fn child_text_by_field(node: &Node, field: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field)
        .map(|child| node_text(&child, source).trim().to_string())
        .filter(|text| !text.is_empty())
}

fn first_identifier(node: &Node, source: &str) -> Option<String> {
    if matches!(node.kind(), "identifier" | "constant") {
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
