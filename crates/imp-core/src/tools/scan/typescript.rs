//! TypeScript/TSX tree-sitter extraction — interfaces, types, classes, enums, functions.
//! Ported from uu-manifest's TypeScript language adapter.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, is_tsx: bool, result: &mut ScanResult) {
    let mut parser = Parser::new();
    let language = if is_tsx {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    } else {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    };
    if parser.set_language(&language).is_err() {
        return;
    }
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return,
    };
    extract_typescript(&tree.root_node(), source, file, result);
}

fn extract_typescript(root: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        match child.kind() {
            "export_statement" => extract_export(&child, source, file, result),
            "interface_declaration" => {
                extract_interface(&child, source, file, Visibility::Private, result);
            }
            "type_alias_declaration" => {
                extract_type_alias(&child, source, file, Visibility::Private, result);
            }
            "enum_declaration" => {
                extract_enum(&child, source, file, Visibility::Private, result);
            }
            "class_declaration" => {
                extract_class(&child, source, file, Visibility::Private, result);
            }
            "function_declaration" => {
                extract_function_decl(&child, source, file, Visibility::Private, result);
            }
            "lexical_declaration" => {
                extract_lexical_functions(&child, source, file, Visibility::Private, result);
            }
            _ => {}
        }
    }
}

fn extract_export(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "interface_declaration" => {
                extract_interface(&child, source, file, Visibility::Public, result);
            }
            "type_alias_declaration" => {
                extract_type_alias(&child, source, file, Visibility::Public, result);
            }
            "enum_declaration" => {
                extract_enum(&child, source, file, Visibility::Public, result);
            }
            "class_declaration" => {
                extract_class(&child, source, file, Visibility::Public, result);
            }
            "function_declaration" => {
                extract_function_decl(&child, source, file, Visibility::Public, result);
            }
            "lexical_declaration" => {
                extract_lexical_functions(&child, source, file, Visibility::Public, result);
            }
            _ => {}
        }
    }
}

fn extract_interface(
    node: &Node,
    source: &str,
    file: &str,
    visibility: Visibility,
    result: &mut ScanResult,
) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(&n, source),
        None => return,
    };

    let mut implements = Vec::new();
    extract_heritage(node, source, &mut implements);

    result.types.insert(
        name.clone(),
        TypeInfo {
            name,
            source: file.to_string(),
            kind: TypeKind::Interface,
            visibility,
            implements,
            ..Default::default()
        },
    );
}

fn extract_type_alias(
    node: &Node,
    source: &str,
    file: &str,
    visibility: Visibility,
    result: &mut ScanResult,
) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(&n, source),
        None => return,
    };

    result.types.insert(
        name.clone(),
        TypeInfo {
            name,
            source: file.to_string(),
            kind: TypeKind::TypeAlias,
            visibility,
            ..Default::default()
        },
    );
}

fn extract_enum(
    node: &Node,
    source: &str,
    file: &str,
    visibility: Visibility,
    result: &mut ScanResult,
) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(&n, source),
        None => return,
    };
    let variants = extract_enum_variants(node, source);

    result.types.insert(
        name.clone(),
        TypeInfo {
            name,
            source: file.to_string(),
            kind: TypeKind::Enum,
            variants,
            visibility,
            ..Default::default()
        },
    );
}

fn extract_enum_variants(node: &Node, source: &str) -> Vec<String> {
    let Some(body) = node.child_by_field_name("body") else {
        return Vec::new();
    };
    let mut variants = Vec::new();
    let mut cursor = body.walk();
    for child in body.named_children(&mut cursor) {
        if let Some(name_node) = child.child_by_field_name("name") {
            variants.push(node_text(&name_node, source));
        }
    }
    variants
}

fn extract_class(
    node: &Node,
    source: &str,
    file: &str,
    visibility: Visibility,
    result: &mut ScanResult,
) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(&n, source),
        None => return,
    };

    let mut implements = Vec::new();
    extract_heritage(node, source, &mut implements);

    let mut methods = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        extract_class_methods(&body, source, file, &name, result, &mut methods);
    }

    result.types.insert(
        name.clone(),
        TypeInfo {
            name,
            source: file.to_string(),
            kind: TypeKind::Class,
            visibility,
            implements,
            methods,
            ..Default::default()
        },
    );
}

fn extract_heritage(node: &Node, source: &str, implements: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "extends_clause"
            || child.kind() == "extends_type_clause"
            || child.kind() == "implements_clause"
        {
            let mut inner_cursor = child.walk();
            for inner in child.named_children(&mut inner_cursor) {
                let text = node_text(&inner, source);
                if !text.is_empty() {
                    implements.push(text);
                }
            }
        }
    }
}

