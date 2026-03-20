use crate::tools::ToolRegistry;

/// Load Lua-defined tools into the registry.
///
/// This is called by imp-lua when Lua extensions register tools via
/// `imp.register_tool()`. The bridge creates a LuaTool struct that
/// implements the Tool trait and delegates execution to the Lua function.
pub fn load_lua_tools(_registry: &mut ToolRegistry) {
    // TODO: Implement once imp-lua bridge is ready.
    // Lua tools are registered dynamically, not at startup.
}
