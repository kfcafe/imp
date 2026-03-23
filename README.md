# imp

AI agent engine. Tools, modes, context management, multi-provider LLM streaming.

```
  ╔╗    ╔╗
  ║╚════╝║
  ║ ■  ■ ║
╔═╩══════╩═╗
║    imp    ║
╚══════════╝
```

## What it does

imp runs AI coding agents. You give it a task, it reasons through it, calls tools, and gets work done.

## Features

### Agent engine

| Feature | Details |
|---------|---------|
| Agent loop | ReAct-style: think → act → observe → repeat until done or max turns |
| Extended thinking | 6 levels (off → minimal → low → medium → high → xhigh) per model support |
| Concurrent tool execution | Readonly tools run in parallel, mutable tools run sequentially |
| Retry with backoff | Transient LLM errors (rate limits, overload, network) retry automatically with exponential backoff + jitter |
| Cancellation | Cooperative cancellation — tools check a shared flag and bail cleanly |
| Cost tracking | Per-turn and cumulative token usage and dollar cost |

### Tools

| Tool | What it does |
|------|-------------|
| `read` | Read files (text + images), with offset/limit for large files |
| `write` | Create or overwrite files, auto-creates parent directories |
| `edit` | Find-and-replace with exact text matching |
| `multi_edit` | Multiple edits to one file in a single call |
| `bash` | Shell execution with timeout, streaming output, process group cleanup |
| `grep` | Regex search across files, respects .gitignore, line-truncation |
| `find` | Glob-based file search, respects .gitignore |
| `ls` | Directory listing with dotfiles |
| `diff` | Unified diff preview and patch application |
| `scan` | Tree-sitter AST extraction — types, functions, imports from source files |
| `web` | Web search (Tavily/Exa) and page content extraction |
| `ask` | Prompt the user for input or multiple-choice decisions |
| `mana` | Unit management — status, list, show, create, close, update |
| `memory` | Persistent key-value memory across sessions |
| Shell tools | User-defined tools via TOML definitions (name, command template, params) |

### Agent modes

| Mode | Allowed tools | Mana actions | Purpose |
|------|--------------|--------------|---------|
| `full` | all | all | Interactive use, trusted user |
| `worker` | read, write, bash, ask | show, update, status, list | Execute a scoped task |
| `orchestrator` | read, mana, ask | all | Plan and delegate via units |
| `planner` | read, mana, ask | status, list, show, create | Decompose work, human approves |
| `reviewer` | read, ask | none | Read-only analysis |
| `auditor` | read, mana | status, list, show | Inspect and report |

Enforcement at two levels: disallowed tools are removed from the registry (never in the prompt), and an execution-time guard rejects hallucinated calls.

### Context management

| Feature | Details |
|---------|---------|
| Observation masking | Old tool outputs replaced with `[masked]` when context reaches threshold (default 60%) |
| Sliding window | Last N turns always kept unmasked (default 10) |
| LLM compaction | When context hits compaction threshold (default 80%), the conversation is summarized by the LLM and replaced with a compact version |
| Token estimation | Fast character-based approximation for context budget decisions |
| Original prompt re-injection | After compaction, the original user prompt is re-stated so the model doesn't lose the goal |

### LLM client (imp-llm)

| Feature | Details |
|---------|---------|
| Streaming | Server-sent events parsed into typed `StreamEvent` variants |
| Multi-provider | Anthropic (native), OpenAI (responses API), Google (Gemini), with a shared `Provider` trait |
| Model registry | Alias resolution (sonnet → claude-sonnet-4-20250514), pricing, capabilities, context window metadata |
| Prompt caching | Anthropic `cache_control` on system prompt, tool definitions, and recent turns |
| OAuth | Token storage, refresh flow, provider-specific auth |
| Extended thinking | Maps thinking levels to provider-specific budget tokens |
| Tool use | Structured tool definitions, argument schema, result messages |

### Sessions

| Feature | Details |
|---------|---------|
| Persistence | Append-only JSONL — every message, tool call, and result is saved |
| Resume | Continue a previous session from where it left off |
| Branching | Tree structure — fork from any point, navigate between branches |
| In-memory mode | Ephemeral sessions for headless/testing use |

### Hooks

| Event | When it fires |
|-------|--------------|
| `before_tool_call` | Before any tool executes — can block the call |
| `after_tool_call` | After tool completes — can modify the result |
| `after_file_write` | After write/edit/multi_edit modifies a file |
| `before_llm_call` | Before each LLM request |
| `on_context_threshold` | When context usage crosses a configured ratio |
| `on_session_start` | Session begins |
| `on_session_shutdown` | Session ends |
| `on_agent_start` | Agent loop starts |
| `on_agent_end` | Agent loop completes |
| `on_turn_end` | Each agent turn completes |

Hooks can be shell commands (TOML config) or programmatic callbacks (Rust/Lua).

### Lua extensions (imp-lua)

Extensions are Lua scripts that register tools, hooks, and commands at runtime. Drop a `.lua` file in `~/.config/imp/lua/` or `<project>/.imp/lua/` and it loads automatically.

