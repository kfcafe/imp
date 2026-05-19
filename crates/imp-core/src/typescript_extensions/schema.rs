use serde::{Deserialize, Serialize};

use crate::reference_monitor::{ResourceScope, ToolActionKind, ToolMetadata};

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

pub const TYPESCRIPT_EXTENSION_MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeScriptExtensionManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub runtime: TypeScriptExtensionRuntimeManifest,
    #[serde(default)]
    pub tools: Vec<TypeScriptExtensionToolManifest>,
}

impl TypeScriptExtensionManifest {
    pub fn validate(&self) -> Result<(), TypeScriptExtensionManifestError> {
        let mut errors = Vec::new();
        if self.schema_version != TYPESCRIPT_EXTENSION_MANIFEST_SCHEMA_VERSION {
            errors.push(format!(
                "unsupported schemaVersion {}; expected {}",
                self.schema_version, TYPESCRIPT_EXTENSION_MANIFEST_SCHEMA_VERSION
            ));
        }
        validate_non_empty("id", &self.id, &mut errors);
        validate_non_empty("name", &self.name, &mut errors);
        validate_non_empty("version", &self.version, &mut errors);
        if self.runtime.command.trim().is_empty() {
            errors.push("runtime.command is required".into());
        }
        if self.runtime.timeout_ms == 0 {
            errors.push("runtime.timeoutMs must be greater than zero".into());
        }
        if self.runtime.output_limit_bytes == 0 {
            errors.push("runtime.outputLimitBytes must be greater than zero".into());
        }
        if self.tools.is_empty() {
            errors.push("at least one tool is required".into());
        }

        let mut seen_tools = std::collections::BTreeSet::new();
        for tool in &self.tools {
            validate_tool(tool, &mut seen_tools, &mut errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(TypeScriptExtensionManifestError { errors })
        }
    }
    pub fn policy_metadata_for_tool(&self, tool: &TypeScriptExtensionToolManifest) -> ToolMetadata {
        let mut metadata = ToolMetadata::new(&tool.name, tool.action_kind());
        metadata.readonly = matches!(tool.side_effect, TypeScriptExtensionSideEffect::ReadOnly);
        metadata.workspace_write = matches!(
            tool.side_effect,
            TypeScriptExtensionSideEffect::WorkspaceWrite
                | TypeScriptExtensionSideEffect::Destructive
        );
        metadata.external_side_effect = !metadata.readonly;
        metadata.network = !matches!(tool.network, TypeScriptExtensionNetwork::None);
        metadata.secrets = !tool.secrets.is_empty();
        metadata.extension = true;
        metadata.default_requires_approval =
            !metadata.readonly || metadata.network || metadata.secrets;
        metadata.requires_approval = metadata.default_requires_approval;
        metadata.extension_id = Some(self.id.clone());
        metadata.manifest_version = Some(self.version.clone());
        metadata.resource_scopes = tool.resource_scopes();
        metadata
    }
}

impl TypeScriptExtensionToolManifest {
    pub fn action_kind(&self) -> ToolActionKind {
        match self.side_effect {
            TypeScriptExtensionSideEffect::ReadOnly => ToolActionKind::Read,
            TypeScriptExtensionSideEffect::WorkspaceWrite => ToolActionKind::Write,
            TypeScriptExtensionSideEffect::ExternalWrite
            | TypeScriptExtensionSideEffect::Destructive => ToolActionKind::Extension,
        }
    }

