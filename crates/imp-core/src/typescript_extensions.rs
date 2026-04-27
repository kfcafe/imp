use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::tools::{Tool, ToolContext, ToolOutput, ToolRegistry};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeScriptExtension {
    pub name: String,
    pub root: PathBuf,
    pub entrypoint: PathBuf,
    pub source: TypeScriptExtensionSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeScriptExtensionSource {
    Local,
    Imported(String),
}

#[derive(Debug, Deserialize)]
struct PackageJson {
    pi: Option<PiPackage>,
}

#[derive(Debug, Deserialize)]
struct PiPackage {
    extensions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct RegisteredTool {
    name: String,
    label: Option<String>,
    description: Option<String>,
    parameters: Option<serde_json::Value>,
}

pub fn discover_typescript_extensions(project_dir: &Path) -> Vec<TypeScriptExtension> {
    discover_typescript_extensions_in(&project_dir.join(".imp").join("extensions"))
}

pub fn discover_typescript_extensions_in(root: &Path) -> Vec<TypeScriptExtension> {
    let mut extensions = Vec::new();
    discover_extension_entries(root, TypeScriptExtensionSource::Local, &mut extensions);
    extensions.sort_by(|a, b| a.name.cmp(&b.name));
    extensions
}

fn discover_extension_entries(
    dir: &Path,
    source: TypeScriptExtensionSource,
    extensions: &mut Vec<TypeScriptExtension>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if path.extension().and_then(|ext| ext.to_str()) == Some("ts") {
                if let Some(name) = file_stem_string(&path) {
                    extensions.push(TypeScriptExtension {
                        name,
                        root: dir.to_path_buf(),
                        entrypoint: path,
                        source: source.clone(),
                    });
                }
            }
            continue;
        }

        if !path.is_dir() {
            continue;
        }

        if let Some(extension) = detect_package_dir(&path, source.clone()) {
            extensions.push(extension);
        } else if matches!(source, TypeScriptExtensionSource::Local) {
            let Some(namespace) = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
            else {
                continue;
            };
            discover_extension_entries(
                &path,
                TypeScriptExtensionSource::Imported(namespace),
                extensions,
            );
        }
    }
}

fn detect_package_dir(
    dir: &Path,
    source: TypeScriptExtensionSource,
) -> Option<TypeScriptExtension> {
    let name = dir.file_name()?.to_string_lossy().to_string();
    let package_json = dir.join("package.json");

    if package_json.exists() {
        if let Ok(content) = std::fs::read_to_string(&package_json) {
            if let Ok(package) = serde_json::from_str::<PackageJson>(&content) {
                if let Some(entrypoint) = package
                    .pi
                    .and_then(|pi| pi.extensions)
                    .and_then(|mut extensions| extensions.drain(..).next())
                {
                    return Some(TypeScriptExtension {
                        name,
                        root: dir.to_path_buf(),
                        entrypoint: dir.join(entrypoint),
                        source,
                    });
                }
            }
        }
    }

    let entrypoint = dir.join("index.ts");
    entrypoint.exists().then_some(TypeScriptExtension {
        name,
        root: dir.to_path_buf(),
        entrypoint,
        source,
    })
}

pub fn load_typescript_extensions(project_dir: &Path, registry: &mut ToolRegistry) -> Result<()> {
    for extension in discover_typescript_extensions(project_dir) {
        load_typescript_extension(&extension, registry)?;
    }
    Ok(())
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
            description: tool.description.unwrap_or_default(),
            parameters: normalize_parameters(tool.parameters),
        }));
    }
    Ok(())
}

