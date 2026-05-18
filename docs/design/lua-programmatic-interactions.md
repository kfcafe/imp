# Lua programmatic interactions

Lua extensions can ask the host for structured user interaction through `imp.ui`.

## API

Primitive:

```lua
local result = imp.ui.request({
  kind = "confirm", -- confirm | select | multi_select | input | custom
  title = "Install aush?",
  message = "This will run uu install --default.",
})
```

Return shape:

```lua
{ ok = true, value = true }
{ ok = false, reason = "cancelled" }
{ ok = false, reason = "unavailable" }
{ ok = false, reason = "invalid", message = "..." }
```

Ergonomic yes/no helper:

```lua
local yes = imp.ui.confirm("Install aush?", "Run uu install --default?")
if yes == true then
  -- proceed
elseif yes == false then
  -- explicit no
else
  -- unavailable or cancelled
end
```

## Request kinds

### confirm

```lua
imp.ui.request({ kind = "confirm", title = "Proceed?", message = "Continue?" })
```

Returns `value = true` or `value = false` when answered.

### select

```lua
imp.ui.request({
  kind = "select",
  title = "Pick target",
  options = {
    { label = "imp", description = "Install imp" },
    { label = "aush", description = "Install aush" },
  },
})
```

Returns `value = { index = 1, label = "imp" }`. Lua-facing indexes are 1-based.

### multi_select

Returns an array of `{ index, label }` selections.

### input

```lua
imp.ui.request({ kind = "input", title = "Version", placeholder = "0.1.3" })
```

Returns the entered string.

### custom

Passes a serialized `ComponentSpec` to the active host UI. Unsupported hosts return cancelled/unavailable.

## Headless and safety behavior

Headless or missing UI contexts must not invent approval. They return `{ ok=false, reason="unavailable" }`; `imp.ui.confirm(...)` returns `nil`. Lua commands that guard installs, writes, or destructive actions should stop unless they receive explicit `true`.

## Current implementation notes

- Lua tools get the active `ToolContext` and can call `imp.ui.request` / `imp.ui.confirm`.
- Lua commands now have an optional `LuaCallContext` execution path so command handlers can call the same API.
- TUI command execution wires the shared `TuiInterface` when available; otherwise commands see a safe null UI.
- The native UI abstraction remains `imp_core::ui::UserInterface`.
