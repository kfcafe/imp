mod bun_runner;
mod discovery;
mod pi_compat;
mod schema;

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;

use crate::error::{Error, Result};
use crate::tools::{Tool, ToolContext, ToolOutput, ToolRegistry};

use bun_runner::{execute_manifest_tool, run_bun_bridge, TypeScriptExtensionCallRequest};
pub use discovery::{
    discover_typescript_extensions, discover_typescript_extensions_in, TypeScriptExtension,
    TypeScriptExtensionSource,
};
use schema::{
    normalize_parameters, RegisteredTool, TypeScriptExtensionCompatibility,
    TypeScriptExtensionManifest, TypeScriptExtensionToolManifest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeScriptExtensionStatus {
    pub extension_name: String,
    pub source: TypeScriptExtensionSource,
    pub tools: Vec<TypeScriptExtensionToolStatus>,
    pub state: TypeScriptExtensionLoadState,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeScriptExtensionToolStatus {
    pub name: String,
    pub compatibility: TypeScriptExtensionCompatibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeScriptExtensionLoadState {
    Executable,
    LoadedWithStubs,
    NeedsDependencies,
    Unsupported,
}

impl TypeScriptExtensionLoadState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Executable => "executable",
            Self::LoadedWithStubs => "loaded with compatibility stubs",
            Self::NeedsDependencies => "needs dependencies",
            Self::Unsupported => "unsupported",
        }
    }
}

pub fn load_typescript_extensions(project_dir: &Path, registry: &mut ToolRegistry) -> Result<()> {
    for extension in discover_typescript_extensions(project_dir) {
        load_typescript_extension(&extension, registry)?;
    }
    Ok(())
}

pub fn inspect_typescript_extension_statuses(project_dir: &Path) -> Vec<TypeScriptExtensionStatus> {
    discover_typescript_extensions(project_dir)
        .into_iter()
        .map(|extension| inspect_typescript_extension_status(&extension))
        .collect()
}

pub fn inspect_typescript_extension_status(
    extension: &TypeScriptExtension,
) -> TypeScriptExtensionStatus {
    match inspect_registered_tools(extension) {
        Ok(tools) => status_from_tools(extension, tools),
        Err(err) => {
            let message = err.to_string();
            TypeScriptExtensionStatus {
                extension_name: extension.name.clone(),
                source: extension.source.clone(),
                tools: Vec::new(),
                state: if is_missing_dependency_error(&message) {
                    TypeScriptExtensionLoadState::NeedsDependencies
                } else {
                    TypeScriptExtensionLoadState::Unsupported
                },
                message: Some(message),
            }
        }
    }
}

fn is_missing_dependency_error(message: &str) -> bool {
    message.contains("Cannot find package")
        || message.contains("Cannot find module")
        || message.contains("Module not found")
        || message.contains("error: Could not resolve")
}

fn status_from_tools(
    extension: &TypeScriptExtension,
    tools: Vec<RegisteredTool>,
) -> TypeScriptExtensionStatus {
    let tool_statuses: Vec<_> = tools
        .into_iter()
        .map(|tool| TypeScriptExtensionToolStatus {
            name: tool.name,
            compatibility: tool.compatibility,
        })
        .collect();

    let has_compatibility_debt = tool_statuses
        .iter()
        .any(|tool| tool.compatibility.has_compatibility_debt());
    TypeScriptExtensionStatus {
        extension_name: extension.name.clone(),
        source: extension.source.clone(),
        tools: tool_statuses,
        state: if has_compatibility_debt {
            TypeScriptExtensionLoadState::LoadedWithStubs
        } else {
            TypeScriptExtensionLoadState::Executable
        },
        message: None,
    }
}

fn load_typescript_extension(
    extension: &TypeScriptExtension,
    registry: &mut ToolRegistry,
) -> Result<()> {
    if extension.manifest_path.is_some() {
        return load_manifest_typescript_extension(extension, registry);
    }

    let tools = inspect_registered_tools(extension)?;
    for tool in tools {
        registry.register(Arc::new(TypeScriptExtensionTool {
            extension: extension.clone(),
            manifest: None,
            name: tool.name,
            label: tool.label,
            description: describe_tool_with_compatibility(
                tool.description.unwrap_or_default(),
                &tool.compatibility,
            ),
            parameters: normalize_parameters(tool.parameters),
            manifest_tool: None,
            manifest_tool_metadata: None,
        }));
    }
    Ok(())
}

