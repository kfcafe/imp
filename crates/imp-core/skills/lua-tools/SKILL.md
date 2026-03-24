---
name: lua-tools
description: >
  Write Lua tools and extensions for imp. Use when the user asks to create a custom tool,
  port a tool from another agent (pi, Claude Code, Codex, OpenClaw), add a hook, register
  a slash command, or build any Lua extension. Covers the full imp.register_tool() API,
  imp.on() hooks, imp.exec() shell access, imp.events inter-extension bus, and
  imp.register_command() slash commands.
---

# Writing Lua Tools for Imp

## File Layout

Extensions live in one of two places:

**User-global** (available in all projects):
```
~/.config/imp/lua/
├── my-tool.lua              # single-file extension
└── my-complex-ext/
    └── init.lua             # directory extension (entry point)
```

**Project-local** (available only in this project):
```
.imp/lua/
├── project-tool.lua
└── project-ext/
    └── init.lua
```

Imp discovers and loads all extensions automatically on startup. Hot reload replaces the entire Lua runtime — no restart needed.

## Registering a Tool

Tools appear in the LLM's tool list. The model can call them like any built-in tool.

```lua
imp.register_tool({
    name = "tool_name",           -- required, snake_case
    label = "Human-Readable Name", -- optional, defaults to name
    description = "What this tool does. The LLM reads this to decide when to use it.",
    readonly = false,              -- true = no side effects (safe for readonly modes)
    params = {
        -- Shorthand: keys are param names, "required = true" marks required params
        query = { type = "string", description = "Search query", required = true },
        limit = { type = "number", description = "Max results" },
        -- OR full JSON Schema:
        -- type = "object",
        -- properties = { ... },
        -- required = { "query" }
    },
    execute = function(call_id, params, ctx)
        -- call_id: unique ID for this invocation
        -- params: table matching the schema above
        -- ctx: { cwd = "/path/to/project", cancelled = false }

        -- Return a string (simplest):
        return { content = "result text" }

        -- Return structured content blocks:
        return {
            content = {
                { type = "text", text = "main output here" },
            },
            details = { key = "value" },  -- metadata (shown in UI, not sent to LLM)
            is_error = false,
        }
    end
})
```

### Return Value Formats

The `execute` function can return:

| Return value | Behavior |
|---|---|
| `{ content = "string" }` | Single text block |
| `{ content = { {type="text", text="..."}, ... } }` | Array of content blocks |
| `{ content = "...", is_error = true }` | Error result shown to LLM |
| `{ content = "...", details = {...} }` | Result with metadata |
| `"plain string"` | Treated as `{ content = "plain string" }` |

### Parameter Schema

Two forms are accepted:

**Shorthand** (recommended for simple tools):
```lua
params = {
    path = { type = "string", description = "File path", required = true },
    pattern = { type = "string", description = "Search pattern" },
}
```
Imp auto-wraps this into a JSON Schema object and extracts `required` fields.

**Full JSON Schema** (when you need `enum`, `anyOf`, nested objects):
```lua
params = {
    type = "object",
    properties = {
        action = {
            type = "string",
            enum = { "search", "replace" },
            description = "Action to perform"
        },
    },
    required = { "action" },
}
```

## Shell Execution

`imp.exec()` runs commands and captures output. Use this to wrap CLI tools.

```lua
local result = imp.exec("rg --json 'pattern' .")
-- result.stdout   (string)
-- result.stderr   (string)
-- result.exit_code (number)

-- With arguments as a table:
local result = imp.exec("curl", { "-s", url })

-- With working directory:
local result = imp.exec("git log --oneline -5", nil, { cwd = ctx.cwd })
```

## Hooks

React to agent lifecycle events:

```lua
imp.on("on_session_start", function(event, ctx)
    -- Runs when a new session begins
end)

imp.on("after_file_write", function(event, ctx)
    -- Runs after any file is written
    -- event contains details about which file
end)

imp.on("before_tool_call", function(event, ctx)
    -- Runs before any tool call
end)
```

## Slash Commands

Register custom `/commands` for the TUI:

```lua
imp.register_command("deploy", {
    description = "Deploy the current project",
    handler = function(args, ctx)
        local result = imp.exec("make deploy")
        return result.stdout
    end
})
```

