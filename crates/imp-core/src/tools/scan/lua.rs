//! Lua tree-sitter extraction — functions and table-like modules.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_lua::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    extract_lua(&tree.root_node(), source, file, result);
}

fn extract_lua(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    match node.kind() {
        "function_declaration" | "function_definition" => {
            extract_function(node, source, file, result)
        }
        "variable_declaration" | "assignment_statement" => {
            extract_table_module(node, source, file, result)
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_lua(&child, source, file, result);
    }
}

fn extract_function(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let Some(name) = function_name(node, source) else {
        return;
    };
    let owner = name
        .split(['.', ':'])
        .next()
        .filter(|owner| *owner != name)
        .map(str::to_string);
    result.functions.insert(
        name.clone(),
        FunctionInfo {
            name: name.clone(),
            source: source_loc(file, node),
            signature: first_line(node_text(node, source)),
            visibility: Visibility::Public,
            ..Default::default()
        },
    );
    if let Some(owner) = owner {
        result.types.entry(owner.clone()).or_insert(TypeInfo {
            name: owner.clone(),
            source: source_loc(file, node),
            kind: TypeKind::Class,
            visibility: Visibility::Public,
            ..Default::default()
        });
        if let Some(typedef) = result.types.get_mut(&owner) {
            let method = name.rsplit(['.', ':']).next().unwrap_or(&name).to_string();
            if !typedef.methods.contains(&method) {
                typedef.methods.push(method);
            }
        }
    }
}

fn extract_table_module(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let text = node_text(node, source);
    if !text.contains('{') {
        return;
    }
    let Some(name) = first_identifier(node, source) else {
        return;
    };
    result.types.entry(name.clone()).or_insert(TypeInfo {
        name,
        source: source_loc(file, node),
        kind: TypeKind::Class,
        visibility: Visibility::Public,
        ..Default::default()
    });
}

fn function_name(node: &Node, source: &str) -> Option<String> {
    child_text_by_field(node, "name", source).or_else(|| {
        node_text(node, source)
            .strip_prefix("function")
            .and_then(|rest| rest.split('(').next())
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
    })
}

fn child_text_by_field(node: &Node, field: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field)
        .map(|child| node_text(&child, source).trim().to_string())
        .filter(|text| !text.is_empty())
}

fn first_identifier(node: &Node, source: &str) -> Option<String> {
    if matches!(node.kind(), "identifier" | "variable_name") {
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

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().trim().to_string()
}

fn node_text<'a>(node: &Node, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

fn source_loc(file: &str, node: &Node) -> String {
    format!("{}:{}", file, node.start_position().row + 1)
}
