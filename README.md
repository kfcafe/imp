# imp

`imp` is an extensible coding agent built for efficiency and performance. It runs in your terminal, uses native development tools, keeps durable sessions, and can coordinate longer tasks with an included mana work graph.

imp is actively developed. The `0.2.0` release line includes the workflow runtime foundations: structured run artifacts, evidence/trace emission, verification gates, and policy/trust events around tool execution. Some extension and embedding surfaces are still evolving.

```bash
brew tap kfcafe/tap && brew install imp
```

## Table of Contents

- [Quick Start](#quick-start)
- [What imp can do](#what-imp-can-do)
- [Interactive Workflow](#interactive-workflow)
- [Sessions and Context](#sessions-and-context)
- [Tools](#tools)
- [Mana](#mana)
- [Providers and Models](#providers-and-models)
- [Modes and Policy](#modes-and-policy)
- [Settings](#settings)
- [Customization](#customization)
- [Programmatic Usage](#programmatic-usage)
- [Architecture](#architecture)
- [Development](#development)
- [CLI Reference](#cli-reference)
- [License](#license)

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

Common entrypoints:

```bash
imp                              # terminal UI
imp chat                         # CLI chat shell
imp -p "Summarize this repo"      # one-shot prompt
imp @src/main.rs "Explain this"   # prompt with file context
imp -c                            # continue recent session
imp run 12.1                      # execute a mana task directly
```

Source install:

```bash
git clone https://github.com/kfcafe/imp.git
cd imp
uu install --default
```

Linux release archives:

```bash
# x86_64
curl -LO https://github.com/kfcafe/imp/releases/latest/download/imp-0.2.0-x86_64-unknown-linux-gnu.tar.gz
tar xzf imp-0.2.0-x86_64-unknown-linux-gnu.tar.gz
sudo mv imp-0.2.0-x86_64-unknown-linux-gnu/imp /usr/local/bin/

# aarch64
curl -LO https://github.com/kfcafe/imp/releases/latest/download/imp-0.2.0-aarch64-unknown-linux-gnu.tar.gz
tar xzf imp-0.2.0-aarch64-unknown-linux-gnu.tar.gz
sudo mv imp-0.2.0-aarch64-unknown-linux-gnu/imp /usr/local/bin/
```

If a locally installed macOS binary is killed immediately after install:

```bash
bash tools/imp-fix-signature.sh ~/.local/imp-current/bin/imp
```

---

## What imp can do

| Area | Features |
|------|----------|
| Interactive coding | terminal UI, CLI chat shell, one-shot prompts, file attachments |
| Native tools | read, write, edit, shell, git, structural code scan, web, ask, memory, mana |
| Sessions | persistent JSONL sessions, continuation, branch history, compaction |
| Context | attached files, project instructions, memory, prior sessions, mana task state |
| Providers | Anthropic, OpenAI, Google, OpenAI-compatible providers, OAuth/API-key auth |
| Secrets | OS credential-store integration; metadata only in `~/.imp/auth.json` |
| Modes | full, worker, orchestrator, planner, reviewer, auditor |
| Extensions | Lua tools/commands/hooks; limited TypeScript compatibility layer |
| Task coordination | included mana work graph for tasks, dependencies, notes, decisions, verify gates |

Typical use:

```bash
imp
# ask: "fix the failing auth test"
# ask: "review this branch"
# ask: "work on 12.1"
# ask: "show me the next task and work on it"
```

---

## Interactive Workflow

Run:

```bash
imp
```

The terminal UI provides:

- streaming assistant output and tool activity
- prompt editor with command palette
- file attachment via `@`
- model and thinking controls
- session tree and branch navigation
- sidebar inspection for tool calls and outputs
- settings, personality, and secrets screens

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

CLI chat shell:

```bash
imp chat
```

| Input | Purpose |
|-------|---------|
| `:help` | show available shell commands |
| `@file` | attach a file |

---

## Sessions and Context

Sessions are stored as append-only JSONL records. Entries include:

- user and assistant messages
- tool calls and tool results
- usage records
- branch metadata
- compaction entries
- checkpoint records

Session commands:

```bash
imp -c      # continue most recent session
imp chat    # CLI shell with persistent sessions
imp         # terminal UI session interface
```

Long-session support:

- `/compact` summarizes older branch history while preserving recent working turns
- old tool outputs can be masked when context is tight
- raw session history remains on disk for replay/export/debugging
- mana tasks can carry durable task state outside the chat transcript

Project context can come from:

- attached files: `imp @README.md "Improve this"`
- project instructions such as `AGENTS.md`
- local memory and previous sessions
- active mana task state

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

## Mana

Mana is included task coordination for longer-running agent work.

If you have never used mana: think of it as a local work graph for agents. A mana task can store what needs to happen, why it matters, what depends on it, what was tried, and what command proves it is done.

Mana records can include:

- tasks and epics
- acceptance criteria
- verify gates
- dependencies
- notes and attempts
- decisions and facts
- worker/run state

Most users can work with mana from inside imp:

```bash
imp
# ask: "work on 12.1"
# ask: "show me the next task and work on it"
```

Direct CLI execution is also available:

```bash
imp run 12.1
```

Example mana-shaped task:

```text
Title: Add validation for empty API tokens
Acceptance: Empty or whitespace-only tokens are rejected with a user-facing error.
Verify: cargo test -p imp-llm auth::token_validation
```

A good verify gate is targeted and decisive:

```bash
cargo test -p imp-llm auth::token_validation
```

`imp mana <unit-id>` is also supported as an explicit mana subcommand, but `imp run <unit-id>` is the clearer direct-execution command.

---

## Providers and Models

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

YouTube reading supports public metadata and captions/transcripts for watch, shorts, embed, and `youtu.be` URLs. It does not require `yt-dlp`, media download, or a web-search API key. Transcript extraction is best effort.

---

## Modes and Policy

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

## Slash-command Skills

Discovered skills are directly invocable from the slash menu. A skill at `.imp/skills/deploy/SKILL.md` or `~/.config/imp/skills/deploy/SKILL.md` creates `/deploy` unless a built-in or Lua command already uses that name. Use `/skill:deploy` to explicitly invoke the skill when a name is ambiguous.

Invoking a skill inserts its `SKILL.md` instructions into the prompt for the next agent turn. YAML frontmatter is stripped. `$ARGUMENTS` is replaced with everything after the command name; if arguments are provided and `$ARGUMENTS` is absent, imp appends `ARGUMENTS: ...`.

Lua extensions remain the path for executable slash behavior with deterministic host-side effects:

```
.imp/skills/deploy/SKILL.md   # agent instructions and usage notes
.imp/lua/deploy.lua           # optional executable /deploy behavior
```

```lua
imp.register_command("deploy", {
    description = "Deploy the current project",
    handler = function(args)
        return imp.exec("./scripts/deploy " .. (args or "")).stdout
    end
})
```

Lua-registered commands are shown in the TUI slash command menu with their descriptions and can be run as `/deploy ...`.

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

### Status

| Area | Status |
|------|--------|
| Terminal UI and CLI chat | active surface |
| Sessions, branching, compaction | active surface |
| Native tools | active surface |
| Provider auth and secure secrets | active surface |
| Lua extensions | stable shipped extension path |
| Mana task execution | active surface |
| TypeScript/Pi extension compatibility | limited compatibility layer |
| Rust SDK | preview |
| Broader orchestration/RPC boundaries | active development |

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
| `mana` | durable task graph, facts, decisions, dependencies, verification, orchestration state |

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

## License

imp is licensed under the Mozilla Public License 2.0 (MPL-2.0).

You may use imp commercially, embed it in proprietary products, build private tools around it, and use it internally. If you modify imp's MPL-covered source files and distribute those modified files or binaries built from them, those modified imp files must remain available under MPL-2.0. Separate applications, plugins, integrations, and larger works that use imp may remain under their own licenses.

See [LICENSE](LICENSE) for the full license text.

## CLI Reference

Common commands:

```bash
imp                         # open terminal UI
imp chat                    # open CLI chat shell
imp -p "prompt"              # one-shot prompt
imp @file "prompt"           # prompt with file context
imp -c                       # continue most recent session
imp run <unit-id>            # execute mana task directly
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