fn extract_class_methods(
    body: &Node,
    source: &str,
    file: &str,
    class_name: &str,
    result: &mut ScanResult,
    methods: &mut Vec<String>,
) {
    let mut cursor = body.walk();
    for child in body.named_children(&mut cursor) {
        if child.kind() == "method_definition" {
            let name = match child.child_by_field_name("name") {
                Some(n) => node_text(&n, source),
                None => continue,
            };
            let is_async = has_child_kind(&child, "async");
            let params = child
                .child_by_field_name("parameters")
                .map(|p| node_text(&p, source))
                .unwrap_or_default();
            let signature = if is_async {
                format!("async {name}{params}")
            } else {
                format!("{name}{params}")
            };

            let qualified = format!("{class_name}::{name}");
            result.functions.insert(
                qualified,
                FunctionInfo {
                    name: name.clone(),
                    source: file.to_string(),
                    signature,
                    visibility: Visibility::Public,
                    is_async,
                    ..Default::default()
                },
            );
            methods.push(name);
        }
    }
}

fn extract_function_decl(
    node: &Node,
    source: &str,
    file: &str,
    visibility: Visibility,
    result: &mut ScanResult,
) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(&n, source),
        None => return,
    };
    let is_async = has_child_kind(node, "async");
    let params = node
        .child_by_field_name("parameters")
        .map(|p| node_text(&p, source))
        .unwrap_or_default();
    let return_type = node
        .child_by_field_name("return_type")
        .map(|r| node_text(&r, source))
        .unwrap_or_default();

    let async_prefix = if is_async { "async " } else { "" };
    let signature = format!("{async_prefix}function {name}{params}{return_type}");

    result.functions.insert(
        name.clone(),
        FunctionInfo {
            name,
            source: file.to_string(),
            signature,
            visibility,
            is_async,
            ..Default::default()
        },
    );
}

fn extract_lexical_functions(
    node: &Node,
    source: &str,
    file: &str,
    visibility: Visibility,
    result: &mut ScanResult,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            let name = match child.child_by_field_name("name") {
                Some(n) => node_text(&n, source),
                None => continue,
            };
            let value = match child.child_by_field_name("value") {
                Some(v) => v,
                None => continue,
            };
            if !matches!(
                value.kind(),
                "arrow_function" | "function_expression" | "function"
            ) {
                continue;
            }

            let is_async = has_child_kind(&value, "async");
            let params = value
                .child_by_field_name("parameters")
                .map(|p| node_text(&p, source))
                .unwrap_or_default();
            let async_prefix = if is_async { "async " } else { "" };
            let signature = format!("{async_prefix}const {name} = {params} =>");

            result.functions.insert(
                name.clone(),
                FunctionInfo {
                    name,
                    source: file.to_string(),
                    signature,
                    visibility: visibility.clone(),
                    is_async,
                    ..Default::default()
                },
            );
        }
    }
}

fn has_child_kind(node: &Node, kind: &str) -> bool {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).any(|c| c.kind() == kind);
    result
}

fn node_text(node: &Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ts(source: &str) -> ScanResult {
        let mut result = ScanResult::default();
        parse(source, "src/models.ts", false, &mut result);
        result
    }

    #[test]
    fn exported_interface() {
        let r = parse_ts("export interface User { name: string; }");
        let t = &r.types["User"];
        assert_eq!(t.kind, TypeKind::Interface);
        assert_eq!(t.visibility, Visibility::Public);
    }

    #[test]
    fn private_interface() {
        let r = parse_ts("interface Internal { x: number; }");
        assert_eq!(r.types["Internal"].visibility, Visibility::Private);
    }

    #[test]
    fn enum_variants() {
        let r = parse_ts(r#"export enum Status { Active = "active", Inactive = "inactive" }"#);
        let t = &r.types["Status"];
        assert_eq!(t.kind, TypeKind::Enum);
        assert_eq!(t.variants, vec!["Active", "Inactive"]);
    }

    #[test]
    fn class_with_methods() {
        let r = parse_ts(
            r#"
export class UserService {
    async findById(id: string): Promise<User> {
        return {} as User;
    }
}
"#,
        );
        let t = &r.types["UserService"];
        assert_eq!(t.kind, TypeKind::Class);
        assert!(t.methods.contains(&"findById".to_string()));
        assert!(r.functions["UserService::findById"].is_async);
    }

    #[test]
    fn exported_function() {
        let r = parse_ts("export function createUser(name: string): User { return { name }; }");
        let f = &r.functions["createUser"];
        assert_eq!(f.visibility, Visibility::Public);
        assert!(f.signature.contains("createUser"));
    }

    #[test]
    fn arrow_function() {
        let r = parse_ts("export const greet = (name: string): string => { return `Hello`; };");
        assert!(r.functions.contains_key("greet"));
        assert_eq!(r.functions["greet"].visibility, Visibility::Public);
    }

    #[test]
    fn async_function() {
        let r = parse_ts("export async function fetchData(url: string): Promise<void> {}");
        assert!(r.functions["fetchData"].is_async);
    }
}