fn inspect_registered_tools(extension: &TypeScriptExtension) -> Result<Vec<RegisteredTool>> {
    let output = run_bun_bridge(extension, "inspect", serde_json::Value::Null)?;
    serde_json::from_value(output).map_err(Error::from)
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

fn run_bun_bridge(
    extension: &TypeScriptExtension,
    action: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value> {
    ensure_bun_available()?;

    let bridge = std::env::temp_dir().join("imp-ts-extension-bridge.ts");
    std::fs::write(&bridge, BUN_BRIDGE).map_err(Error::from)?;

    let output = Command::new("bun")
        .arg(&bridge)
        .arg(action)
        .arg(&extension.entrypoint)
        .arg(serde_json::to_string(&payload)?)
        .current_dir(&extension.root)
        .output()
        .map_err(Error::from)?;

    if !output.status.success() {
        return Err(Error::Tool(format!(
            "TypeScript extension '{}' failed: {}",
            extension.name,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    serde_json::from_slice(&output.stdout).map_err(Error::from)
}

fn ensure_bun_available() -> Result<()> {
    Command::new("bun")
        .arg("--version")
        .output()
        .map(|_| ())
        .map_err(|_| {
            Error::Tool(
                "TypeScript extensions require Bun. Install Bun and ensure `bun` is on PATH."
                    .to_string(),
            )
        })
}

fn normalize_parameters(parameters: Option<serde_json::Value>) -> serde_json::Value {
    parameters.unwrap_or_else(|| serde_json::json!({ "type": "object", "properties": {} }))
}

fn file_stem_string(path: &Path) -> Option<String> {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .filter(|stem| !stem.is_empty())
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

const BUN_BRIDGE: &str = r#"
import { pathToFileURL } from 'node:url';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { writeFileSync } from 'node:fs';

const [,, action, entrypoint, payloadJson] = process.argv;
const payload = JSON.parse(payloadJson ?? 'null');
const tools = [];
const lifecycleHandlers = new Map();

const Type = {
  String: (options = {}) => ({ type: 'string', ...options }),
  Number: (options = {}) => ({ type: 'number', ...options }),
  Integer: (options = {}) => ({ type: 'integer', ...options }),
  Boolean: (options = {}) => ({ type: 'boolean', ...options }),
  Array: (items, options = {}) => ({ type: 'array', items, ...options }),
  Object: (properties = {}, options = {}) => ({ type: 'object', properties, ...options }),
  Optional: (schema) => ({ ...schema, __optional: true }),
  Union: (schemas, options = {}) => ({ anyOf: schemas, ...options }),
  Literal: (value, options = {}) => ({ const: value, ...options }),
};

const StringEnum = (values) => ({ enum: values });

const piModule = {
  Type,
  StringEnum,
  getAgentDir: () => process.cwd(),
  getSettingsListTheme: () => ({}),
  withFileMutationQueue: async (_path, fn) => fn(),
  createReadTool: () => unsupportedNativeTool('read'),
  createBashTool: () => unsupportedNativeTool('bash'),
  createEditTool: () => unsupportedNativeTool('edit'),
  createWriteTool: () => unsupportedNativeTool('write'),
};

const tuiModule = {
  Text: class Text { constructor(text) { this.text = text; } },
  Key: { ctrlShift: (key) => `ctrl+shift+${key}` },
  matchesKey: () => false,
  truncateToWidth: (s) => s,
  wrapTextWithAnsi: (s) => [s],
  Container: class Container {},
  SettingsList: class SettingsList {},
  SelectList: class SelectList {},
  DynamicBorder: class DynamicBorder {},
};

function unsupportedNativeTool(name) {
  return {
    name,
    label: name,
    description: `${name} is not available through imp's Pi compatibility shim yet`,
    parameters: { type: 'object', properties: {} },
    async execute() { throw new Error(`create${name}Tool is not supported by imp TypeScript extensions yet`); },
  };
}

const originalImport = globalThis.__import ?? ((specifier) => import(specifier));
async function importWithShim(specifier) {
  if (specifier === '@mariozechner/pi-coding-agent' || specifier === '@mariozechner/pi-ai') return piModule;
  if (specifier === '@sinclair/typebox') return { Type };
  if (specifier === '@mariozechner/pi-tui') return tuiModule;
  return originalImport(specifier);
}

globalThis.__imp_import = importWithShim;

async function loadEntrypoint(path) {
  const source = await Bun.file(path).text();
  const rewritten = source
    .replaceAll('import { Type } from "@mariozechner/pi-coding-agent";', 'const { Type } = globalThis.__imp_pi;')
    .replaceAll("import { Type } from '@mariozechner/pi-coding-agent';", 'const { Type } = globalThis.__imp_pi;')
    .replaceAll('import { StringEnum } from "@mariozechner/pi-ai";', 'const { StringEnum } = globalThis.__imp_pi;')
    .replaceAll("import { StringEnum } from '@mariozechner/pi-ai';", 'const { StringEnum } = globalThis.__imp_pi;')
    .replaceAll('import { Type } from "@sinclair/typebox";', 'const { Type } = globalThis.__imp_typebox;')
    .replaceAll("import { Type } from '@sinclair/typebox';", 'const { Type } = globalThis.__imp_typebox;')
    .replaceAll('from "@mariozechner/pi-coding-agent"', 'from "data:text/javascript,export default globalThis.__imp_pi;"')
    .replaceAll("from '@mariozechner/pi-coding-agent'", "from 'data:text/javascript,export default globalThis.__imp_pi;'")
    .replaceAll('from "@mariozechner/pi-ai"', 'from "data:text/javascript,"')
    .replaceAll("from '@mariozechner/pi-ai'", "from 'data:text/javascript,'")
    .replaceAll('from "@sinclair/typebox"', 'from "data:text/javascript,"')
    .replaceAll("from '@sinclair/typebox'", "from 'data:text/javascript,'")
    .replaceAll('import { matchesKey, Text, truncateToWidth } from "@mariozechner/pi-tui";', 'const { matchesKey, Text, truncateToWidth } = globalThis.__imp_tui;')
    .replaceAll("import { matchesKey, Text, truncateToWidth } from '@mariozechner/pi-tui';", 'const { matchesKey, Text, truncateToWidth } = globalThis.__imp_tui;')
    .replaceAll('from "@mariozechner/pi-tui"', 'from "data:text/javascript,"')
    .replaceAll("from '@mariozechner/pi-tui'", "from 'data:text/javascript,'");
  const rewrittenPath = join(tmpdir(), `imp-ts-extension-entry-${process.pid}.ts`);
  writeFileSync(rewrittenPath, rewritten);
  return import(pathToFileURL(rewrittenPath).href);
}

globalThis.__imp_Type = Type;
globalThis.__imp_StringEnum = StringEnum;
globalThis.__imp_pi = piModule;
globalThis.__imp_typebox = { Type };
globalThis.__imp_tui = tuiModule;

function normalizeSchema(schema) {
  if (!schema || typeof schema !== 'object') return schema;
  if (schema.type === 'object' && schema.properties) {
    const required = [];
    const properties = {};
    for (const [key, value] of Object.entries(schema.properties)) {
      const normalized = normalizeSchema(value);
      if (!value.__optional) required.push(key);
      delete normalized.__optional;
      properties[key] = normalized;
    }
    return { ...schema, properties, required };
  }
  if (schema.anyOf) return { ...schema, anyOf: schema.anyOf.map(normalizeSchema) };
  if (schema.items) return { ...schema, items: normalizeSchema(schema.items) };
  return { ...schema };
}

const pi = {
  registerTool(def) { tools.push(def); },
  registerCommand() {},
  on(event, handler) {
    const handlers = lifecycleHandlers.get(event) ?? [];
    handlers.push(handler);
    lifecycleHandlers.set(event, handlers);
  },
};

function extensionContext() {
  return {
    cwd: process.cwd(),
    hasUI: false,
    ui: { notify() {}, setStatus() {}, custom() { throw new Error('ctx.ui.custom is not supported by imp TypeScript extensions yet'); } },
    sessionManager: {
      getBranch() { return []; },
      getEntries() { return []; },
    },
  };
}

async function fireLifecycle(event) {
  const handlers = lifecycleHandlers.get(event) ?? [];
  for (const handler of handlers) {
    await handler({}, extensionContext());
  }
}

try {
  const mod = await loadEntrypoint(entrypoint);
  const init = mod.default ?? mod;
  if (typeof init !== 'function') throw new Error('extension default export is not a function');
  await init(pi);
  await fireLifecycle('session_start');

  if (action === 'inspect') {
    console.log(JSON.stringify(tools.map((tool) => ({
      name: tool.name,
      label: tool.label,
      description: tool.description,
      parameters: normalizeSchema(tool.parameters ?? { type: 'object', properties: {} }),
    }))));
  } else if (action === 'execute') {
    const tool = tools.find((tool) => tool.name === payload.tool);
    if (!tool) throw new Error(`tool not registered: ${payload.tool}`);
    const result = await tool.execute('imp-ts-call', payload.params ?? {}, new AbortController().signal, () => {}, extensionContext());
    console.log(JSON.stringify(result));
  } else {
    throw new Error(`unknown bridge action: ${action}`);
  }
} catch (error) {
  console.error(error?.stack ?? String(error));
  process.exit(1);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
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
    fn loads_official_dynamic_tools_fixture() {
        if Command::new("bun").arg("--version").output().is_err() {
            return;
        }

        let temp = TempDir::new().unwrap();
        let extension_dir = temp
            .path()
            .join(".imp")
            .join("extensions")
            .join("pi")
            .join("dynamic-tools");
        std::fs::create_dir_all(&extension_dir).unwrap();
        std::fs::write(
            extension_dir.join("index.ts"),
            r#"
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
"#,
        )
        .unwrap();

        let mut registry = ToolRegistry::new();
        load_typescript_extensions(temp.path(), &mut registry).unwrap();
        let tool = registry
            .get("echo_session")
            .expect("dynamic tool registered");
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
        }
    }
}
