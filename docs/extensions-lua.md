# Lua extensions

Lua is the shipped extension runtime for imp.

Primary implementation areas:

- `crates/imp-lua/src/loader.rs`
- `crates/imp-lua/src/bridge.rs`
- `crates/imp-lua/src/sandbox.rs`
- `crates/imp-core/src/config.rs`

## Load paths

```text
~/.config/imp/lua/
<project>/.imp/lua/
```

A Lua extension can be a `.lua` file or a directory with `init.lua`.

## Capabilities

Extension capability policy controls access to:

- shell execution
- filesystem access
- HTTP
- secrets
- native imp tools
- UI prompts

Use the narrowest policy that supports the extension.

## Commands

```lua
imp.register_command("greet", {
    description = "Say hello",
    handler = function(args)
        return "Hello, " .. (args or "world")
    end
})
```

Registered commands appear as slash-command extensions where supported by the UI/runtime surface.

## Tools

```lua
imp.register_tool({
    name = "echo_custom",
    description = "Echo text",
    execute = function(call_id, params, ctx)
        return { text = params.text }
    end
})
```

Tool handlers receive a call id, parameter table, and context table. Context includes cwd and cancellation state.

## Hooks

Lua extensions can register handlers for runtime events through the host API. Hooks should be kept small and deterministic where possible.

## Stability

Lua is the current shipped extension path. TypeScript extension support exists in repository code paths but should not be documented as the stable shipped extension system.
