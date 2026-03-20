use std::sync::{Arc, Mutex};

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

/// Manages the Lua state for extensions.
pub struct LuaRuntime {
    lua: Lua,
    tools: Arc<Mutex<Vec<LuaToolHandle>>>,
    hooks: Arc<Mutex<Vec<LuaHookHandle>>>,
    commands: Arc<Mutex<Vec<LuaCommandHandle>>>,
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
        self.tools.lock().unwrap().iter().map(|t| t.name.clone()).collect()
    }

    /// Get hook event names.
    pub fn hook_events(&self) -> Vec<String> {
        self.hooks.lock().unwrap().iter().map(|h| h.event.clone()).collect()
    }
}