| Feature | Details |
|---------|---------|
| Custom tools | `imp.register_tool({ name, execute, params, ... })` — Lua functions that appear as native tools to the LLM |
| Hook handlers | `imp.on("before_tool_call", fn)` — intercept any hook event from Lua |
| Slash commands | `imp.register_command("name", { handler })` — add TUI slash commands |
| Shell execution | `imp.exec("cmd")` — run shell commands from Lua, get stdout/stderr/exit code |
| Inter-extension events | `imp.events.on("name", fn)` / `imp.events.emit("name", data)` — extensions communicate with each other |
| Hot reload | `reload()` drops all state and re-discovers/re-loads extensions |
| Error isolation | One extension crashing doesn't affect others — errors are reported, not fatal |
| JSON bridge | Automatic Lua table ↔ JSON conversion for tool params and results |

Discovery order:
1. `~/.config/imp/lua/*.lua` — user extensions
2. `~/.config/imp/lua/*/init.lua` — user extension directories
3. `<project>/.imp/lua/*.lua` — project-local extensions

```lua
-- ~/.config/imp/lua/timestamp.lua
imp.register_tool({
    name = "timestamp",
    description = "Returns the current Unix timestamp",
    readonly = true,
    params = {},
    execute = function(call_id, params, ctx)
        local result = imp.exec("date +%s")
        return { content = result.stdout }
    end
})
```

### Shell backends

| Backend | How it works |
|---------|-------------|
| `sh` (default) | Spawns `sh -c <command>` — standard process execution |
| `rush` | In-process execution via [rush](https://github.com/kfcafe/rush) library — built-in commands (ls, grep, cat, find, git) run without fork/exec |
| `rush-daemon` | Connects to a running rush daemon over Unix socket (planned) |

### Configuration

| Layer | Source | Scope |
|-------|--------|-------|
| 1 | Built-in defaults | Always |
| 2 | `~/.config/imp/config.toml` | Personal |
| 3 | `<project>/.imp/config.toml` | Per-repo |
| 4 | Environment variables | Per-session |
| 5 | CLI flags | Per-invocation |

### System prompt assembly

| Layer | Content |
|-------|---------|
| 1. Identity | "You are imp" + available tools list (filtered by mode/role) |
| 2. Project context | AGENTS.md files discovered from project and user config |
| 3. Skills | Available skill files with descriptions and paths |
| 4. Facts | Project facts from mana with verification timestamps |
| 5. Task | Unit title, description, verify command, previous attempts, dependencies (headless mode) |

## Crates

```
imp/
├── crates/
│   ├── imp-llm/    # Multi-provider LLM client (standalone)
│   ├── imp-core/   # Agent engine, tools, sessions, hooks
│   ├── imp-tui/    # Terminal UI (interactive mode)
│   ├── imp-cli/    # CLI binary
│   └── imp-lua/    # Lua scripting integration
```

**`imp-llm`** — Standalone LLM client. Streaming, prompt caching, model registry, OAuth. Works independently of the agent engine.

**`imp-core`** — The agent engine. Agent loop, tool registry, builder pattern, roles, modes, context management, session persistence, system prompt assembly, hook system.

**`imp-tui`** — Terminal UI with streaming output, input editing, slash commands, settings panel.

**`imp-cli`** — Entry point. Interactive TUI, headless task execution (`imp run <unit-id>`), RPC mode for integration.

## Modes

Modes control what an agent is allowed to do, enforced at both tool registration and execution time.

| Mode | Tools | Purpose |
|------|-------|---------|
| `full` | everything | Interactive use, trusted user |
| `worker` | read + write + bash | Execute a task, no coordination |
| `orchestrator` | read + mana + ask | Plan and delegate, no file writes |
| `planner` | read + mana (create) + ask | Decompose work, human approves |
| `reviewer` | read + ask | Read-only analysis |
| `auditor` | read + mana (read-only) | Inspect and report |

```toml
# .imp/config.toml
mode = "orchestrator"
```

Or via environment: `IMP_MODE=worker imp run 5.1`

## Configuration

Layered config, each level overrides the previous:

1. Built-in defaults
2. `~/.config/imp/config.toml` — personal defaults
3. `<project>/.imp/config.toml` — repo-shared settings
4. Environment variables (`IMP_MODEL`, `IMP_MODE`, `IMP_THINKING`)
5. CLI flags

```toml
# ~/.config/imp/config.toml
model = "sonnet"
thinking = "medium"
max_turns = 100

[context]
observation_mask_threshold = 0.6
compaction_threshold = 0.8
mask_window = 10

[shell]
backend = "rush"  # or "sh" (default)
```

## Providers

| Provider | Models |
|----------|--------|
| Anthropic | Claude Sonnet, Haiku, Opus |
| OpenAI | GPT-4o, o3, o3-mini |
| Google | Gemini 2.5 Pro, Flash |
| AWS Bedrock | Claude via Bedrock |
| xAI | Grok 3, Grok 2 |
| Groq | Llama 3.3 70B |

## License

Apache-2.0
