use std::process::Command;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use imp_core::tools::lua::{parameter_schema_from_lua, tool_output_from_lua_result};
use imp_core::tools::{Tool, ToolContext, ToolOutput, ToolRegistry};
use imp_core::Error as CoreError;
use mlua::{Function, Lua, Table, Value};
use serde_json::json;

use crate::sandbox::{LuaCommandHandle, LuaError, LuaHookHandle, LuaRuntime, LuaToolHandle};

/// A `Tool` implementation backed by a Lua function registered with
/// `imp.register_tool()`.
pub struct LuaTool {
    name: String,
    label: String,
    description: String,
    readonly: bool,
    params: serde_json::Value,
    runtime: Arc<Mutex<LuaRuntime>>,
    handle_index: usize,
}

#[async_trait]
impl Tool for LuaTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> serde_json::Value {
        parameter_schema_from_lua(&self.params)
    }

    fn is_readonly(&self) -> bool {
        self.readonly
    }

    async fn execute(
        &self,
        call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> imp_core::Result<ToolOutput> {
        let runtime = Arc::clone(&self.runtime);
        let handle_index = self.handle_index;
        let call_id = call_id.to_string();
        let ctx_json = json!({
            "cwd": ctx.cwd.display().to_string(),
            "cancelled": ctx.is_cancelled(),
        });

        tokio::task::spawn_blocking(move || {
            let runtime_guard = runtime
                .lock()
                .map_err(|_| CoreError::Tool("Lua runtime lock poisoned".into()))?;
            let tools = runtime_guard.tools();
            let handles = tools
                .lock()
                .map_err(|_| CoreError::Tool("Lua tool registry lock poisoned".into()))?;
            let handle = handles.get(handle_index).ok_or_else(|| {
                CoreError::Tool(format!("Lua tool handle {handle_index} not found"))
            })?;

            let execute_fn: Function = runtime_guard
                .lua()
                .registry_value(&handle.execute_key)
                .map_err(lua_tool_error)?;
            let lua_params =
                json_to_lua_value(runtime_guard.lua(), &params).map_err(lua_tool_error)?;
            let lua_ctx =
                json_to_lua_value(runtime_guard.lua(), &ctx_json).map_err(lua_tool_error)?;
            let result: Value = execute_fn
                .call((call_id.as_str(), lua_params, lua_ctx))
                .map_err(lua_tool_error)?;

            tool_output_from_lua_result(lua_value_to_json(result))
        })
        .await
        .map_err(|error| CoreError::Tool(format!("Lua tool task failed: {error}")))?
    }
}

/// Register all currently loaded Lua tools with imp-core's tool registry.
pub fn load_lua_tools(runtime: Arc<Mutex<LuaRuntime>>, registry: &mut ToolRegistry) {
    let handles = {
        let runtime_guard = runtime
            .lock()
            .expect("Lua runtime lock poisoned while loading tools");
        let tools = runtime_guard.tools();
        let handles = tools
            .lock()
            .expect("Lua tool registry lock poisoned while loading tools");

        handles
            .iter()
            .enumerate()
            .map(|(index, handle)| LuaTool {
                name: handle.name.clone(),
                label: handle.label.clone(),
                description: handle.description.clone(),
                readonly: handle.readonly,
                params: handle.params.clone(),
                runtime: Arc::clone(&runtime),
                handle_index: index,
            })
            .collect::<Vec<_>>()
    };

    for tool in handles {
        registry.register(Arc::new(tool));
    }
}

fn lua_tool_error(error: mlua::Error) -> CoreError {
    CoreError::Tool(format!("Lua tool error: {error}"))
}

