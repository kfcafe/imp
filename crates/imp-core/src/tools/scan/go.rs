//! Go tree-sitter extraction — structs, interfaces, functions, methods.
//! Ported from uu-manifest's Go language adapter.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return,
    };
    let is_test = file.ends_with("_test.go");
    extract_go(&tree.root_node(), source, file, is_test, result);
}

fn extract_go(root: &Node, source: &str, file: &str, is_test: bool, result: &mut ScanResult) {
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        match child.kind() {
            "type_declaration" => extract_type_decl(&child, source, file, result),
            "function_declaration" => extract_function(&child, source, file, is_test, result),
            "method_declaration" => extract_method(&child, source, file, is_test, result),
            _ => {}
        }
    }
}

fn extract_type_decl(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "type_spec" {
            let name = match child.child_by_field_name("name") {
                Some(n) => node_text(&n, source).to_string(),
                None => continue,
            };
            let vis = go_visibility(&name);
            let type_node = match child.child_by_field_name("type") {
                Some(n) => n,
                None => continue,
            };

            match type_node.kind() {
                "struct_type" => {
                    let fields = extract_struct_fields(&type_node, source);
                    result.types.insert(
                        name.clone(),
                        TypeInfo {
                            name,
                            source: source_loc(file, &child),
                            kind: TypeKind::Struct,
                            fields,
                            visibility: vis,
                            ..Default::default()
                        },
                    );
                }
                "interface_type" => {
                    let methods = extract_interface_methods(&type_node, source);
                    result.types.insert(
                        name.clone(),
                        TypeInfo {
                            name,
                            source: source_loc(file, &child),
                            kind: TypeKind::Interface,
                            methods,
                            visibility: vis,
                            ..Default::default()
                        },
                    );
                }
                _ => {
                    result.types.insert(
                        name.clone(),
                        TypeInfo {
                            name,
                            source: source_loc(file, &child),
                            kind: TypeKind::TypeAlias,
                            visibility: vis,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
}

fn extract_struct_fields(node: &Node, source: &str) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "field_declaration_list" {
            let mut list_cursor = child.walk();
            for field_node in child.named_children(&mut list_cursor) {
                if field_node.kind() == "field_declaration" {
                    extract_single_field(&field_node, source, &mut fields);
                }
            }
        } else if child.kind() == "field_declaration" {
            extract_single_field(&child, source, &mut fields);
        }
    }
    fields
}

fn extract_single_field(field_node: &Node, source: &str, fields: &mut Vec<Field>) {
    let type_name = field_node
        .child_by_field_name("type")
        .map(|t| node_text(&t, source).to_string())
        .unwrap_or_default();
    let mut inner = field_node.walk();
    for name_child in field_node.named_children(&mut inner) {
        if name_child.kind() == "field_identifier" {
            let name = node_text(&name_child, source).to_string();
            let optional = type_name.starts_with('*');
            fields.push(Field {
                name,
                type_name: type_name.clone(),
                optional,
            });
        }
    }
}

fn extract_interface_methods(node: &Node, source: &str) -> Vec<String> {
    let mut methods = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "method_spec" || child.kind() == "method_elem" {
            if let Some(name_node) = child.child_by_field_name("name") {
                methods.push(node_text(&name_node, source).to_string());
            }
        }
    }
    methods
}

fn extract_function(
    node: &Node,
    source: &str,
    file: &str,
    is_test_file: bool,
    result: &mut ScanResult,
) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(&n, source).to_string(),
        None => return,
    };
    let vis = go_visibility(&name);
    let is_test = is_test_file && name.starts_with("Test");
    let sig = build_fn_signature(node, source, &name);

    result.functions.insert(
        name.clone(),
        FunctionInfo {
            name,
            source: source_loc(file, node),
            signature: sig,
            visibility: vis,
            is_async: false,
            is_test,
        },
    );
}

