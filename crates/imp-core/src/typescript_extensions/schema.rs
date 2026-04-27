use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct RegisteredTool {
    pub name: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
    #[serde(default)]
    pub compatibility: TypeScriptExtensionCompatibility,
}

#[derive(Debug, Default, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TypeScriptExtensionCompatibility {
    #[serde(default)]
    pub lifecycle_events: Vec<String>,
    #[serde(default)]
    pub stubbed_apis: Vec<String>,
    #[serde(default)]
    pub unsupported_apis: Vec<String>,
    #[serde(default)]
    pub custom_renderers: Vec<String>,
}

impl TypeScriptExtensionCompatibility {
    pub fn has_compatibility_debt(&self) -> bool {
        !self.stubbed_apis.is_empty()
            || !self.unsupported_apis.is_empty()
            || !self.custom_renderers.is_empty()
    }
}

pub(super) fn normalize_parameters(parameters: Option<serde_json::Value>) -> serde_json::Value {
    parameters.unwrap_or_else(|| serde_json::json!({ "type": "object", "properties": {} }))
}
