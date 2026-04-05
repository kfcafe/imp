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

Notable slash commands:
- `/new` — start a fresh in-memory session
- `/compact` — summarize older branch history into a structured handoff while preserving recent turns verbatim for future model context
- `/personality` — edit identity/behavior/profile settings

## What it does

imp is an agent engine — not a wrapper around an LLM API. It runs a full ReAct loop (think → act → observe → repeat), manages context intelligently, and gives the model real tools to work with.

**Tools** — File I/O, shell execution, code search (grep, find, AST scan), web search, diff preview/apply, user prompts, mana unit management, session search, and persistent memory. Readonly tools run in parallel. Prefer native tools over shell wrappers when available; for mana operations, use the built-in `mana` tool instead of `bash` for equivalent actions.

**Context management** — As conversations grow, imp first masks old tool outputs and can now compact older history behind an explicit branch-local compaction boundary. `/compact` preserves recent working turns verbatim, summarizes older work into a structured handoff, and makes future turns use the compacted active history rather than the full raw transcript. Raw session entries remain on disk for replay/fork/export.

**Modes** — Control what the agent can do. `full` for interactive use, `worker` for scoped tasks, `orchestrator` for planning and delegation, `reviewer` for read-only analysis. Enforced at both tool registration and execution time — disallowed tools never appear in the prompt. When delegating, imp should use mana child jobs as the worker substrate rather than inventing a separate subtask model.

**Sessions** — Every message, tool call, and result is persisted to append-only JSONL. Resume any session, fork from any point, navigate between branches.

**Extensions** — Drop a Lua script in `~/.config/imp/lua/` and it loads automatically. Register custom tools, hook into events, add slash commands. One extension crashing doesn't affect others.

## TUI

The interactive terminal UI gives you:

- Streaming responses with thinking indicators
- Command palette (`/`) with fuzzy search
- Personality editor (`/personality`) for identity, behavior, scope, and profiles
- Model selector (Ctrl+L) with quick cycling
- Thinking level control (Shift+Tab: off → minimal → low → medium → high → xhigh)
- Session tree view for branching conversations
- Manual `/compact` command for structured context compaction of long sessions
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
| `bash` | Shell execution in the workspace; prefer more specific native tools when available |
| `grep` | Regex search across files, respects .gitignore |
| `find` | Glob-based file search |
| `ls` | Directory listing |
| `diff` | Unified diff preview and patch application |
| `scan` | Tree-sitter AST extraction — types, functions, imports |
| `web` | Web search (Tavily/Exa) and page content extraction |
| `ask` | Prompt the user for input or multiple-choice |
| `mana` | Native mana work coordination — status, list/show, create/update/close/claim/release, logs/agents, next/tree, and run |
| `memory` | Persistent key-value store across sessions |
| `session_search` | Search past conversations from the local session index |

You can also define shell tools via TOML config, or register tools from Lua extensions.

## Providers

imp works with 11 LLM providers out of the box. Native integrations for Anthropic, OpenAI, and Google, plus any provider that speaks the OpenAI Chat Completions protocol.

| Provider | Models | Auth |
|----------|--------|------|
| Anthropic | Claude Sonnet 4.6, Haiku 4.5, Opus 4.6 | `ANTHROPIC_API_KEY` or OAuth |
| OpenAI | GPT-5.4, GPT-5.4 mini, GPT-5.4 nano, GPT-5.3 ChatGPT, GPT-5.3 Codex, plus custom model strings for preview/legacy models | `OPENAI_API_KEY` |
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

imp also uses provider-tuned default output caps when `max_tokens` is not set explicitly. That helps avoid accidentally requesting each model's absolute maximum completion size on every turn, which improves latency and reduces output-token spend while preserving explicit user overrides.

### Web search provider keys

The `web` tool supports Tavily, Exa, Linkup, and Perplexity.

You can configure Tavily and Exa in either of three ways:

```bash
# Option 1: environment variables
export TAVILY_API_KEY=tvly-...
export EXA_API_KEY=exa-...

# Option 2: save them in imp's auth store from the CLI
imp web-login tavily
imp web-login exa
imp secrets list
imp secrets show exa
imp secrets exa
imp secrets my-service
imp secrets rm my-service

# Option 3: inside the TUI app
# /secrets → choose a provider/service
# or /settings → Tavily API key / Exa API key fields
```

Saved keys now live in **imp's secure auth storage**: secret values go to your OS keychain when available, while `~/.config/imp/auth.json` keeps only metadata. Once saved, the `web` tool will auto-detect them even if you have not exported the env vars in your shell.

The first-run setup flow now also includes an optional web-search step where you can choose Tavily, Exa, or skip for now.

Provider selection order for the `web` tool:
1. explicit tool param (`provider = "exa"`)
2. `IMP_WEB_PROVIDER`
3. first available saved/env credential among Tavily, Exa, Linkup, Perplexity
4. default fallback (`tavily`)