fn extract_method(
    node: &Node,
    source: &str,
    file: &str,
    is_test_file: bool,
    result: &mut ScanResult,
) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(&n, source).to_string(),
        None => return,
    };
    let receiver_type = extract_receiver_type(node, source);
    let vis = go_visibility(&name);
    let is_test = is_test_file && name.starts_with("Test");
    let sig = build_fn_signature(node, source, &name);

    let qualified = if receiver_type.is_empty() {
        name.clone()
    } else {
        format!("{receiver_type}::{name}")
    };
    result.functions.insert(
        qualified,
        FunctionInfo {
            name: name.clone(),
            source: source_loc(file, node),
            signature: sig,
            visibility: vis,
            is_async: false,
            is_test,
        },
    );

    if !receiver_type.is_empty() {
        if let Some(typedef) = result.types.get_mut(&receiver_type) {
            if !typedef.methods.contains(&name) {
                typedef.methods.push(name);
            }
        }
    }
}

fn extract_receiver_type(node: &Node, source: &str) -> String {
    let receiver = match node.child_by_field_name("receiver") {
        Some(r) => r,
        None => return String::new(),
    };
    let mut cursor = receiver.walk();
    for child in receiver.named_children(&mut cursor) {
        if child.kind() == "parameter_declaration" {
            if let Some(type_node) = child.child_by_field_name("type") {
                let text = node_text(&type_node, source);
                return text.trim_start_matches('*').to_string();
            }
        }
    }
    String::new()
}

fn build_fn_signature(node: &Node, source: &str, name: &str) -> String {
    let params = node
        .child_by_field_name("parameters")
        .map(|n| node_text(&n, source))
        .unwrap_or("()");
    let ret = node
        .child_by_field_name("result")
        .map(|n| format!(" {}", node_text(&n, source)))
        .unwrap_or_default();
    format!("func {name}{params}{ret}")
}

fn go_visibility(name: &str) -> Visibility {
    if name.starts_with(|c: char| c.is_uppercase()) {
        Visibility::Public
    } else {
        Visibility::Private
    }
}

fn node_text<'a>(node: &Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

fn source_loc(file: &str, node: &Node) -> String {
    format!("{}:{}", file, node.start_position().row + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_go_str(source: &str) -> ScanResult {
        let mut result = ScanResult::default();
        parse(source, "main.go", &mut result);
        result
    }

    #[test]
    fn struct_with_fields() {
        let r = parse_go_str(
            r#"
package main
type User struct {
    Name  string
    Email string
    Age   int
}
"#,
        );
        let t = &r.types["User"];
        assert_eq!(t.kind, TypeKind::Struct);
        assert_eq!(t.visibility, Visibility::Public);
        assert_eq!(t.fields.len(), 3);
    }

    #[test]
    fn unexported_is_private() {
        let r = parse_go_str("package main\ntype config struct { debug bool }");
        assert_eq!(r.types["config"].visibility, Visibility::Private);
    }

    #[test]
    fn interface_methods() {
        let r = parse_go_str(
            r#"
package main
type Repository interface {
    Get(id string) error
    Save(item Item) error
}
"#,
        );
        let t = &r.types["Repository"];
        assert_eq!(t.kind, TypeKind::Interface);
        assert!(t.methods.contains(&"Get".to_string()));
        assert!(t.methods.contains(&"Save".to_string()));
    }

    #[test]
    fn method_adds_to_type() {
        let r = parse_go_str(
            r#"
package main
type Server struct { Port int }
func (s *Server) Start() error { return nil }
"#,
        );
        let t = &r.types["Server"];
        assert!(t.methods.contains(&"Start".to_string()));
        assert!(r.functions.contains_key("Server::Start"));
    }

    #[test]
    fn pointer_field_optional() {
        let r = parse_go_str(
            "package main\ntype Config struct {\n    Name string\n    Parent *Config\n}",
        );
        let t = &r.types["Config"];
        assert!(!t.fields[0].optional);
        assert!(t.fields[1].optional);
    }
}