fn load_manifest_typescript_extension(
    extension: &TypeScriptExtension,
    registry: &mut ToolRegistry,
) -> Result<()> {
    let manifest_path = extension
        .manifest_path
        .as_ref()
        .ok_or_else(|| Error::Tool("missing TypeScript extension manifest path".into()))?;
    let manifest_text = std::fs::read_to_string(manifest_path).map_err(Error::from)?;
    let manifest: TypeScriptExtensionManifest = serde_json::from_str(&manifest_text)?;
    manifest
        .validate()
        .map_err(|err| Error::Tool(err.to_string()))?;

    for tool in manifest.tools.iter().cloned() {
        registry.register(Arc::new(TypeScriptExtensionTool {
            extension: extension.clone(),
            manifest: Some(manifest.clone()),
            name: tool.name.clone(),
            label: tool.label.clone(),
            description: tool.description.clone(),
            parameters: tool.input_schema.clone(),
            manifest_tool: Some(tool.clone()),
            manifest_tool_metadata: Some(manifest.policy_metadata_for_tool(&tool)),
        }));
    }
    Ok(())
}

fn inspect_registered_tools(extension: &TypeScriptExtension) -> Result<Vec<RegisteredTool>> {
    let output = run_bun_bridge(extension, "inspect", serde_json::Value::Null)?;
    serde_json::from_value(output).map_err(Error::from)
}

fn describe_tool_with_compatibility(
    description: String,
    compatibility: &schema::TypeScriptExtensionCompatibility,
) -> String {
    let mut notes = Vec::new();
    if !compatibility.stubbed_apis.is_empty() {
        notes.push(format!(
            "stubbed APIs: {}",
            compatibility.stubbed_apis.join(", ")
        ));
    }
    if !compatibility.unsupported_apis.is_empty() {
        notes.push(format!(
            "unsupported APIs: {}",
            compatibility.unsupported_apis.join(", ")
        ));
    }
    if !compatibility.custom_renderers.is_empty() {
        notes.push(format!(
            "custom renderers detected but not mapped: {}",
            compatibility.custom_renderers.join(", ")
        ));
    }
    if !compatibility.lifecycle_events.is_empty() {
        notes.push(format!(
            "lifecycle hooks: {}",
            compatibility.lifecycle_events.join(", ")
        ));
    }

    if notes.is_empty() {
        description
    } else if description.is_empty() {
        format!("TypeScript extension compatibility: {}", notes.join("; "))
    } else {
        format!(
            "{description}\n\nTypeScript extension compatibility: {}",
            notes.join("; ")
        )
    }
}

fn mediated_extension_env(tool: &TypeScriptExtensionToolManifest) -> Vec<(String, String)> {
    tool.env
        .iter()
        .filter_map(|key| std::env::var(key).ok().map(|value| (key.clone(), value)))
        .collect()
}

fn execute_registered_tool(
    extension: &TypeScriptExtension,
    tool_name: &str,
    params: serde_json::Value,
) -> Result<ToolOutput> {
    let payload = serde_json::json!({
        "tool": tool_name,
        "params": params,
    });
    let output = run_bun_bridge(extension, "execute", payload)?;
    let content = output
        .get("content")
        .and_then(|content| content.as_array())
        .and_then(|content| content.first())
        .and_then(|block| block.get("text"))
        .and_then(|text| text.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| output.to_string());
    let mut tool_output = ToolOutput::text(content);
    tool_output.details = output
        .get("details")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    Ok(tool_output)
}

struct TypeScriptExtensionTool {
    extension: TypeScriptExtension,
    manifest: Option<TypeScriptExtensionManifest>,
    name: String,
    label: Option<String>,
    description: String,
    parameters: serde_json::Value,
    manifest_tool: Option<TypeScriptExtensionToolManifest>,
    manifest_tool_metadata: Option<crate::reference_monitor::ToolMetadata>,
}

