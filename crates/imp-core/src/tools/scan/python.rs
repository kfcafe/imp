//! Python tree-sitter extraction — classes, functions, decorators.
//! Ported from uu-manifest's Python language adapter.

use tree_sitter::{Node, Parser};

use super::types::*;

pub fn parse(source: &str, file: &str, result: &mut ScanResult) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return,
    };
    let is_test = is_test_file(file);
    extract_python(&tree.root_node(), source, file, is_test, result);
}

fn is_test_file(path: &str) -> bool {
    let filename = path.rsplit('/').next().unwrap_or(path);
    filename.starts_with("test_") || filename.ends_with("_test.py")
}

fn extract_python(root: &Node, source: &str, file: &str, is_test: bool, result: &mut ScanResult) {
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        match child.kind() {
            "class_definition" => extract_class(&child, source, file, result),
            "function_definition" => {
                extract_function(&child, source, file, is_test, &[], result);
            }
            "decorated_definition" => {
                extract_decorated(&child, source, file, is_test, result);
            }
            _ => {}
        }
    }
}

fn extract_class(node: &Node, source: &str, file: &str, result: &mut ScanResult) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(&n, source),
        None => return,
    };

    let mut implements = Vec::new();
    if let Some(superclasses) = node.child_by_field_name("superclasses") {
        let mut cursor = superclasses.walk();
        for child in superclasses.named_children(&mut cursor) {
            let text = node_text(&child, source);
            if !text.contains('=') && !text.is_empty() {
                implements.push(text);
            }
        }
    }

    let visibility = if name.starts_with('_') {
        Visibility::Private
    } else {
        Visibility::Public
    };

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
        match child.kind() {
            "function_definition" => {
                if let Some(name) = extract_method(&child, source, file, class_name, &[], result) {
                    methods.push(name);
                }
            }
            "decorated_definition" => {
                let decorators = collect_decorators(&child, source);
                if let Some(func_node) = child.child_by_field_name("definition") {
                    if func_node.kind() == "function_definition" {
                        if let Some(name) = extract_method(
                            &func_node,
                            source,
                            file,
                            class_name,
                            &decorators,
                            result,
                        ) {
                            methods.push(name);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn extract_method(
    node: &Node,
    source: &str,
    file: &str,
    class_name: &str,
    decorators: &[String],
    result: &mut ScanResult,
) -> Option<String> {
    let name = node_text(&node.child_by_field_name("name")?, source);

    let visibility = if name.starts_with('_') && !name.starts_with("__") {
        Visibility::Private
    } else {
        Visibility::Public
    };

    let is_async = source[node.byte_range()].starts_with("async ");
    let params = node
        .child_by_field_name("parameters")
        .map(|p| node_text(&p, source))
        .unwrap_or_default();

    let decorator_prefix = decorators
        .iter()
        .map(|d| format!("@{d} "))
        .collect::<String>();
    let async_prefix = if is_async { "async " } else { "" };
    let signature = format!("{decorator_prefix}{async_prefix}def {name}{params}");

    let qualified = format!("{class_name}::{name}");
    result.functions.insert(
        qualified,
        FunctionInfo {
            name: name.clone(),
            source: file.to_string(),
            signature,
            visibility,
            is_async,
            ..Default::default()
        },
    );
    Some(name)
}

fn extract_function(
    node: &Node,
    source: &str,
    file: &str,
    is_test: bool,
    decorators: &[String],
    result: &mut ScanResult,
) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(&n, source),
        None => return,
    };

    let visibility = if name.starts_with('_') {
        Visibility::Private
    } else {
        Visibility::Public
    };

    let is_async = source[node.byte_range()].starts_with("async ");
    let func_is_test = is_test || name.starts_with("test_");

    let params = node
        .child_by_field_name("parameters")
        .map(|p| node_text(&p, source))
        .unwrap_or_default();

    let decorator_prefix = decorators
        .iter()
        .map(|d| format!("@{d} "))
        .collect::<String>();
    let async_prefix = if is_async { "async " } else { "" };
    let signature = format!("{decorator_prefix}{async_prefix}def {name}{params}");

    result.functions.insert(
        name.clone(),
        FunctionInfo {
            name,
            source: file.to_string(),
            signature,
            visibility,
            is_async,
            is_test: func_is_test,
        },
    );
}

fn extract_decorated(
    node: &Node,
    source: &str,
    file: &str,
    is_test: bool,
    result: &mut ScanResult,
) {
    let decorators = collect_decorators(node, source);
    if let Some(definition) = node.child_by_field_name("definition") {
        match definition.kind() {
            "function_definition" => {
                extract_function(&definition, source, file, is_test, &decorators, result);
            }
            "class_definition" => {
                extract_class(&definition, source, file, result);
            }
            _ => {}
        }
    }
}

fn collect_decorators(node: &Node, source: &str) -> Vec<String> {
    let mut decorators = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "decorator" {
            let text = node_text(&child, source);
            let name = text.strip_prefix('@').unwrap_or(&text).trim().to_string();
            decorators.push(name);
        }
    }
    decorators
}

fn node_text(node: &Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_py(source: &str) -> ScanResult {
        let mut result = ScanResult::default();
        parse(source, "models.py", &mut result);
        result
    }

    #[test]
    fn class_with_inheritance() {
        let r = parse_py(
            r#"
class Admin(User, Serializable):
    pass
"#,
        );
        let t = &r.types["Admin"];
        assert_eq!(t.kind, TypeKind::Class);
        assert!(t.implements.contains(&"User".to_string()));
        assert!(t.implements.contains(&"Serializable".to_string()));
    }

    #[test]
    fn function_visibility() {
        let r = parse_py("def create_user(name): pass\ndef _internal(): pass");
        assert_eq!(r.functions["create_user"].visibility, Visibility::Public);
        assert_eq!(r.functions["_internal"].visibility, Visibility::Private);
    }

    #[test]
    fn async_function() {
        let r = parse_py("async def fetch(url): pass");
        assert!(r.functions["fetch"].is_async);
    }

    #[test]
    fn class_methods() {
        let r = parse_py(
            r#"
class Service:
    def process(self): pass
    def _internal(self): pass
"#,
        );
        let t = &r.types["Service"];
        assert!(t.methods.contains(&"process".to_string()));
        assert!(t.methods.contains(&"_internal".to_string()));
    }

    #[test]
    fn decorated_function() {
        let r = parse_py(
            r#"
class Config:
    @property
    def name(self):
        return self._name
"#,
        );
        let f = &r.functions["Config::name"];
        assert!(f.signature.contains("@property"));
    }
}
