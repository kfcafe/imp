use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use imp_core::config::AgentMode;
use imp_core::tools::{FileCache, FileTracker, Tool, ToolContext, ToolUpdate};
use imp_core::ui::UserInterface;
use mlua::Lua;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LuaError {
    #[error("Lua error: {0}")]
    Mlua(#[from] mlua::Error),

    #[error("Extension error: {0}")]
    Extension(String),
}

/// Handle to a Lua-registered tool.
pub struct LuaToolHandle {
    pub name: String,
    pub label: String,
    pub description: String,
    pub readonly: bool,
    pub params: serde_json::Value,
    /// Registry key for the execute function stored in Lua.
    pub execute_key: mlua::RegistryKey,
}

/// Handle to a Lua-registered hook.
pub struct LuaHookHandle {
    pub event: String,
    /// Registry key for the handler function stored in Lua.
    pub handler_key: mlua::RegistryKey,
}

/// Handle to a Lua-registered command.
pub struct LuaCommandHandle {
    pub name: String,
    pub description: String,
    pub handler_key: mlua::RegistryKey,
}

/// Context passed to Lua host API functions during tool execution.
///
/// Mirrors `ToolContext` but is stored separately so the Lua
/// `imp.tool()` callback can construct a fresh `ToolContext` for
/// each native tool call.
pub struct LuaCallContext {
    pub cwd: PathBuf,
    pub cancelled: Arc<std::sync::atomic::AtomicBool>,
    pub update_tx: tokio::sync::mpsc::Sender<ToolUpdate>,
    pub ui: Arc<dyn UserInterface>,
    pub file_cache: Arc<FileCache>,
    pub file_tracker: Arc<std::sync::Mutex<FileTracker>>,
    pub mode: AgentMode,
    pub read_max_lines: usize,
}

impl LuaCallContext {
    /// Build a `ToolContext` from the stored fields.
    pub fn to_tool_context(&self) -> ToolContext {
        ToolContext {
            cwd: self.cwd.clone(),
            cancelled: Arc::clone(&self.cancelled),
            update_tx: self.update_tx.clone(),
            ui: Arc::clone(&self.ui),
            file_cache: Arc::clone(&self.file_cache),
            file_tracker: Arc::clone(&self.file_tracker),
            mode: self.mode,
            read_max_lines: self.read_max_lines,
        }
    }
}

/// Manages the Lua state for extensions.
pub struct LuaRuntime {
    lua: Lua,
    tools: Arc<Mutex<Vec<LuaToolHandle>>>,
    hooks: Arc<Mutex<Vec<LuaHookHandle>>>,
    commands: Arc<Mutex<Vec<LuaCommandHandle>>>,
    /// Native imp tools available via `imp.tool()` from Lua.
    native_tools: Arc<Mutex<HashMap<String, Arc<dyn Tool>>>>,
    /// Active execution context for `imp.tool()` calls.
    call_context: Arc<Mutex<Option<LuaCallContext>>>,
    /// Env vars this extension is allowed to read via `imp.env()`.
    allowed_env: Arc<Mutex<HashSet<String>>>,
    /// Whether Lua host-side native tool calls are permitted for the current execution.
    allow_native_tool_calls: Arc<AtomicBool>,
}

impl LuaRuntime {
    /// Create a new Lua runtime with standard libraries.
    pub fn new() -> Result<Self, LuaError> {
        let lua = Lua::new();
        Ok(Self {
            lua,
            tools: Arc::new(Mutex::new(Vec::new())),
            hooks: Arc::new(Mutex::new(Vec::new())),
            commands: Arc::new(Mutex::new(Vec::new())),
            native_tools: Arc::new(Mutex::new(HashMap::new())),
            call_context: Arc::new(Mutex::new(None)),
            allowed_env: Arc::new(Mutex::new(HashSet::new())),
            allow_native_tool_calls: Arc::new(AtomicBool::new(true)),
        })
    }

    /// Get a reference to the underlying Lua state.
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// Get a clone of the tools handle for external access.
    pub fn tools(&self) -> Arc<Mutex<Vec<LuaToolHandle>>> {
        Arc::clone(&self.tools)
    }

    /// Get a clone of the hooks handle for external access.
    pub fn hooks(&self) -> Arc<Mutex<Vec<LuaHookHandle>>> {
        Arc::clone(&self.hooks)
    }

    /// Get a clone of the commands handle for external access.
    pub fn commands(&self) -> Arc<Mutex<Vec<LuaCommandHandle>>> {
        Arc::clone(&self.commands)
    }

    /// Get a clone of the native tools map.
    pub fn native_tools(&self) -> Arc<Mutex<HashMap<String, Arc<dyn Tool>>>> {
        Arc::clone(&self.native_tools)
    }

    /// Get a clone of the call context handle.
    pub fn call_context(&self) -> Arc<Mutex<Option<LuaCallContext>>> {
        Arc::clone(&self.call_context)
    }

    /// Get a clone of the allowed-env handle.
    pub fn allowed_env(&self) -> Arc<Mutex<HashSet<String>>> {
        Arc::clone(&self.allowed_env)
    }

    /// Get whether `imp.tool()` calls are currently permitted.
    pub fn allow_native_tool_calls(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.allow_native_tool_calls)
    }

    /// Populate the native tool registry (called once after tools are registered).
    pub fn set_native_tools(&self, tools: HashMap<String, Arc<dyn Tool>>) {
        *self.native_tools.lock().unwrap() = tools;
    }

    /// Set the call context before executing a Lua tool function.
    pub fn set_call_context(&self, ctx: LuaCallContext) {
        *self.call_context.lock().unwrap() = Some(ctx);
    }

    /// Clear the call context after execution.
    pub fn clear_call_context(&self) {
        *self.call_context.lock().unwrap() = None;
    }

    /// Set the allowed env vars for this extension.
    pub fn set_allowed_env(&self, vars: HashSet<String>) {
        *self.allowed_env.lock().unwrap() = vars;
    }

    /// Set whether `imp.tool()` calls are permitted for the current runtime.
    pub fn set_allow_native_tool_calls(&self, allowed: bool) {
        self.allow_native_tool_calls.store(allowed, Ordering::Relaxed);
    }

    /// Register a tool handle (called from bridge).
    pub fn register_tool(&self, handle: LuaToolHandle) {
        self.tools.lock().unwrap().push(handle);
    }

    /// Register a hook handle (called from bridge).
    pub fn register_hook(&self, handle: LuaHookHandle) {
        self.hooks.lock().unwrap().push(handle);
    }

    /// Register a command handle (called from bridge).
    pub fn register_command(&self, handle: LuaCommandHandle) {
        self.commands.lock().unwrap().push(handle);
    }

    /// Execute a Lua script string.
    pub fn exec(&self, source: &str) -> Result<(), LuaError> {
        self.lua.load(source).exec()?;
        Ok(())
    }

    /// Execute a Lua file.
    pub fn exec_file(&self, path: &std::path::Path) -> Result<(), LuaError> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| LuaError::Extension(format!("{}: {}", path.display(), e)))?;
        self.lua
            .load(&source)
            .set_name(path.to_string_lossy())
            .exec()?;
        Ok(())
    }

    /// Clear all registered tools, hooks, and commands.
    pub fn clear_registrations(&self) {
        self.tools.lock().unwrap().clear();
        self.hooks.lock().unwrap().clear();
        self.commands.lock().unwrap().clear();
    }

    /// Number of registered tools.
    pub fn tool_count(&self) -> usize {
        self.tools.lock().unwrap().len()
    }

    /// Number of registered hooks.
    pub fn hook_count(&self) -> usize {
        self.hooks.lock().unwrap().len()
    }

    /// Number of registered commands.
    pub fn command_count(&self) -> usize {
        self.commands.lock().unwrap().len()
    }

    /// Get tool names.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools
            .lock()
            .unwrap()
            .iter()
            .map(|t| t.name.clone())
            .collect()
    }

    /// Get hook event names.
    pub fn hook_events(&self) -> Vec<String> {
        self.hooks
            .lock()
            .unwrap()
            .iter()
            .map(|h| h.event.clone())
            .collect()
    }

    /// Execute a registered command by name, returning its string output.
    ///
    /// Returns `Ok(None)` if the command returned nil (silent success).
    /// Returns `Ok(Some(text))` if the command returned a string or value.
    /// Returns `Err` if the command handler or name wasn't found.
    pub fn execute_command(&self, name: &str, args: &str) -> Result<Option<String>, LuaError> {
        let commands = self.commands.lock().unwrap();
        let handle = commands
            .iter()
            .find(|c| c.name == name)
            .ok_or_else(|| LuaError::Extension(format!("command '{name}' not found")))?;

        let handler: mlua::Function = self
            .lua
            .registry_value(&handle.handler_key)
            .map_err(LuaError::Mlua)?;

        let result: mlua::Value = handler.call(args.to_string()).map_err(LuaError::Mlua)?;

        match result {
            mlua::Value::Nil => Ok(None),
            mlua::Value::String(s) => Ok(Some(
                s.to_str()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|_| "(non-utf8)".into()),
            )),
            other => {
                let json = crate::bridge::lua_value_to_json(other);
                Ok(Some(format!("{json}")))
            }
        }
    }

    /// Get command names.
    pub fn command_names(&self) -> Vec<String> {
        self.commands
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.name.clone())
            .collect()
    }

    /// Check if a command with the given name exists.
    pub fn has_command(&self, name: &str) -> bool {
        self.commands.lock().unwrap().iter().any(|c| c.name == name)
    }
}