#[async_trait]
impl Tool for TypeScriptExtensionTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.name)
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> serde_json::Value {
        self.parameters.clone()
    }

    fn is_readonly(&self) -> bool {
        self.manifest_tool.as_ref().is_some_and(|tool| {
            matches!(
                tool.side_effect,
                schema::TypeScriptExtensionSideEffect::ReadOnly
            )
        })
    }

    fn policy_metadata(&self) -> crate::reference_monitor::ToolMetadata {
        let mut metadata = self.manifest_tool_metadata.clone().unwrap_or_else(|| {
            let mut metadata = crate::reference_monitor::ToolMetadata::new(
                self.name(),
                crate::reference_monitor::ToolActionKind::Extension,
            );
            metadata.extension = true;
            metadata.extension_id = Some(self.extension.name.clone());
            metadata.requires_approval = true;
            metadata
        });
        metadata.name = self.name().to_string();
        metadata
    }

    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        if let (Some(manifest), Some(tool)) = (&self.manifest, &self.manifest_tool) {
            let result = execute_manifest_tool(TypeScriptExtensionCallRequest {
                extension: &self.extension,
                manifest,
                tool,
                arguments: params,
                env: mediated_extension_env(tool),
                cwd: ctx.cwd,
                run_id: None,
            })?;
            let output = ToolOutput {
                content: result.content,
                details: result.details.unwrap_or(serde_json::Value::Null),
                is_error: result.is_error,
            };
            return Ok(output);
        }

        execute_registered_tool(&self.extension, &self.name, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn discovers_manifest_extension_packages() {
        let temp = TempDir::new().unwrap();
        let extension_dir = temp.path().join(".imp").join("extensions").join("example");
        std::fs::create_dir_all(&extension_dir).unwrap();
        std::fs::write(
            extension_dir.join("imp.extension.json"),
            r#"{
                "schemaVersion": 1,
                "id": "example.echo",
                "name": "Example Echo",
                "version": "0.1.0",
                "runtime": {
                    "kind": "typescript-subprocess",
                    "command": "bun",
                    "protocol": "one-shot-json"
                },
                "tools": [{
                    "name": "example_echo",
                    "description": "Echo text.",
                    "inputSchema": { "type": "object", "properties": {} },
                    "sideEffect": "read-only",
                    "resourceScope": { "kind": "none" },
                    "network": "none"
                }]
            }"#,
        )
        .unwrap();

        let extensions = discover_typescript_extensions(temp.path());
        let extension = extensions
            .iter()
            .find(|extension| extension.name == "example")
            .expect("manifest extension discovered");
        assert_eq!(
            extension.manifest_path.as_deref(),
            Some(extension_dir.join("imp.extension.json").as_path())
        );
    }

    #[tokio::test]
    async fn executes_manifest_one_shot_json_tool() {
        let temp = TempDir::new().unwrap();
        let extension_dir = temp.path().join(".imp").join("extensions").join("example");
        std::fs::create_dir_all(&extension_dir).unwrap();
        let script = extension_dir.join("tool.sh");
        std::fs::write(
            &script,
            r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"id":"call-1","type":"tool_result","content":[{"type":"text","text":"hello from ts"}],"details":{"ok":true}}'
"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&script).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&script, permissions).unwrap();
        }
        std::fs::write(
            extension_dir.join("imp.extension.json"),
            format!(
                r#"{{
                    "schemaVersion": 1,
                    "id": "example.echo",
                    "name": "Example Echo",
                    "version": "0.1.0",
                    "runtime": {{
                        "kind": "typescript-subprocess",
                        "command": "{}",
                        "protocol": "one-shot-json"
                    }},
                    "tools": [{{
                        "name": "example_echo",
                        "description": "Echo text.",
                        "inputSchema": {{ "type": "object", "properties": {{}} }},
                        "sideEffect": "read-only",
                        "resourceScope": {{ "kind": "none" }},
                        "network": "none"
                    }}]
                }}"#,
                script.display()
            ),
        )
        .unwrap();

        let mut registry = ToolRegistry::new();
        load_typescript_extensions(temp.path(), &mut registry).unwrap();
        let tool = registry
            .get("example_echo")
            .expect("manifest tool registered");
        let output = tool
            .execute(
                "call-1",
                serde_json::json!({}),
                test_tool_context(temp.path()),
            )
            .await
            .unwrap();
        assert!(!output.is_error);
        let Some(imp_llm::ContentBlock::Text { text }) = output.content.first() else {
            panic!("expected text content block: {:?}", output.content);
        };
        assert_eq!(text, "hello from ts");
        assert_eq!(output.details["ok"], true);
        assert!(tool.is_readonly());
        assert!(tool.policy_metadata().extension);
    }

    #[test]
    fn discovers_repository_example_typescript_extension_manifest() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("workspace root");
        let manifest_path = repo_root.join("extensions/example/imp.extension.json");
        let manifest_text = std::fs::read_to_string(&manifest_path).unwrap();
        let manifest: TypeScriptExtensionManifest = serde_json::from_str(&manifest_text).unwrap();
        manifest.validate().unwrap();
        assert_eq!(manifest.id, "example.typescript");
        assert!(manifest
            .tools
            .iter()
            .any(|tool| tool.name == "example_echo"));
        let write_tool = manifest
            .tools
            .iter()
            .find(|tool| tool.name == "example_write_demo")
            .expect("workspace-write demo tool");
        let metadata = manifest.policy_metadata_for_tool(write_tool);
        assert!(metadata.workspace_write);
        assert!(metadata.requires_approval);
    }

    #[test]
    fn manifest_extension_tool_policy_metadata_uses_registered_tool_name() {
        let temp = TempDir::new().unwrap();
        let extension_dir = temp.path().join(".imp").join("extensions").join("example");
        std::fs::create_dir_all(&extension_dir).unwrap();
        std::fs::write(
            extension_dir.join("imp.extension.json"),
            r#"{
                "schemaVersion": 1,
                "id": "example.echo",
                "name": "Example Echo",
                "version": "0.1.0",
                "runtime": {
                    "kind": "typescript-subprocess",
                    "command": "python3",
                    "protocol": "one-shot-json"
                },
                "tools": [{
                    "name": "example_echo",
                    "description": "Echo text.",
                    "inputSchema": { "type": "object", "properties": {} },
                    "sideEffect": "read-only",
                    "resourceScope": { "kind": "none" },
                    "network": "none"
                }]
            }"#,
        )
        .unwrap();
        let mut registry = ToolRegistry::new();
        load_typescript_extensions(temp.path(), &mut registry).unwrap();
        let tool = registry.get("example_echo").expect("tool registered");
        let metadata = tool.policy_metadata();
        assert_eq!(metadata.name, "example_echo");
        assert_eq!(
            metadata.action_kind,
            crate::reference_monitor::ToolActionKind::Read
        );
        assert!(metadata.readonly);
        assert!(metadata.extension);
        assert_eq!(metadata.extension_id.as_deref(), Some("example.echo"));
        assert_eq!(metadata.manifest_version.as_deref(), Some("0.1.0"));
        assert!(!metadata.requires_approval);
    }

    #[test]
    fn discovers_local_and_imported_extensions() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join(".imp").join("extensions");
        std::fs::create_dir_all(root.join("pi").join("echo")).unwrap();
        std::fs::write(root.join("pi").join("echo").join("index.ts"), "").unwrap();
        std::fs::write(root.join("single.ts"), "").unwrap();

        let extensions = discover_typescript_extensions(temp.path());
        let names: Vec<_> = extensions.iter().map(|ext| ext.name.as_str()).collect();
        assert!(names.contains(&"echo"));
        assert!(names.contains(&"single"));
    }

    #[test]
    fn reports_missing_bun_for_runtime_execution() {
        if Command::new("bun").arg("--version").output().is_ok() {
            return;
        }

        let temp = TempDir::new().unwrap();
        let extension_dir = temp
            .path()
            .join(".imp")
            .join("extensions")
            .join("pi")
            .join("echo");
        std::fs::create_dir_all(&extension_dir).unwrap();
        std::fs::write(
            extension_dir.join("index.ts"),
            "export default function(pi) { pi.registerTool({ name: 'echo_ts', execute() {} }); }\n",
        )
        .unwrap();
        let mut registry = ToolRegistry::new();
        let error = load_typescript_extensions(temp.path(), &mut registry).unwrap_err();
        assert!(error
            .to_string()
            .contains("TypeScript extensions require Bun"));
    }

    #[test]
    fn loads_simple_register_tool_extension() {
        if Command::new("bun").arg("--version").output().is_err() {
            return;
        }

        let temp = TempDir::new().unwrap();
        let extension_dir = temp
            .path()
            .join(".imp")
            .join("extensions")
            .join("pi")
            .join("echo");
        std::fs::create_dir_all(&extension_dir).unwrap();
        std::fs::write(
            extension_dir.join("index.ts"),
            r#"
import { Type } from '@sinclair/typebox';
export default function(pi) {
  pi.registerTool({
    name: 'echo_ts',
    label: 'Echo TS',
    description: 'Echo a message from TypeScript',
    parameters: Type.Object({ message: Type.String({ description: 'Message to echo' }) }),
    async execute(_id, params) {
      return { content: [{ type: 'text', text: `echo:${params.message}` }], details: { ok: true } };
    }
  });
}
"#,
        )
        .unwrap();

        let mut registry = ToolRegistry::new();
        load_typescript_extensions(temp.path(), &mut registry).unwrap();
        assert!(registry.get("echo_ts").is_some());
    }

    #[test]
    fn classifies_missing_dependency_as_needs_dependencies() {
        if Command::new("bun").arg("--version").output().is_err() {
            return;
        }

        let temp = TempDir::new().unwrap();
        let extension_dir = temp
            .path()
            .join(".imp")
            .join("extensions")
            .join("pi")
            .join("missing-dep");
        std::fs::create_dir_all(&extension_dir).unwrap();
        std::fs::write(
            extension_dir.join("index.ts"),
            r#"
import missingDependency from 'definitely-missing-imp-extension-dependency';
export default function(pi) {
  pi.registerTool({ name: 'missing_dep', execute() { return missingDependency; } });
}
"#,
        )
        .unwrap();

        let statuses = inspect_typescript_extension_statuses(temp.path());
        let status = statuses
            .iter()
            .find(|status| status.extension_name == "missing-dep")
            .expect("missing-dep status");
        assert_eq!(
            status.state,
            TypeScriptExtensionLoadState::NeedsDependencies
        );
        assert!(status
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("definitely-missing-imp-extension-dependency"));
    }

    #[test]
    fn classifies_dynamic_tools_as_loaded_with_stubs() {
        if Command::new("bun").arg("--version").output().is_err() {
            return;
        }

        let temp = TempDir::new().unwrap();
        write_dynamic_tools_fixture(temp.path());

        let statuses = inspect_typescript_extension_statuses(temp.path());
        let status = statuses
            .iter()
            .find(|status| status.extension_name == "dynamic-tools")
            .expect("dynamic-tools status");
        assert_eq!(status.state, TypeScriptExtensionLoadState::LoadedWithStubs);
        assert!(status.tools.iter().any(|tool| tool.name == "echo_session"
            && tool
                .compatibility
                .stubbed_apis
                .contains(&"registerCommand".to_string())));
    }

    fn write_dynamic_tools_fixture(project_dir: &Path) {
        let extension_dir = project_dir
            .join(".imp")
            .join("extensions")
            .join("pi")
            .join("dynamic-tools");
        std::fs::create_dir_all(&extension_dir).unwrap();
        std::fs::write(extension_dir.join("index.ts"), DYNAMIC_TOOLS_FIXTURE).unwrap();
    }

    const DYNAMIC_TOOLS_FIXTURE: &str = r#"