    pub fn resource_scopes(&self) -> Vec<ResourceScope> {
        let mut scopes = match &self.resource_scope {
            TypeScriptExtensionResourceScope::None => vec![ResourceScope::Extension {
                id: self.name.clone(),
            }],
            TypeScriptExtensionResourceScope::Workspace { read, write } => read
                .iter()
                .map(|path| ResourceScope::Directory { path: path.into() })
                .chain(
                    write
                        .iter()
                        .map(|path| ResourceScope::File { path: path.into() }),
                )
                .collect::<Vec<_>>(),
            TypeScriptExtensionResourceScope::Command { program } => {
                vec![ResourceScope::Command {
                    program: program.clone(),
                }]
            }
            TypeScriptExtensionResourceScope::Network { hosts } => hosts
                .iter()
                .map(|host| ResourceScope::Network {
                    host: Some(host.clone()),
                })
                .collect(),
        };
        for secret in &self.secrets {
            scopes.push(ResourceScope::Secret {
                name: Some(secret.clone()),
            });
        }
        if scopes.is_empty() {
            scopes.push(ResourceScope::Extension {
                id: self.name.clone(),
            });
        }
        scopes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeScriptExtensionRuntimeManifest {
    pub kind: TypeScriptExtensionRuntimeKind,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub protocol: TypeScriptExtensionProtocol,
    #[serde(default = "default_runtime_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_output_limit_bytes")]
    pub output_limit_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TypeScriptExtensionRuntimeKind {
    #[serde(rename = "typescript-subprocess")]
    TypeScriptSubprocess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TypeScriptExtensionProtocol {
    JsonLines,
    OneShotJson,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeScriptExtensionToolManifest {
    pub name: String,
    pub label: Option<String>,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub side_effect: TypeScriptExtensionSideEffect,
    pub resource_scope: TypeScriptExtensionResourceScope,
    pub network: TypeScriptExtensionNetwork,
    #[serde(default)]
    pub secrets: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default = "default_tool_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_output_limit_bytes")]
    pub output_limit_bytes: u64,
    #[serde(default)]
    pub policy_tags: Vec<String>,
    #[serde(default)]
    pub verifier_tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TypeScriptExtensionSideEffect {
    ReadOnly,
    WorkspaceWrite,
    ExternalWrite,
    Destructive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TypeScriptExtensionResourceScope {
    None,
    Workspace {
        #[serde(default)]
        read: Vec<String>,
        #[serde(default)]
        write: Vec<String>,
    },
    Command {
        program: String,
    },
    Network {
        hosts: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TypeScriptExtensionNetwork {
    None,
    DeclaredHosts { hosts: Vec<String> },
    Unrestricted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeScriptExtensionManifestError {
    pub errors: Vec<String>,
}

impl std::fmt::Display for TypeScriptExtensionManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid TypeScript extension manifest: {}",
            self.errors.join("; ")
        )
    }
}

impl std::error::Error for TypeScriptExtensionManifestError {}

fn validate_tool(
    tool: &TypeScriptExtensionToolManifest,
    seen_tools: &mut std::collections::BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    validate_non_empty("tools[].name", &tool.name, errors);
    validate_non_empty("tools[].description", &tool.description, errors);
    if !is_valid_tool_name(&tool.name) {
        errors.push(format!(
            "tool name '{}' must contain only ascii letters, digits, '_' or '-' and must start with a letter",
            tool.name
        ));
    }
    if !seen_tools.insert(tool.name.clone()) {
        errors.push(format!("duplicate tool name '{}'", tool.name));
    }
    if !is_object_schema(&tool.input_schema) {
        errors.push(format!(
            "tool '{}' inputSchema must be an object schema",
            tool.name
        ));
    }
    if tool.timeout_ms == 0 {
        errors.push(format!(
            "tool '{}' timeoutMs must be greater than zero",
            tool.name
        ));
    }
    if tool.output_limit_bytes == 0 {
        errors.push(format!(
            "tool '{}' outputLimitBytes must be greater than zero",
            tool.name
        ));
    }
    match (&tool.side_effect, &tool.resource_scope) {
        (TypeScriptExtensionSideEffect::ReadOnly, _) => {}
        (TypeScriptExtensionSideEffect::WorkspaceWrite, TypeScriptExtensionResourceScope::Workspace { write, .. })
            if !write.is_empty() => {}
        (TypeScriptExtensionSideEffect::WorkspaceWrite, _) => errors.push(format!(
            "tool '{}' sideEffect workspace-write requires resourceScope.kind=workspace with write globs",
            tool.name
        )),
        (TypeScriptExtensionSideEffect::ExternalWrite | TypeScriptExtensionSideEffect::Destructive, TypeScriptExtensionResourceScope::None) => errors.push(format!(
            "tool '{}' sideEffect {:?} requires an explicit non-none resourceScope",
            tool.name, tool.side_effect
        )),
        _ => {}
    }
    if let TypeScriptExtensionNetwork::DeclaredHosts { hosts } = &tool.network {
        if hosts.is_empty() {
            errors.push(format!(
                "tool '{}' network declared-hosts requires hosts",
                tool.name
            ));
        }
    }
}

fn validate_non_empty(field: &str, value: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{field} is required"));
    }
}

fn is_valid_tool_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn is_object_schema(schema: &serde_json::Value) -> bool {
    schema
        .get("type")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|kind| kind == "object")
}

fn default_runtime_timeout_ms() -> u64 {
    10_000
}

fn default_tool_timeout_ms() -> u64 {
    5_000
}

fn default_output_limit_bytes() -> u64 {
    64 * 1024
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_manifest() -> TypeScriptExtensionManifest {
        TypeScriptExtensionManifest {
            schema_version: TYPESCRIPT_EXTENSION_MANIFEST_SCHEMA_VERSION,
            id: "example.echo".into(),
            name: "Example Echo".into(),
            version: "0.1.0".into(),
            runtime: TypeScriptExtensionRuntimeManifest {
                kind: TypeScriptExtensionRuntimeKind::TypeScriptSubprocess,
                command: "bun".into(),
                args: vec!["run".into(), "src/tool.ts".into()],
                protocol: TypeScriptExtensionProtocol::OneShotJson,
                timeout_ms: 10_000,
                output_limit_bytes: 65_536,
            },
            tools: vec![TypeScriptExtensionToolManifest {
                name: "example_echo".into(),
                label: Some("Echo".into()),
                description: "Echo text".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": { "text": { "type": "string" } },
                    "required": ["text"],
                    "additionalProperties": false
                }),
                side_effect: TypeScriptExtensionSideEffect::ReadOnly,
                resource_scope: TypeScriptExtensionResourceScope::None,
                network: TypeScriptExtensionNetwork::None,
                secrets: Vec::new(),
                env: Vec::new(),
                timeout_ms: 5_000,
                output_limit_bytes: 65_536,
                policy_tags: vec!["extension".into()],
                verifier_tags: Vec::new(),
            }],
        }
    }

    #[test]
    fn typescript_extension_manifest_accepts_minimal_readonly_tool() {
        let manifest = valid_manifest();
        manifest.validate().unwrap();
        let encoded = serde_json::to_string(&manifest).unwrap();
        let decoded: TypeScriptExtensionManifest = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, manifest);
    }

