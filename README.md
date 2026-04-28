# imp

Rust-native coding agent runtime and terminal interface, built for tool use, durable sessions, and mana-coordinated software work.

```bash
brew tap kfcafe/tap && brew install imp
```

## Contents

- [Install](#install)
- [Quick start](#quick-start)
- [Mana](#mana)
- [Architecture](#architecture)
- [Tools](#tools)
- [Sessions and context](#sessions-and-context)
- [Modes](#modes)
- [Providers and secrets](#providers-and-secrets)
- [Web](#web)
- [Extensions](#extensions)
- [Configuration](#configuration)
- [Rust SDK](#rust-sdk)
- [Diagnostics](#diagnostics)
- [Influences](#influences)

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

For source upgrades, prefer:

```bash
uu install --default
```

On macOS, if a locally installed binary is killed immediately after install, re-sign it ad hoc:

```bash
bash tools/imp-fix-signature.sh ~/.local/imp-current/bin/imp
```

## Quick start

```bash
# Fullscreen terminal UI
imp

# CLI chat shell
imp chat

# One-shot prompt
imp -p "What does this project do?"

# Attach files as context
imp @src/main.rs "Explain this code"

# Continue the most recent session
imp -c

# Execute a mana unit
imp mana 5.1
```

TUI shortcuts:

| Input | Action |
|-------|--------|
| `/` | command palette |
| `@` | file finder / attach context |
| `Ctrl+L` | model selector |
| `Shift+Tab` | cycle thinking level |
| `/compact` | compact older branch history |
| `/settings` | edit UI/runtime settings |
| `/personality` | edit identity and behavior profile |

CLI shell shortcuts:

| Input | Action |
|-------|--------|
| `:help` | show shell commands |
| `@file` | attach a file |

## Mana

imp is the live agent runtime for mana-shaped work.

**mana** is the durable coordination layer: tasks, epics, facts, decisions, dependencies, workers, logs, and verification gates.

**imp** is the execution layer: model loop, tools, terminal UI, provider access, local sessions, context assembly, and runtime policy.

Preferred command:

```bash
imp mana <unit-id>
```

When running a mana unit, imp loads:

- title and description
- acceptance criteria
- verify command
- dependencies and status
- notes and decisions
- relevant project context

Then it executes the work, runs the verify gate, and records the outcome back into mana.

`imp run <unit-id>` remains available as a compatibility alias during migration.

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

Runtime boundary:

| Layer | Responsibility |
|-------|----------------|
| `imp-cli` | command parsing, login/setup flows, headless execution, RPC/chat entrypoints |
| `imp-tui` | cockpit UI, editor, views, rendering, interaction state |
| `imp-core` | agent loop, tool registry, session persistence, context, hooks, mana runtime integration |
| `imp-llm` | provider abstraction, streaming parsers, auth, model metadata |
| `imp-lua` | Lua extension loading, sandboxing, bridge APIs |
| `mana` | durable work graph, facts, decisions, verification, orchestration state |

Useful design docs:

- `ARCHITECTURE.md`
- `AGENTS.md`
- `imp_ontology.md`
- `vNext.md`
- `../docs/architecture/mana-platform-target-architecture.md`

## Tools

imp exposes native tools to the agent. Native tools are preferred over shell commands where available.

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

Readonly tools can run in parallel. Tools are filtered by mode before they are shown to the model and checked again at execution time.

## Sessions and context

Sessions are append-only JSONL records containing:

- user and assistant messages
- tool calls and tool results
- usage records
- branch metadata
- compaction entries
- checkpoint records

Supported session operations:

- continue recent work with `imp -c`
- fork/navigate branch history
- persist and reopen sessions
- compact older branch history with `/compact`
- preserve raw entries for replay/export/debugging

Context controls:

- old tool outputs can be masked when context is tight
- compaction preserves recent turns verbatim and summarizes older work
- mana units carry durable task context outside the chat transcript

## Modes

| Mode | Purpose |
|------|---------|
| `full` | normal interactive use |
| `worker` | execute a scoped task |
| `orchestrator` | plan/decompose and coordinate workers |
| `planner` | read, ask, and create structured work |
| `reviewer` | read-only code/design review |
| `auditor` | read-only inspection with mana visibility |

Set mode with config, env, or CLI where supported:

```bash
IMP_MODE=worker imp mana 5.1
```

Mode enforcement happens during tool registration and tool execution.

## Providers and secrets

imp includes native Anthropic, OpenAI, and Google integrations, plus OpenAI-compatible providers.

| Provider | Auth |
|----------|------|
| Anthropic | `ANTHROPIC_API_KEY` or OAuth |
| OpenAI / ChatGPT / Codex | `OPENAI_API_KEY` or OAuth where supported |
| Google | `GOOGLE_API_KEY` |
| Moonshot / Kimi | `MOONSHOT_API_KEY`, `KIMI_API_KEY`, or stored secret |
| DeepSeek | `DEEPSEEK_API_KEY` |
| Groq | `GROQ_API_KEY` |
| Cerebras | `CEREBRAS_API_KEY` |
| xAI | `XAI_API_KEY` |
| Mistral | `MISTRAL_API_KEY` |
| Together | `TOGETHER_API_KEY` |
| OpenRouter | `OPENROUTER_API_KEY` |
| Fireworks | `FIREWORKS_API_KEY` |

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

Secret storage:

| Platform | Store |
|----------|-------|
| macOS | Keychain |
| Linux | Secret Service, e.g. GNOME Keyring or KWallet |
| Windows | native credential store |

`~/.imp/auth.json` stores metadata only. Secret values are stored in the OS credential store.

## Web

The `web` tool supports Tavily, Exa, Linkup, and Perplexity.

Provider selection order:

1. explicit tool parameter
2. `IMP_WEB_PROVIDER`
3. first available saved/env credential
4. default fallback

```bash
export TAVILY_API_KEY=tvly-...
export EXA_API_KEY=exa-...
export IMP_WEB_PROVIDER=exa

imp web-login tavily
imp web-login exa
imp secrets exa
```

YouTube support:

- watch, shorts, embed, and `youtu.be` URLs
- public metadata extraction
- public captions/transcripts when available
- no `yt-dlp`, media download, or web-search API key required
- best-effort extraction; unavailable captions return metadata and diagnostics

## Extensions

### Lua

Lua is the current stable extension path.

Load paths:

- `~/.config/imp/lua/`
- `<project>/.imp/lua/`

Example:

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

Lua extensions can register tools, commands, and hooks. Capability policy controls extension access to shell, filesystem, HTTP, secrets, and native imp tools.

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

## Hooks

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
```

TUI configuration surfaces:

- `/settings` — display and runtime preferences
- `/personality` — identity, behavior sliders, global/project scope, profiles
- `/secrets` — provider/service credential setup

## Rust SDK

`imp-core` exposes an early SDK through `imp_core::sdk`.

Re-exported host-facing types:

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

## Diagnostics

Benchmarks:

```bash
cargo bench -p imp-core --bench core_hot_paths
bash tools/run-benchmarks.sh
```

Local diagnostics:

```bash
bash tools/run-leaks.sh
bash tools/run-miri.sh
bash tools/run-asan.sh
bash tools/run-tsan.sh
bash tools/run-stress.sh
```

See `tools/README.md` for requirements and caveats.

## Influences

imp's design draws from:

- Unix-style local tools and inspectable text workflows
- ReAct-style tool-using agents
- issue trackers and build systems with explicit verification gates
- terminal-native developer tools
- durable task graphs rather than transcript-only handoff
