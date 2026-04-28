# imp

`imp` is a Rust-native coding agent runtime for local software work. It gives a model native tools for reading, editing, searching, running commands, using git, reading the web, and coordinating durable work through mana.

`mana` is the durable work graph. `imp` is the live runtime that executes work against it.

```bash
brew tap kfcafe/tap && brew install imp
```

## Table of Contents

- [Quick Start](#quick-start)
- [Providers & Models](#providers--models)
- [Interactive Mode](#interactive-mode)
  - [Editor](#editor)
  - [Commands](#commands)
  - [Keyboard Shortcuts](#keyboard-shortcuts)
- [Sessions](#sessions)
  - [Branching](#branching)
  - [Compaction](#compaction)
- [Mana](#mana)
- [Tools](#tools)
- [Modes](#modes)
- [Settings](#settings)
- [Context](#context)
- [Customization](#customization)
  - [Lua Extensions](#lua-extensions)
  - [TypeScript Extensions](#typescript-extensions)
  - [Hooks](#hooks)
- [Programmatic Usage](#programmatic-usage)
- [Architecture](#architecture)
- [Development](#development)
- [Philosophy](#philosophy)
- [CLI Reference](#cli-reference)

---

## Quick Start

Install imp:

```bash
brew tap kfcafe/tap && brew install imp
```

Authenticate with an API key:

```bash
export ANTHROPIC_API_KEY=sk-ant-...
imp
```

Or use OAuth where supported:

```bash
imp login          # Anthropic OAuth
imp login openai   # OpenAI / ChatGPT OAuth
imp login kimi     # guided Kimi setup
```

Then talk to imp. The default interactive entrypoint opens the terminal UI:

```bash
imp
```

Other common entrypoints:

```bash
imp chat                         # CLI chat shell
imp -p "Summarize this repo"      # one-shot prompt
imp @src/main.rs "Explain this"   # prompt with file context
imp -c                            # continue recent session
imp run 12.1                      # execute a mana unit directly
```

Platform/source notes:

```bash
# Linux x86_64
curl -LO https://github.com/kfcafe/imp/releases/latest/download/imp-0.1.0-x86_64-unknown-linux-gnu.tar.gz
tar xzf imp-0.1.0-x86_64-unknown-linux-gnu.tar.gz
sudo mv imp-0.1.0-x86_64-unknown-linux-gnu/imp /usr/local/bin/

# Linux aarch64
curl -LO https://github.com/kfcafe/imp/releases/latest/download/imp-0.1.0-aarch64-unknown-linux-gnu.tar.gz
tar xzf imp-0.1.0-aarch64-unknown-linux-gnu.tar.gz
sudo mv imp-0.1.0-aarch64-unknown-linux-gnu/imp /usr/local/bin/

# source install
git clone https://github.com/kfcafe/imp.git
cd imp
uu install --default
```

If a locally installed macOS binary is killed immediately after install:

```bash
bash tools/imp-fix-signature.sh ~/.local/imp-current/bin/imp
```

---

## Providers & Models

imp includes native Anthropic, OpenAI, and Google integrations, plus OpenAI-compatible providers.

Common environment variables:

```bash
export ANTHROPIC_API_KEY=...
export OPENAI_API_KEY=...
export GOOGLE_API_KEY=...
export OPENROUTER_API_KEY=...
```

Common auth and secret commands:

```bash
imp login               # Anthropic OAuth
imp login openai        # OpenAI / ChatGPT OAuth
imp login kimi          # guided Kimi API-key setup

imp secrets moonshot    # store an API key securely
imp secrets list        # list configured providers
imp secrets show exa    # show metadata, not secret values
imp secrets doctor      # verify secure-storage references
```

Supported provider families include Anthropic, OpenAI/ChatGPT/Codex, Google, Moonshot/Kimi, DeepSeek, Groq, Cerebras, xAI, Mistral, Together, OpenRouter, Fireworks, and compatible APIs.

Secrets are stored in the OS credential store:

| Platform | Store |
|----------|-------|
| macOS | Keychain |
| Linux | Secret Service |
| Windows | native credential store |

`~/.imp/auth.json` stores metadata only.

### Web providers

The `web` tool supports Tavily, Exa, Linkup, and Perplexity.

```bash
export TAVILY_API_KEY=tvly-...
export EXA_API_KEY=exa-...
export IMP_WEB_PROVIDER=exa

imp web-login tavily
imp web-login exa
imp secrets exa
```

Provider selection order:

1. explicit tool parameter
2. `IMP_WEB_PROVIDER`
3. first available saved/env credential
4. default fallback

YouTube reading supports public metadata and captions/transcripts for watch, shorts, embed, and `youtu.be` URLs. It does not require `yt-dlp`, media download, or a web-search API key. Transcript extraction is best effort.

---

## Interactive Mode

Run:

```bash
imp
```

The terminal UI provides:

- message stream with assistant output and tool activity
- editor for prompts and steering
- command palette
- model and thinking controls
- file attachment via `@`
- session tree and branch navigation
- sidebar inspection for tool calls and outputs
- settings, personality, and secrets screens

### Editor

| Feature | How |
|---------|-----|
| File reference | type `@` to fuzzy-search project files |
| Command palette | type `/` |
| Model selector | `Ctrl+L` |
| Thinking level | `Shift+Tab` |
| Settings | `/settings` |
| Personality | `/personality` |
| Context compaction | `/compact` |

### Commands

Common built-in commands:

| Command | Purpose |
|---------|---------|
| `/settings` | edit display/runtime preferences |
| `/personality` | edit identity and behavior profile |
| `/compact` | summarize older branch history |
| `/secrets` | manage provider/service credentials |
| `/memory` | inspect or use persistent memory surfaces |
| `/recall` | search previous sessions |
| `/plan` | create or update mana-backed planning state |

The CLI chat shell has shell-style commands. Start it with:

```bash
imp chat
```

Useful shell inputs:

| Input | Purpose |
|-------|---------|
| `:help` | show available shell commands |
| `@file` | attach a file |

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+L` | open model selector |
| `Shift+Tab` | cycle thinking level |
| `/` | command palette |
| `@` | attach file context |
| `Esc` | cancel/escape current UI state where supported |

---

## Sessions

Sessions are stored as append-only JSONL records. Entries include messages, tool calls, tool results, usage records, branch metadata, compaction entries, and checkpoint records.

```bash
imp -c      # continue most recent session
imp chat    # interactive CLI shell with persistent session support
imp         # TUI session interface
```

### Branching

imp preserves branch metadata so you can continue from prior points without destroying the original history. The raw session file remains available for replay, export, or debugging.

### Compaction

Long sessions can exhaust context windows. `/compact` summarizes older branch history while preserving recent working turns. The full raw history remains in the session file; compaction affects what is sent forward as model context.

Context controls include:

- masking old tool outputs when context is tight
- branch-local compaction boundaries
- mana units carrying durable task context outside the chat transcript

---

## Mana

Mana is a local-first work coordination system for coding agents. It turns work into durable records with explicit scope, dependencies, verification gates, attempts, notes, decisions, and facts.

imp is the runtime that executes those records.

```bash
mana init
mana create "Fix CSV export" --verify "cargo test csv::export"
imp
# ask: "work on 12.1"
```

### Why mana

Agent work is fragile when plans, failures, and completion criteria live only in chat. Mana gives work a durable shape:

- **units** describe the work
- **verify gates** define completion
- **dependencies** encode order
- **attempts and notes** preserve execution history
- **facts and decisions** capture verified project memory

### Running mana work with imp

For most use, open imp and ask it to work from mana:

```bash
imp
# "work on 12.1"
# "show me the next task and work on it"
```

For direct CLI execution:

```bash
mana show 12.1
imp run 12.1
mana show 12.1
```

When executing a mana unit, imp reads the title, description, acceptance criteria, verify command, dependencies, notes, decisions, and project context. It performs the work, runs the verify gate, and records the result.

A good verify gate is targeted and decisive:

```bash
cargo test -p imp-llm auth::token_validation
```

A weak verify gate is broad or ambiguous:

```bash
cargo test
```

`imp mana <unit-id>` is also supported as an explicit mana subcommand, but `imp run <unit-id>` is the clearer direct-execution command.

---

## Tools

imp exposes native tools to the agent. Native tools are preferred over shell commands when available.

| Tool | Purpose |
|------|---------|
| `read` | read text files and images with offset/limit support |
| `write` | create or overwrite files |
| `edit` | exact find/replace edits |
| `multi_edit` | multiple coordinated edits to one file |
| `bash` | shell execution with timeout/cancellation |
| `git` | status, diff, log, stage, commit, restore, worktrees |
| `scan` | tree-sitter structural code extraction |
| `web` | web search, page read, YouTube metadata/caption extraction |
| `ask` | ask the user for input or choices |
| `mana` | inspect/update/create/close/claim/release/run mana units |
| `memory` | persistent memory across sessions |
| `session_search` | search local conversation history |

Readonly tools can run in parallel. Tools are filtered by mode before they are shown to the model and checked again during execution.

---

## Modes

Modes control tool visibility and execution policy.

| Mode | Purpose |
|------|---------|
| `full` | normal interactive use |
| `worker` | execute a scoped task |
| `orchestrator` | plan/decompose and coordinate workers |
| `planner` | read, ask, and create structured work |
| `reviewer` | read-only code/design review |
| `auditor` | read-only inspection with mana visibility |

Example:

```bash
IMP_MODE=reviewer imp chat
IMP_MODE=worker imp run 5.1
```

Disallowed tools are not included in the model prompt and are still blocked at execution time.

---

## Settings

Configuration precedence, lowest to highest:

1. built-in defaults
2. `~/.config/imp/config.toml`
3. `<project>/.imp/config.toml`
4. environment variables such as `IMP_MODEL`, `IMP_MODE`, `IMP_THINKING`
5. CLI flags

Example:

```toml
model = "sonnet"
thinking = "medium"
max_turns = 100
max_tokens = 2048

[context]
observation_mask_threshold = 0.6
mask_window = 10

[web]
search_provider = "exa"

[ui]
notify_on_agent_complete = true
```

TUI settings surfaces:

- `/settings` — display and runtime preferences
- `/personality` — identity, behavior sliders, global/project scope, profiles
- `/secrets` — provider/service credential setup

---

## Context

imp assembles context from the active session, attached files, project instructions, tool results, memory, and mana task state.

Project instruction files:

- `AGENTS.md`
- local `AGENTS.md` files closer to the code being changed
- future/project-local `.imp` resources where configured

File context:

```bash
imp @README.md "Improve this document"
imp @crates/imp-core/src/agent.rs "Explain the turn loop"
```

Long-session context:

- `/compact` summarizes older branch history
- recent working turns stay available
- raw session history remains on disk
- mana units keep durable task state outside the transcript

---

## Customization

imp can be customized with Lua extensions today, TypeScript compatibility where supported, configuration, hooks, project instructions, and skills.

### Lua Extensions

Lua is the current stable extension path.

Load paths:

- `~/.config/imp/lua/`
- `<project>/.imp/lua/`

Register a tool:

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
```

Register a command:

```lua
imp.register_command("greet", {
    description = "Say hello",
    handler = function(args) return "Hello, " .. (args or "world") end
})
```

Register a hook:

```lua
imp.on("after_file_write", function(event)
    imp.exec("cargo fmt -- " .. event.path)
end)
```

Capability policy controls extension access to shell, filesystem, HTTP, secrets, and native imp tools.

### TypeScript Extensions

TypeScript extension support is a limited compatibility and forward-direction layer, not full Pi API parity.

```bash
imp import --from pi
```

Current support:

- Bun-backed `.ts` entrypoints
- Pi-style `registerTool(...)`
- common TypeBox-style schemas
- text/details tool results
- limited `session_start` lifecycle hooks for dynamic tool registration

Limited/stubbed:

- `registerCommand(...)`
- `ctx.ui.notify(...)` / `ctx.ui.setStatus(...)`
- `sessionManager.getBranch()` / `getEntries()`
- rich custom UI renderers
- built-in tool overrides
- unrestricted filesystem/network/process/env/secrets/native access

imp should notify and ask before running Bun install commands; it must not install dependencies silently during import.

### Hooks

Shell hook commands support placeholder interpolation. Quote placeholders that may contain shell-sensitive text, for example `'{command}'` or `"{command}"`, so imp can escape the value as one shell argument before executing `sh -c`.

| Event | When |
|-------|------|
| `before_tool_call` | before a tool executes; can block |
| `after_tool_call` | after a tool completes; can modify result |
| `after_file_write` | after write/edit modifies a file |
| `before_llm_call` | before each model request |
| `on_context_threshold` | context usage crosses a configured ratio |
| `on_session_start` / `on_session_shutdown` | session lifecycle |
| `on_agent_start` / `on_agent_end` | agent loop lifecycle |
| `on_turn_end` | each agent turn completes |

---

## Programmatic Usage

### Rust SDK

`imp-core` exposes an early SDK through `imp_core::sdk`.

```rust,no_run
use imp_core::sdk::{AgentEvent, ImpSession, Result, SessionOptions};

#[tokio::main]
async fn main() -> Result<()> {
    let mut session = ImpSession::create(SessionOptions {
        cwd: std::env::current_dir()?,
        ..Default::default()
    })
    .await?;

    session.prompt("Summarize this repository.").await?;

    while let Some(event) = session.recv_event().await {
        if let AgentEvent::AgentEnd { .. } = event {
            break;
        }
    }

    session.wait().await
}
```

Host-facing types include `ImpSession`, `SessionOptions`, `SessionChoice`, `AgentEvent`, `UserInterface`, `ThinkingLevel`, `Model`, and core `Result` / `Error` types.

See `crates/imp-core/examples/sdk_session.rs` for a working example.

### RPC / process integration

imp has CLI/RPC-oriented surfaces in active development. Prefer the Rust SDK for embedded Rust hosts, `imp` / `imp chat` for human-operated local workflows, and `imp run <unit-id>` for direct mana unit execution.

---

## Architecture

```text
imp/
├── crates/
│   ├── imp-cli     CLI entry point, TUI launch, chat/headless/RPC modes
│   ├── imp-core    Agent loop, tools, sessions, hooks, context, SDK
│   ├── imp-llm     Streaming LLM client, providers, model registry, auth
│   ├── imp-lua     Lua extension runtime
│   └── imp-tui     Fullscreen terminal UI
```

Boundary summary:

| Layer | Responsibility |
|-------|----------------|
| `imp-cli` | command parsing, setup/login, chat/headless/RPC entrypoints |
| `imp-tui` | terminal UI, editor, views, rendering, interaction state |
| `imp-core` | agent loop, tools, session persistence, context, hooks, mana integration |
| `imp-llm` | providers, streaming parsers, model metadata, auth |
| `imp-lua` | Lua extension loading, sandboxing, bridge APIs |
| `mana` | durable work graph, facts, decisions, dependencies, verification, orchestration state |

Design docs:

- `ARCHITECTURE.md`
- `AGENTS.md`
- `imp_ontology.md`
- `vNext.md`
- `../docs/architecture/mana-platform-target-architecture.md`

---

## Development

Benchmarks:

```bash
cargo bench -p imp-core --bench core_hot_paths
bash tools/run-benchmarks.sh
```

Diagnostics:

```bash
bash tools/run-leaks.sh
bash tools/run-miri.sh
bash tools/run-asan.sh
bash tools/run-tsan.sh
bash tools/run-stress.sh
```

See `tools/README.md` for requirements and caveats.

---

## Philosophy

imp keeps the runtime local, inspectable, and tool-native. The core should provide reliable execution, sessions, context, policy, and mana integration; project-specific workflow belongs in mana units, instructions, skills, hooks, and extensions.

Influences include Unix-style local tools, ReAct-style tool-use loops, issue trackers with explicit verification gates, terminal-native developer workflows, and durable task graphs for handoff.

---

## CLI Reference

Common commands:

```bash
imp                         # open TUI
imp chat                    # open CLI chat shell
imp -p "prompt"              # one-shot prompt
imp @file "prompt"           # prompt with file context
imp -c                       # continue most recent session
imp run <unit-id>            # execute mana unit directly
imp mana <unit-id>           # explicit mana subcommand, also supported

imp login [provider]         # OAuth / guided auth
imp secrets list             # list saved credential metadata
imp secrets show <provider>  # show metadata, not secret values
imp secrets doctor           # check secure-storage references
imp web-login <provider>     # save web provider credential
imp import --from pi         # import supported Pi TypeScript extensions
```

Environment variables:

```bash
IMP_MODEL=sonnet
IMP_MODE=worker
IMP_THINKING=medium
IMP_WEB_PROVIDER=exa
ANTHROPIC_API_KEY=...
OPENAI_API_KEY=...
GOOGLE_API_KEY=...
```
