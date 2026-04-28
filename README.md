# imp

`imp` is a Rust-native coding agent runtime and terminal interface for local software work.

It combines:

- a terminal UI and CLI chat shell
- a tool-using agent loop
- durable JSONL sessions
- native code, shell, git, web, and mana tools
- provider auth and secure secret storage
- Lua extensions
- mana-backed task execution

`mana` is the durable work graph: tasks, epics, facts, decisions, dependencies, workers, logs, and verify gates. `imp` is the live execution layer that reads mana work, runs tools, and writes outcomes back.

```bash
brew tap kfcafe/tap && brew install imp
```

## Contents

- [Overview](#overview)
- [Install](#install)
- [Core workflow](#core-workflow)
- [Mana workflow](#mana-workflow)
- [Typical workflows](#typical-workflows)
- [Runtime architecture](#runtime-architecture)
- [Tool surface](#tool-surface)
- [Sessions and context](#sessions-and-context)
- [Modes and policy](#modes-and-policy)
- [Providers and secrets](#providers-and-secrets)
- [Configuration](#configuration)
- [Extensions](#extensions)
- [Stable vs preview](#stable-vs-preview)
- [Rust SDK](#rust-sdk)
- [Development](#development)
- [Influences](#influences)

## Overview

imp is intended for agentic coding work where the agent needs to inspect files, edit code, run commands, manage context, and coordinate durable tasks.

Primary entrypoints:

| Command | Purpose |
|---------|---------|
| `imp` | open the fullscreen terminal UI |
| `imp chat` | open the CLI chat shell |
| `imp -p "..."` | run a one-shot prompt |
| `imp @file "..."` | prompt with file context |
| `imp -c` | continue the most recent session |
| `imp mana <unit-id>` | execute a mana unit |

Core concepts:

| Concept | Meaning |
|---------|---------|
| Agent runtime | model loop, tool calls, policy checks, context assembly |
| Session | append-only local JSONL transcript with branches and tool records |
| Tool | native action available to the model, such as read/edit/git/web/mana |
| Mode | permission profile controlling which tools are visible and executable |
| Mana unit | durable task/epic/fact/decision in the mana work graph |
| Verify gate | command or check that proves a mana unit is complete |

## Install

### Homebrew

```bash
brew tap kfcafe/tap && brew install imp
```

### Linux release archive

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

### From source

```bash
git clone https://github.com/kfcafe/imp.git
cd imp
uu install --default
imp
```

Raw Cargo install:

```bash
cargo install --path .
~/.cargo/bin/imp install-local
imp
```

For source upgrades:

```bash
uu install --default
```

macOS local signing fallback:

```bash
bash tools/imp-fix-signature.sh ~/.local/imp-current/bin/imp
```

## Core workflow

### Terminal UI

```bash
imp
```

Common controls:

| Input | Action |
|-------|--------|
| `/` | command palette |
| `@` | file finder / attach context |
| `Ctrl+L` | model selector |
| `Shift+Tab` | cycle thinking level |
| `/compact` | compact older branch history |
| `/settings` | edit UI/runtime settings |
| `/personality` | edit identity and behavior profile |

### CLI chat shell

```bash
imp chat
```

Common shell inputs:

| Input | Action |
|-------|--------|
| `:help` | show shell commands |
| `@file` | attach a file |

### One-shot prompt

```bash
imp -p "Summarize this repository"
imp @src/main.rs "Explain this file"
imp -c
```

## Mana workflow

imp is the live worker/runtime for mana units.

```bash
imp mana <unit-id>
```

When executing a mana unit, imp reads:

- title and description
- acceptance criteria
- verify command
- dependency/status metadata
- notes and decisions
- relevant project context

Then imp performs the work, runs the verify gate, and records the result.

Example mana-shaped work:

```text
Title: Add validation for empty API tokens
Acceptance: Empty or whitespace-only tokens are rejected with a user-facing error.
Verify: cargo test -p imp-llm auth::token_validation
```

Run it:

```bash
imp mana 12.1
```

A verify gate should be specific enough that a passing check means the unit is complete. Prefer targeted commands over broad or ambiguous checks.

Compatibility note: `imp run <unit-id>` remains available during migration, but `imp mana <unit-id>` is the preferred command.

## Typical workflows

### Interactive coding session

```bash
imp
# attach files with @
# ask for a plan or implementation
# inspect tool calls in the sidebar
# run targeted checks before finishing
```

### Run a mana task

```bash
mana show 12.1
imp mana 12.1
mana show 12.1
```

Use mana for work that benefits from durable state, acceptance criteria, dependencies, logs, or handoff.

### Review-only mode

```bash
IMP_MODE=reviewer imp chat
```

Reviewer mode is read-only. Mutation tools are not shown to the model and are still blocked at execution time.

### Attach context and compact long sessions

```bash
imp @crates/imp-core/src/agent.rs "Find the turn-loop entrypoints"
# later, in the TUI:
/compact
```

Compaction summarizes older branch history while keeping recent working turns available.

### Add a Lua tool

Create a file under `~/.config/imp/lua/` or `<project>/.imp/lua/`:

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

## Runtime architecture

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

## Tool surface

Native tools are preferred over shell commands when available.

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

## Sessions and context

Sessions are append-only JSONL records containing:

- user and assistant messages
- tool calls and tool results
- usage records
- branch metadata
- compaction entries
- checkpoint records

Supported operations:

- continue recent work with `imp -c`
- fork or navigate branch history
- persist and reopen sessions
- compact older branch history with `/compact`
- preserve raw entries for replay, export, or debugging

Context controls:

- old tool outputs can be masked when context is tight
- compaction preserves recent turns and summarizes older work
- mana units carry durable task context outside the chat transcript

## Modes and policy

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
IMP_MODE=worker imp mana 5.1
```

Mode enforcement happens during tool registration and execution. Disallowed tools are not included in the model prompt.

## Providers and secrets

imp includes native Anthropic, OpenAI, and Google integrations, plus OpenAI-compatible providers.

Common environment variables:

```bash
export ANTHROPIC_API_KEY=...
export OPENAI_API_KEY=...
export GOOGLE_API_KEY=...
export OPENROUTER_API_KEY=...
```

Common auth commands:

```bash
imp login               # Anthropic OAuth
imp login openai        # OpenAI / ChatGPT OAuth
imp login kimi          # guided Kimi API-key setup

imp secrets moonshot    # store an API key securely
imp secrets list        # list configured providers
imp secrets show exa    # show metadata, not secret values
imp secrets doctor      # verify secure-storage references
```

Supported provider families include Anthropic, OpenAI/ChatGPT/Codex, Google, Moonshot/Kimi, DeepSeek, Groq, Cerebras, xAI, Mistral, Together, OpenRouter, and Fireworks.

Secret values are stored in the OS credential store:

| Platform | Store |
|----------|-------|
| macOS | Keychain |
| Linux | Secret Service |
| Windows | native credential store |

`~/.imp/auth.json` stores metadata only.

### Web credentials

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

## Configuration

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

TUI configuration surfaces:

- `/settings` — display and runtime preferences
- `/personality` — identity, behavior sliders, global/project scope, profiles
- `/secrets` — provider/service credential setup

## Extensions

### Lua

Lua is the current stable extension path.

Load paths:

- `~/.config/imp/lua/`
- `<project>/.imp/lua/`

Lua extensions can register tools, commands, and hooks. Capability policy controls extension access to shell, filesystem, HTTP, secrets, and native imp tools.

Example command:

```lua
imp.register_command("greet", {
    description = "Say hello",
    handler = function(args) return "Hello, " .. (args or "world") end
})
```

Example hook:

```lua
imp.on("after_file_write", function(event)
    imp.exec("cargo fmt -- " .. event.path)
end)
```

### TypeScript

TypeScript extension support is a compatibility and forward-direction layer, not full Pi API parity.

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

## Stable vs preview

| Area | Status |
|------|--------|
| Rust runtime and core agent loop | stable active surface |
| TUI and CLI chat shell | stable active surface |
| Sessions, branching, and compaction | stable active surface |
| Native tools | stable active surface |
| Provider auth and secure secrets | stable active surface |
| Lua extensions | stable shipped extension path |
| Mana unit execution | stable active surface |
| TypeScript/Pi extension compatibility | limited compatibility layer |
| Rust SDK | preview |
| Broader mana orchestration boundaries | active migration area |

## Rust SDK

`imp-core` exposes an early SDK through `imp_core::sdk`.

Host-facing types include:

- `ImpSession`
- `SessionOptions`
- `SessionChoice`
- `AgentEvent`
- `UserInterface`
- `ThinkingLevel`
- `Model`
- `Result` / `Error`

Example:

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

See `crates/imp-core/examples/sdk_session.rs` for a working example.

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

## Influences

imp draws on Unix-style local tooling, ReAct-style tool-use loops, issue trackers with explicit verification, terminal-native developer workflows, and durable task graphs for handoff.