## Inter-Extension Events

Extensions can communicate with each other:

```lua
-- Extension A: listen
imp.events.on("build_complete", function(data)
    -- data is whatever was emitted
end)

-- Extension B: emit
imp.events.emit("build_complete", { status = "ok", duration = 3.2 })
```

Handler errors are caught and logged — one bad handler doesn't break others.

## Porting Tools from Other Agents

Most tools in pi, Claude Code, and other agents are structured wrappers around CLI commands. The pattern for porting:

1. Identify what CLI the tool wraps (e.g., `probe`, `ast-grep`, `tokei`, `curl`)
2. Write a Lua tool that calls `imp.exec()` with that CLI
3. Parse the CLI output and return structured content

### Example: Wrapping a CLI tool

```lua
-- Port of a code search tool that wraps the `probe` CLI
imp.register_tool({
    name = "probe_search",
    label = "Semantic Code Search",
    description = "Search code using Probe. Supports boolean queries (AND, OR, NOT), "
        .. "phrases, and hints (ext:, dir:, lang:). Returns complete code blocks.",
    readonly = true,
    params = {
        query = { type = "string", description = "Search query", required = true },
        path = { type = "string", description = "Directory to search (default: cwd)" },
        max_results = { type = "number", description = "Max results (default: 10)" },
    },
    execute = function(call_id, params, ctx)
        local path = params.path or ctx.cwd
        local limit = params.max_results or 10
        local cmd = string.format(
            "probe '%s' --dir '%s' --max-results %d",
            params.query:gsub("'", "'\\''"),
            path:gsub("'", "'\\''"),
            limit
        )
        local result = imp.exec(cmd, nil, { cwd = ctx.cwd })
        if result.exit_code ~= 0 then
            return { content = "probe error: " .. result.stderr, is_error = true }
        end
        return { content = result.stdout }
    end
})
```

### Example: Tool with JSON parsing

```lua
-- Wrap a tool that outputs JSON
imp.register_tool({
    name = "code_stats",
    label = "Code Statistics",
    description = "Count lines of code by language using tokei.",
    readonly = true,
    params = {
        path = { type = "string", description = "Directory to analyze" },
    },
    execute = function(call_id, params, ctx)
        local path = params.path or ctx.cwd
        local result = imp.exec("tokei --output json " .. path)
        if result.exit_code ~= 0 then
            return { content = "tokei error: " .. result.stderr, is_error = true }
        end
        -- Return raw JSON as text — the LLM can read it
        return { content = result.stdout }
    end
})
```

## Common Patterns

### Error handling
```lua
execute = function(call_id, params, ctx)
    if not params.required_field then
        return { content = "Missing required parameter: required_field", is_error = true }
    end

    local result = imp.exec(cmd)
    if result.exit_code ~= 0 then
        return {
            content = "Command failed (exit " .. result.exit_code .. "): " .. result.stderr,
            is_error = true,
        }
    end

    return { content = result.stdout }
end
```

### Respecting cancellation
```lua
execute = function(call_id, params, ctx)
    if ctx.cancelled then
        return { content = "Cancelled", is_error = true }
    end
    -- ... do work ...
end
```

### Multiple content blocks
```lua
return {
    content = {
        { type = "text", text = "## Results\n\n" .. main_output },
        { type = "text", text = "\n---\n" .. summary },
    },
    details = {
        files_searched = 42,
        matches = 7,
    },
}
```

## Testing a Lua Tool

1. Write the `.lua` file in `~/.config/imp/lua/` or `.imp/lua/`
2. Start imp (or hot-reload if running)
3. Ask the agent to use the tool — it appears in the tool list automatically
4. Check output. If the tool errors, the error message is shown to the LLM.

## Rules

- Tool names must be `snake_case` — the LLM uses them as function names
- Keep descriptions clear and specific — the LLM decides when to use the tool based on this text
- Set `readonly = true` for tools that only read data (search, stats, lookup)
- Always handle `exit_code ~= 0` from `imp.exec()` — return `is_error = true`
- Shell-escape user-provided strings to prevent injection
- Prefer returning text the LLM can parse over complex structured data
- One tool per file for simple tools; use `init.lua` directories for complex ones
