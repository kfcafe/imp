use imp_llm::ContentBlock;
use serde_json::{json, Map, Value};

use crate::error::{Error, Result};
use crate::tools::ToolOutput;

/// Normalize Lua tool parameter definitions into JSON Schema.
///
/// Lua extensions may register either a full JSON Schema object or a shorthand
/// object whose keys are parameter names. The returned value is always a valid
/// object schema for the tool registry.
#[must_use]
pub fn parameter_schema_from_lua(params: &Value) -> Value {
    if looks_like_json_schema(params) {
        return params.clone();
    }

    let mut properties = match params {
        Value::Object(map) => map.clone(),
        _ => Map::new(),
    };

    let required: Vec<Value> = properties
        .iter()
        .filter_map(|(name, definition)| {
            definition
                .get("required")
                .and_then(Value::as_bool)
                .filter(|required| *required)
                .map(|_| Value::String(name.clone()))
        })
        .collect();

    // Strip the non-standard "required" field from each property definition
    for (_name, definition) in properties.iter_mut() {
        if let Value::Object(ref mut map) = definition {
            map.remove("required");
        }
    }

    let mut schema = json!({
        "type": "object",
        "properties": properties,
    });

    if !required.is_empty() {
        schema["required"] = Value::Array(required);
    }

    schema
}

/// Convert a Lua tool's JSON result into imp's native `ToolOutput`.
///
/// Supported result forms:
/// - a plain string → single text block
/// - an object with `{ content, details, is_error }`
/// - `content` as a string, a single content block, or an array of blocks
pub fn tool_output_from_lua_result(result: Value) -> Result<ToolOutput> {
    match result {
        Value::Null => Ok(ToolOutput {
            content: Vec::new(),
            details: Value::Null,
            is_error: false,
        }),
        Value::String(text) => Ok(ToolOutput::text(text)),
        Value::Object(mut object) => {
            let is_error = object
                .remove("is_error")
                .or_else(|| object.remove("isError"))
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let details = object.remove("details").unwrap_or(Value::Null);
            let content = parse_lua_content(object.remove("content").unwrap_or(Value::Null))?;

            Ok(ToolOutput {
                content,
                details,
                is_error,
            })
        }
        other => Ok(ToolOutput::text(other.to_string())),
    }
}

fn looks_like_json_schema(params: &Value) -> bool {
    let Some(object) = params.as_object() else {
        return false;
    };

    object.contains_key("type")
        || object.contains_key("properties")
        || object.contains_key("required")
        || object.contains_key("anyOf")
        || object.contains_key("oneOf")
        || object.contains_key("allOf")
        || object.contains_key("$ref")
}

fn parse_lua_content(content: Value) -> Result<Vec<ContentBlock>> {
    match content {
        Value::Null => Ok(Vec::new()),
        Value::String(text) => Ok(vec![ContentBlock::Text { text }]),
        Value::Array(_) => serde_json::from_value(content).map_err(|error| {
            Error::Tool(format!("Lua tool returned invalid content blocks: {error}"))
        }),
        Value::Object(object) if object.contains_key("type") => {
            let block: ContentBlock =
                serde_json::from_value(Value::Object(object)).map_err(|error| {
                    Error::Tool(format!(
                        "Lua tool returned an invalid content block: {error}"
                    ))
                })?;
            Ok(vec![block])
        }
        Value::Object(object) => Ok(vec![ContentBlock::Text {
            text: Value::Object(object).to_string(),
        }]),
        other => Ok(vec![ContentBlock::Text {
            text: other.to_string(),
        }]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract_text(output: &ToolOutput) -> String {
        output
            .content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn parameter_schema_wraps_shorthand_definitions() {
        let schema = parameter_schema_from_lua(&json!({
            "name": { "type": "string", "required": true },
            "times": { "type": "number" }
        }));

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["name"]["type"], "string");
        assert_eq!(schema["properties"]["times"]["type"], "number");
        assert_eq!(schema["required"], json!(["name"]));
    }

    #[test]
    fn parameter_schema_preserves_full_schema() {
        let original = json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            },
            "required": ["path"]
        });

        assert_eq!(parameter_schema_from_lua(&original), original);
    }

    #[test]
    fn tool_output_parser_accepts_string_content() {
        let output = tool_output_from_lua_result(json!({
            "content": "hello from lua",
            "details": { "source": "test" },
            "is_error": true
        }))
        .unwrap();

        assert!(output.is_error);
        assert_eq!(output.details["source"], "test");
        assert_eq!(extract_text(&output), "hello from lua");
    }
}
