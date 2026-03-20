use std::process::Command;
use mlua::{Function, Lua, Table, Value};

use crate::sandbox::{LuaCommandHandle, LuaError, LuaHookHandle, LuaRuntime, LuaToolHandle};

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
        let label: String = def.get::<Option<String>>("label")?.unwrap_or_else(|| name.clone());
        let description: String = def.get::<Option<String>>("description")?.unwrap_or_default();
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
    let exec_fn = lua.create_function(|lua_inner, (cmd, args, opts): (String, Option<Table>, Option<Table>)| {
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
        result.set("stdout", String::from_utf8_lossy(&output.stdout).to_string())?;
        result.set("stderr", String::from_utf8_lossy(&output.stderr).to_string())?;
        result.set("exit_code", output.status.code().unwrap_or(-1))?;

        Ok(result)
    })?;
    imp.set("exec", exec_fn)?;

    // ── imp.register_command(name, definition) ───────────────────
    let commands = runtime.commands();
    let register_command_fn = lua.create_function(move |lua_inner, (name, def): (String, Table)| {
        let description: String = def.get::<Option<String>>("description")?.unwrap_or_default();
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
        Value::Number(n) => {
            serde_json::Number::from_f64(n)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        Value::String(s) => serde_json::Value::String(s.to_str().map(|s| s.to_string()).unwrap_or_default()),
        Value::Table(t) => {
            // Check if it's an array (sequential integer keys starting at 1)
            let len = t.raw_len();
            if len > 0 {
                // Check if all keys 1..=len exist (it's an array)
                let is_array = (1..=len).all(|i| t.get::<Value>(i).ok().map(|v| !matches!(v, Value::Nil)).unwrap_or(false));
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