```bash
# OAuth login
imp login              # Anthropic OAuth
imp login openai       # OpenAI / ChatGPT OAuth

# API/service secrets
imp secrets deepseek     # Prompts for api_key
imp secrets exa          # Prompts for api_key
imp secrets my-service   # Prompts for field names, then values
imp secrets list         # List configured secret providers
imp secrets show exa     # Inspect saved secret metadata (not values)
imp secrets rm my-service # Remove saved credentials
```

For arbitrary services, `imp secrets <provider>` stores named secret fields in imp's secure auth store. The flow is generic: you enter field names first (default `api_key`), then imp prompts for each value. Lua extensions can then read them with `imp.secret("provider", "field")` or `imp.secret_fields("provider")` without relying on `.env` files.

# Or just set the env var
export DEEPSEEK_API_KEY=sk-...

# Switch models
imp -m deepseek        # CLI flag
imp -m grok            # Aliases work
# or Ctrl+L in the TUI
```

## Configuration

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
mask_window = 10

[personality.profile.identity]
name = "imp"
work_style = "practical"
voice = "concise"
focus = "coding"
role = "agent"

[personality.profile.sliders]
autonomy = "high"
verbosity = "low"
caution = "high"
warmth = "medium"
planning_depth = "medium"

[personality.profiles]
active = "builder"

[personality.profiles.saved.builder.identity]
name = "imp"
work_style = "practical"
voice = "concise"
focus = "coding"
role = "agent"

[personality.profiles.saved.builder.sliders]
autonomy = "high"
verbosity = "low"
caution = "high"
warmth = "medium"
planning_depth = "medium"


[web]
search_provider = "exa"
```

Personality is layered through the same config stack as everything else:
- global personality in `~/.config/imp/config.toml`
- project personality in `<project>/.imp/config.toml`
- project values override global values for that repository

Use `/personality` in the TUI to edit:
- the identity sentence (`You are imp, a practical, concise, coding agent.`)
- 5-band behavior sliders
- global vs project scope
- named personality profiles

The built-in picker lists are structured rather than freeform:
- work style: practical, careful, disciplined, methodical, focused, thorough, precise, deliberate, skeptical, patient
- voice: concise, clear, direct, calm, thoughtful, collaborative, structured, friendly, terse, warm
- focus: coding, engineering, software, debugging, research, writing, planning, operations, analysis, general
- role: agent, assistant, worker, collaborator, partner, reviewer, planner

Slider bands use five stable labels:
- very low
- low
- balanced
- high
- very high


```bash
export IMP_WEB_PROVIDER=exa
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

## Delegating work with mana child jobs

When `imp` delegates work, it should create **mana child jobs** under the parent job instead of inventing a second todo or planning system.

Use a concise child-job description with:
- goal plus current-state framing
- clear scope boundaries, including out-of-scope notes
- one expected deliverable
- explicit patch or no-patch guidance
- important files or subsystem focus when known
- a concrete done condition and verify expectation

The full delegated child-job contract and reusable template live in `ARCHITECTURE.md`.

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

See `ARCHITECTURE.md` for design notes, including the delegated mana child-job contract for planner/orchestrator-authored work.

## Benchmarks

`imp-core` includes focused benchmarks for grep/search and other hot paths:

```bash
cargo bench -p imp-core --bench grep_vs_probe
cargo bench -p imp-core --bench core_hot_paths
```

Convenience wrapper:

```bash
cd imp && bash tools/run-benchmarks.sh
```

## Memory / leak checks

For local diagnostics on macOS:

```bash
cd imp && bash tools/run-leaks.sh
cd imp && bash tools/run-miri.sh
cd imp && bash tools/run-asan.sh
cd imp && bash tools/run-tsan.sh
cd imp && bash tools/run-stress.sh
```

See `imp/tools/README.md` for caveats and requirements.

## Integration with mana

imp is the worker engine for [mana](https://github.com/kfcafe/mana), a coordination substrate for AI coding agents. When mana dispatches a unit, imp runs it headlessly:

```bash
# mana calls this automatically
imp run <unit-id>
```

The agent reads the unit's title, description, and verify command, works through the task, and reports back. The verify gate must pass for the unit to close.

## Install

**Homebrew** (macOS + Linux):
```bash
brew tap kfcafe/tap && brew install imp
```

**Direct download** (Linux):
```bash
# x86_64
curl -LO https://github.com/kfcafe/imp/releases/latest/download/imp-0.1.0-x86_64-unknown-linux-gnu.tar.gz
tar xzf imp-0.1.0-x86_64-unknown-linux-gnu.tar.gz
sudo mv imp-0.1.0-x86_64-unknown-linux-gnu/imp /usr/local/bin/

# aarch64
curl -LO https://github.com/kfcafe/imp/releases/latest/download/imp-0.1.0-aarch64-unknown-linux-gnu.tar.gz
tar xzf imp-0.1.0-aarch64-unknown-linux-gnu.tar.gz
sudo mv imp-0.1.0-aarch64-unknown-linux-gnu/imp /usr/local/bin/
```

**From source:**
```bash
git clone https://github.com/kfcafe/imp.git
cd imp
cargo install --path crates/imp-cli
```

## License

Apache-2.0