    #[test]
    fn typescript_extension_manifest_applies_safe_defaults() {
        let decoded: TypeScriptExtensionManifest = serde_json::from_value(json!({
            "schemaVersion": 1,
            "id": "example.defaults",
            "name": "Defaults",
            "version": "0.1.0",
            "runtime": {
                "kind": "typescript-subprocess",
                "command": "bun",
                "protocol": "one-shot-json"
            },
            "tools": [{
                "name": "default_tool",
                "description": "Uses defaults",
                "inputSchema": { "type": "object", "properties": {} },
                "sideEffect": "read-only",
                "resourceScope": { "kind": "none" },
                "network": "none"
            }]
        }))
        .unwrap();
        decoded.validate().unwrap();
        assert!(decoded.tools[0].secrets.is_empty());
        assert!(decoded.tools[0].env.is_empty());
        assert_eq!(decoded.tools[0].timeout_ms, 5_000);
        assert_eq!(decoded.tools[0].output_limit_bytes, 65_536);
    }

    #[test]
    fn typescript_extension_manifest_rejects_invalid_capabilities() {
        let mut manifest = valid_manifest();
        manifest.tools[0].name = "1bad".into();
        manifest.tools[0].side_effect = TypeScriptExtensionSideEffect::WorkspaceWrite;
        manifest.tools[0].resource_scope = TypeScriptExtensionResourceScope::None;
        manifest.tools[0].input_schema = json!({ "type": "string" });
        manifest.tools.push(manifest.tools[0].clone());

        let error = manifest.validate().unwrap_err();
        let message = error.to_string();
        assert!(message.contains("tool name '1bad'"));
        assert!(message.contains("duplicate tool name '1bad'"));
        assert!(message.contains("inputSchema must be an object schema"));
        assert!(message.contains("workspace-write requires"));
    }

    #[test]
    fn typescript_extension_manifest_rejects_declared_network_without_hosts() {
        let mut manifest = valid_manifest();
        manifest.tools[0].network = TypeScriptExtensionNetwork::DeclaredHosts { hosts: Vec::new() };
        let error = manifest.validate().unwrap_err();
        assert!(error.to_string().contains("declared-hosts requires hosts"));
    }

    #[test]
    fn typescript_extension_manifest_maps_policy_metadata() {
        let mut manifest = valid_manifest();
        manifest.tools[0].side_effect = TypeScriptExtensionSideEffect::WorkspaceWrite;
        manifest.tools[0].resource_scope = TypeScriptExtensionResourceScope::Workspace {
            read: vec!["docs/**".into()],
            write: vec!["docs/generated/**".into()],
        };

        let metadata = manifest.policy_metadata_for_tool(&manifest.tools[0]);
        assert_eq!(metadata.name, "example_echo");
        assert_eq!(metadata.action_kind, ToolActionKind::Write);
        assert!(!metadata.readonly);
        assert!(metadata.workspace_write);
        assert!(metadata.extension);
        assert_eq!(metadata.extension_id.as_deref(), Some("example.echo"));
        assert_eq!(metadata.manifest_version.as_deref(), Some("0.1.0"));
        assert!(metadata.requires_approval);
        assert!(metadata.resource_scopes.iter().any(|scope| matches!(
            scope,
            ResourceScope::File { path } if path == std::path::Path::new("docs/generated/**")
        )));
    }
}