import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { Type } from "@sinclair/typebox";

const ECHO_PARAMS = Type.Object({
  message: Type.String({ description: "Message to echo" }),
});

export default function dynamicToolsExtension(pi: ExtensionAPI) {
  const registeredToolNames = new Set<string>();

  const registerEchoTool = (name: string, label: string, prefix: string): boolean => {
    if (registeredToolNames.has(name)) return false;
    registeredToolNames.add(name);
    pi.registerTool({
      name,
      label,
      description: `Echo a message with prefix: ${prefix}`,
      parameters: ECHO_PARAMS,
      async execute(_toolCallId, params) {
        return {
          content: [{ type: "text", text: `${prefix}${params.message}` }],
          details: { tool: name, prefix },
        };
      },
    });
    return true;
  };

  pi.on("session_start", (_event, ctx) => {
    registerEchoTool("echo_session", "Echo Session", "[session] ");
    ctx.ui.notify("Registered dynamic tool: echo_session", "info");
  });

  pi.registerCommand("add-echo-tool", {
    description: "Register a new echo tool dynamically: /add-echo-tool <tool_name>",
    handler: async () => {},
  });
}
"#;

    #[test]
    fn loads_official_dynamic_tools_fixture() {
        if Command::new("bun").arg("--version").output().is_err() {
            return;
        }

        let temp = TempDir::new().unwrap();
        write_dynamic_tools_fixture(temp.path());

        let mut registry = ToolRegistry::new();
        load_typescript_extensions(temp.path(), &mut registry).unwrap();
        let tool = registry
            .get("echo_session")
            .expect("dynamic tool registered");
        assert!(tool.description().contains("stubbed APIs: registerCommand"));
        assert!(tool
            .description()
            .contains("lifecycle hooks: session_start"));
        let output = futures::executor::block_on(tool.execute(
            "call_echo_session",
            serde_json::json!({ "message": "hello" }),
            test_tool_context(temp.path()),
        ))
        .unwrap();
        assert_eq!(output.text_content(), Some("[session] hello"));
    }

    #[tokio::test]
    async fn loads_example_manifest_extension_fixture() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("extensions")
            .join("example");
        if !source.exists() || Command::new("node").arg("--version").output().is_err() {
            return;
        }

        let temp = TempDir::new().unwrap();
        let extension_dir = temp.path().join(".imp").join("extensions").join("example");
        copy_dir_for_test(&source, &extension_dir).unwrap();

        let mut registry = ToolRegistry::new();
        load_typescript_extensions(temp.path(), &mut registry).unwrap();
        let tool = registry
            .get("example_echo")
            .expect("example_echo registered");
        assert!(tool.is_readonly());
        assert_eq!(
            tool.policy_metadata().extension_id.as_deref(),
            Some("example.typescript")
        );

        let output = futures::executor::block_on(tool.execute(
            "call_example_echo",
            serde_json::json!({ "text": "hello example" }),
            test_tool_context(temp.path()),
        ))
        .unwrap();
        assert_eq!(output.text_content(), Some("hello example"));
        assert_eq!(output.details["tool"], "example_echo");
    }

    #[test]
    fn loads_color_palette_fixture_when_available() {
        if Command::new("bun").arg("--version").output().is_err() {
            return;
        }

        let source = Path::new("/Users/test/.pi/agent/extensions/color-palette");
        if !source.exists() {
            return;
        }

        let temp = TempDir::new().unwrap();
        let extension_dir = temp
            .path()
            .join(".imp")
            .join("extensions")
            .join("pi")
            .join("color-palette");
        copy_dir_for_test(source, &extension_dir).unwrap();

        let mut registry = ToolRegistry::new();
        load_typescript_extensions(temp.path(), &mut registry).unwrap();
        let tool = registry
            .get("color_palette")
            .expect("color_palette tool registered");

        let output = futures::executor::block_on(tool.execute(
            "call_color_palette",
            serde_json::json!({
                "action": "harmony",
                "color": "#3b82f6",
                "mode": "complementary"
            }),
            test_tool_context(temp.path()),
        ))
        .unwrap();

        let text = output.text_content().unwrap_or_default();
        assert!(text.contains("harmony"));
        assert!(text.contains("#"));
    }

    fn copy_dir_for_test(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let source_path = entry.path();
            let dest_path = dst.join(entry.file_name());
            if source_path.is_dir() {
                copy_dir_for_test(&source_path, &dest_path)?;
            } else {
                std::fs::copy(&source_path, &dest_path)?;
            }
        }
        Ok(())
    }

    fn test_tool_context(cwd: &Path) -> ToolContext {
        use crate::config::AgentMode;
        use crate::mana_review::TurnManaReviewAccumulator;
        use crate::tools::{CheckpointState, FileCache, FileTracker};
        use crate::ui::NullInterface;
        use std::sync::atomic::AtomicBool;
        use tokio::sync::mpsc;

        let (update_tx, _) = mpsc::channel(1);
        let (command_tx, _) = mpsc::channel(1);

        ToolContext {
            cwd: cwd.to_path_buf(),
            cancelled: Arc::new(AtomicBool::new(false)),
            update_tx,
            command_tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            checkpoint_state: Arc::new(CheckpointState::default()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: AgentMode::Full,
            read_max_lines: 0,
            turn_mana_review: Arc::new(std::sync::Mutex::new(TurnManaReviewAccumulator::default())),
            config: Arc::new(crate::config::Config::default()),
            run_policy: Default::default(),
            supporting_provenance: Vec::new(),
        }
    }
}
