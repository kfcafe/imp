# imp

An AI coding agent that runs in your terminal. Give it a task, it thinks through it, calls tools, and gets work done.

```
brew tap kfcafe/tap && brew install imp
```

## Quick start

```bash
# Start an interactive session
imp

# Ask a one-shot question
imp -p "What does this project do?"

# Include files as context
imp @src/main.rs "Explain this code"

# Run a mana unit headlessly
imp run 5.1

# Continue your last session
imp -c
```

Type `/` in the editor to open the command palette. Arrow keys, Tab, or Ctrl+N/P to navigate. Enter to select.

## What it does

imp is an agent engine — not a wrapper around an LLM API. It runs a full ReAct loop (think → act → observe → repeat), manages context intelligently, and gives the model real tools to work with.

**Tools** — File I/O, shell execution, code search (grep, find, AST scan), web search, diff preview/apply, user prompts, mana unit management, and persistent memory. Readonly tools run in parallel.

**Context management** — As conversations grow, imp masks old tool outputs, keeps a sliding window of recent turns, and compacts the full conversation via LLM summarization when context hits 80%. The original task is re-injected after compaction so the model never loses the goal.

**Modes** — Control what the agent can do. `full` for interactive use, `worker` for scoped tasks, `orchestrator` for planning and delegation, `reviewer` for read-only analysis. Enforced at both tool registration and execution time — disallowed tools never appear in the prompt.

**Sessions** — Every message, tool call, and result is persisted to append-only JSONL. Resume any session, fork from any point, navigate between branches.

**Extensions** — Drop a Lua script in `~/.config/imp/lua/` and it loads automatically. Register custom tools, hook into events, add slash commands. One extension crashing doesn't affect others.

## TUI

The interactive terminal UI gives you:

- Streaming responses with thinking indicators
- Command palette (`/`) with fuzzy search
- Model selector (Ctrl+L) with quick cycling
- Thinking level control (Shift+Tab: off → minimal → low → medium → high → xhigh)
- Session tree view for branching conversations
- Sidebar for tool output inspection
- Input history, multi-line editing, file finder (`@`)
- Mouse support for scrolling and clicking tool calls

## Tools

| Tool | What it does |
|------|-------------|
| `read` | Read files (text + images), with offset/limit for large files |
| `write` | Create or overwrite files, auto-creates directories |
| `edit` | Find-and-replace with exact text matching |
| `multi_edit` | Multiple edits to one file in a single call |
| `bash` | Shell execution with timeout and streaming output |
| `grep` | Regex search across files, respects .gitignore |
| `find` | Glob-based file search |
| `ls` | Directory listing |
| `diff` | Unified diff preview and patch application |
| `scan` | Tree-sitter AST extraction — types, functions, imports |
| `web` | Web search (Tavily/Exa) and page content extraction |
| `ask` | Prompt the user for input or multiple-choice |
| `mana` | Unit management — create, update, close, status |
| `memory` | Persistent key-value store across sessions |

You can also define shell tools via TOML config, or register tools from Lua extensions.

## Providers

imp works with 11 LLM providers out of the box. Native integrations for Anthropic, OpenAI, and Google, plus any provider that speaks the OpenAI Chat Completions protocol.

| Provider | Models | Auth |
|----------|--------|------|
| Anthropic | Claude Sonnet 4.6, Haiku 4.5, Opus 4.6 | `ANTHROPIC_API_KEY` or OAuth |
| OpenAI | GPT-4o, o3, o4-mini | `OPENAI_API_KEY` |
| Google | Gemini 2.5 Pro, Flash | `GOOGLE_API_KEY` |
| DeepSeek | DeepSeek V3, R1 | `DEEPSEEK_API_KEY` |
| Groq | Llama 3.3 70B | `GROQ_API_KEY` |
| Cerebras | Llama 3.3 70B | `CEREBRAS_API_KEY` |
| xAI | Grok 3, Grok 3 Mini | `XAI_API_KEY` |
| Mistral | Mistral Large, Codestral | `MISTRAL_API_KEY` |
| Together | Llama 3.3 70B Turbo, Qwen 2.5 72B | `TOGETHER_API_KEY` |
| OpenRouter | Any model via OpenRouter | `OPENROUTER_API_KEY` |
| Fireworks | Llama 3.3 70B | `FIREWORKS_API_KEY` |

Set an env var and it's auto-detected — no login step needed. Prompt caching is automatic on Anthropic (system prompt, tools, recent turns).

```bash
# Login (stores key locally)
imp login              # Anthropic OAuth
imp login deepseek     # Prompts for API key
imp login cerebras     # Any provider works

# Or just set the env var
export DEEPSEEK_API_KEY=sk-...

# Switch models
imp -m deepseek        # CLI flag
imp -m grok            # Aliases work
# or Ctrl+L in the TUI
```

## Configuration

Layered — each level overrides the previous:

1. Built-in defaults
2. `~/.config/imp/config.toml` — personal
3. `<project>/.imp/config.toml` — per-repo
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
```

## Modes

| Mode | Allowed tools | Purpose |
|------|--------------|---------|
| `full` | everything | Interactive use |
| `worker` | read, write, bash, ask | Execute a scoped task |
| `orchestrator` | read, mana, ask | Plan and delegate |
| `planner` | read, mana (create), ask | Decompose work |
| `reviewer` | read, ask | Read-only analysis |
| `auditor` | read, mana (read-only) | Inspect and report |

```bash
IMP_MODE=worker imp run 5.1
```

## Lua extensions

Drop scripts in `~/.config/imp/lua/` or `<project>/.imp/lua/`:

```lua
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

imp.register_command("greet", {
    description = "Say hello",
    handler = function(args) return "Hello, " .. (args or "world") end
})

imp.on("after_file_write", function(event)
    imp.exec("cargo fmt -- " .. event.path)
end)
```

## Hooks

| Event | When |
|-------|------|
| `before_tool_call` | Before tool executes — can block |
| `after_tool_call` | After tool completes — can modify result |
| `after_file_write` | After write/edit modifies a file |
| `before_llm_call` | Before each LLM request |
| `on_context_threshold` | Context usage crosses a configured ratio |
| `on_session_start/shutdown` | Session lifecycle |
| `on_agent_start/end` | Agent loop lifecycle |
| `on_turn_end` | Each agent turn completes |

## Architecture

```
imp/
├── crates/
│   ├── imp-llm     Streaming LLM client, model registry, OAuth
│   ├── imp-core    Agent engine, tools, sessions, hooks, context
│   ├── imp-tui     Terminal UI
│   ├── imp-lua     Lua scripting runtime
│   └── imp-cli     Binary entry point
```

**imp-llm** is standalone — you can use it as a Rust library for streaming LLM access without the agent engine.

## Integration with mana

imp is the worker engine for [mana](https://github.com/kfcafe/mana), a coordination substrate for AI coding agents. When mana dispatches a unit, imp runs it headlessly:

```bash
# mana calls this automatically
imp run <unit-id>
```

The agent reads the unit's title, description, and verify command, works through the task, and reports back. The verify gate must pass for the unit to close.

## Install

**Homebrew** (macOS arm64):
```bash
brew tap kfcafe/tap && brew install imp
```

**From source:**
```bash
git clone https://github.com/kfcafe/imp.git
cd imp
cargo install --path crates/imp-cli
```

## License

Apache-2.0