/// Set up the `imp` global table with host API functions.
///
/// Exposes to Lua:
/// - imp.on(event, handler)           — subscribe to hook events
/// - imp.register_tool(def)           — register a custom tool
/// - imp.exec(command, args, opts)    — run a shell command
/// - imp.register_command(name, def)  — register a slash command
/// - imp.events.on() / imp.events.emit() — inter-extension event bus
pub fn setup_host_api(runtime: &LuaRuntime) -> Result<(), LuaError> {
    let lua = runtime.lua();

    let imp = lua.create_table()?;

    // ── imp.on(event_name, handler) ──────────────────────────────
    let hooks = runtime.hooks();
    let on_fn = lua.create_function(move |lua_inner, (event, handler): (String, Function)| {
        let key = lua_inner.create_registry_value(handler)?;
        let handle = LuaHookHandle {
            event,
            handler_key: key,
        };
        hooks.lock().unwrap().push(handle);
        Ok(())
    })?;
    imp.set("on", on_fn)?;

    // ── imp.register_tool(definition) ────────────────────────────
    let tools = runtime.tools();
    let register_tool_fn = lua.create_function(move |lua_inner, def: Table| {
        let name: String = def.get("name")?;
        let label: String = def
            .get::<Option<String>>("label")?
            .unwrap_or_else(|| name.clone());
        let description: String = def
            .get::<Option<String>>("description")?
            .unwrap_or_default();
        let readonly: bool = def.get::<Option<bool>>("readonly")?.unwrap_or(false);

        let params_val: Value = def.get("params")?;
        let params = lua_value_to_json(params_val);

        let execute_fn: Function = def.get("execute")?;
        let key = lua_inner.create_registry_value(execute_fn)?;

        let handle = LuaToolHandle {
            name,
            label,
            description,
            readonly,
            params,
            execute_key: key,
        };
        tools.lock().unwrap().push(handle);
        Ok(())
    })?;
    imp.set("register_tool", register_tool_fn)?;

    // ── imp.exec(command, args, opts) ────────────────────────────
    let exec_fn = lua.create_function(
        |lua_inner, (cmd, args, opts): (String, Option<Table>, Option<Table>)| {
            let mut command = Command::new("sh");
            command.arg("-c");

            // Build the full command string
            let full_cmd = if let Some(args_table) = args {
                let mut parts = vec![cmd];
                for pair in args_table.sequence_values::<String>() {
                    parts.push(pair?);
                }
                parts.join(" ")
            } else {
                cmd
            };
            command.arg(&full_cmd);

            // Apply opts
            if let Some(opts_table) = &opts {
                if let Ok(Some(cwd)) = opts_table.get::<Option<String>>("cwd") {
                    command.current_dir(cwd);
                }
            }

            let output = command.output().map_err(mlua::Error::external)?;

            let result = lua_inner.create_table()?;
            result.set(
                "stdout",
                String::from_utf8_lossy(&output.stdout).to_string(),
            )?;
            result.set(
                "stderr",
                String::from_utf8_lossy(&output.stderr).to_string(),
            )?;
            result.set("exit_code", output.status.code().unwrap_or(-1))?;

            Ok(result)
        },
    )?;
    imp.set("exec", exec_fn)?;

    // ── imp.register_command(name, definition) ───────────────────
    let commands = runtime.commands();
    let register_command_fn =
        lua.create_function(move |lua_inner, (name, def): (String, Table)| {
            let description: String = def
                .get::<Option<String>>("description")?
                .unwrap_or_default();
            let handler: Function = def.get("handler")?;
            let key = lua_inner.create_registry_value(handler)?;

            let handle = LuaCommandHandle {
                name,
                description,
                handler_key: key,
            };
            commands.lock().unwrap().push(handle);
            Ok(())
        })?;
    imp.set("register_command", register_command_fn)?;

    // ── imp.events (inter-extension event bus) ───────────────────
    let events = lua.create_table()?;

    // Store handlers in a Lua table: { event_name = { handler1, handler2, ... } }
    let handlers_table = lua.create_table()?;
    lua.set_named_registry_value("__imp_event_handlers", handlers_table)?;

    let events_on = lua.create_function(|lua_inner, (name, handler): (String, Function)| {
        let handlers: Table = lua_inner.named_registry_value("__imp_event_handlers")?;
        let list: Table = match handlers.get::<Option<Table>>(name.as_str())? {
            Some(t) => t,
            None => {
                let t = lua_inner.create_table()?;
                handlers.set(name.as_str(), t.clone())?;
                t
            }
        };
        let len = list.raw_len();
        list.set(len + 1, handler)?;
        Ok(())
    })?;
    events.set("on", events_on)?;

    let events_emit = lua.create_function(|lua_inner, (name, data): (String, Value)| {
        let handlers: Table = lua_inner.named_registry_value("__imp_event_handlers")?;
        if let Some(list) = handlers.get::<Option<Table>>(name.as_str())? {
            for pair in list.sequence_values::<Function>() {
                let handler = pair?;
                // Errors in event handlers are caught and ignored (logged via eprintln)
                if let Err(e) = handler.call::<()>(data.clone()) {
                    eprintln!("[imp-lua] event handler error for '{}': {}", name, e);
                }
            }
        }
        Ok(())
    })?;
    events.set("emit", events_emit)?;

    imp.set("events", events)?;

    // ── Set the global ───────────────────────────────────────────
    lua.globals().set("imp", imp)?;

    Ok(())
}

/// Convert a Lua value to serde_json::Value.
pub fn lua_value_to_json(value: Value) -> serde_json::Value {
    match value {
        Value::Nil => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(b),
        Value::Integer(i) => serde_json::Value::Number(serde_json::Number::from(i)),
        Value::Number(n) => serde_json::Number::from_f64(n)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::String(s) => {
            serde_json::Value::String(s.to_str().map(|s| s.to_string()).unwrap_or_default())
        }
        Value::Table(t) => {
            // Check if it's an array (sequential integer keys starting at 1)
            let len = t.raw_len();
            if len > 0 {
                // Check if all keys 1..=len exist (it's an array)
                let is_array = (1..=len).all(|i| {
                    t.get::<Value>(i)
                        .ok()
                        .map(|v| !matches!(v, Value::Nil))
                        .unwrap_or(false)
                });
                if is_array {
                    let arr: Vec<serde_json::Value> = (1..=len)
                        .filter_map(|i| t.get::<Value>(i).ok().map(lua_value_to_json))
                        .collect();
                    return serde_json::Value::Array(arr);
                }
            }

            // Otherwise it's an object
            let mut map = serde_json::Map::new();
            if let Ok(pairs) = t.pairs::<String, Value>().collect::<Result<Vec<_>, _>>() {
                for (k, v) in pairs {
                    map.insert(k, lua_value_to_json(v));
                }
            }
            serde_json::Value::Object(map)
        }
        _ => serde_json::Value::Null,
    }
}

/// Convert a serde_json::Value to a Lua value.
pub fn json_to_lua_value(lua: &Lua, value: &serde_json::Value) -> mlua::Result<Value> {
    match value {
        serde_json::Value::Null => Ok(Value::Nil),
        serde_json::Value::Bool(b) => Ok(Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Number(f))
            } else {
                Ok(Value::Nil)
            }
        }
        serde_json::Value::String(s) => Ok(Value::String(lua.create_string(s)?)),
        serde_json::Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, v) in arr.iter().enumerate() {
                table.set(i + 1, json_to_lua_value(lua, v)?)?;
            }
            Ok(Value::Table(table))
        }
        serde_json::Value::Object(map) => {
            let table = lua.create_table()?;
            for (k, v) in map {
                table.set(k.as_str(), json_to_lua_value(lua, v)?)?;
            }
            Ok(Value::Table(table))
        }
    }
}
