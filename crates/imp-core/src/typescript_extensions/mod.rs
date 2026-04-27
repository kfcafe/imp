mod bun_runner;
mod discovery;
mod pi_compat;
mod schema;

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;

use crate::error::{Error, Result};
use crate::tools::{Tool, ToolContext, ToolOutput, ToolRegistry};

use bun_runner::run_bun_bridge;
pub use discovery::{
    discover_typescript_extensions, discover_typescript_extensions_in, TypeScriptExtension,
    TypeScriptExtensionSource,
};
use schema::{normalize_parameters, RegisteredTool, TypeScriptExtensionCompatibility};

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
    let tools = inspect_registered_tools(extension)?;
    for tool in tools {
        registry.register(Arc::new(TypeScriptExtensionTool {
            extension: extension.clone(),
            name: tool.name,
            label: tool.label,
            description: describe_tool_with_compatibility(
                tool.description.unwrap_or_default(),
                &tool.compatibility,
            ),
            parameters: normalize_parameters(tool.parameters),
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
    name: String,
    label: Option<String>,
    description: String,
    parameters: serde_json::Value,
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
        false
    }

    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolOutput> {
        execute_registered_tool(&self.extension, &self.name, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

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

    #[test]
    fn loads_color_palette_fixture_when_available() {
        if Command::new("bun").arg("--version").output().is_err() {
            return;
        }

        let source = Path::new("/Users/asher/.pi/agent/extensions/color-palette");
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
            lua_tool_loader: None,
            mode: AgentMode::Full,
            read_max_lines: 0,
            turn_mana_review: Arc::new(std::sync::Mutex::new(TurnManaReviewAccumulator::default())),
            config: Arc::new(crate::config::Config::default()),
        }
    }
}
