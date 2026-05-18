# imp

**A terminal-native coding agent with durable work built in.**

imp is an extensible agent runtime for real software work: interactive coding, one-shot automation, long-running task execution, secure tools, persistent sessions, and a local mana work graph for work that should survive beyond a chat transcript.

The `0.2.0` line adds the workflow runtime foundations: structured run artifacts, trace/evidence emission, verification gates, policy/trust events around tool execution, and a clearer path from prompt -> work -> proof.

```bash
brew tap kfcafe/tap && brew install imp
```

## Why imp

Most coding agents are either a chat UI with tools bolted on, or an orchestration layer that forgets the interactive loop. imp is both:

- **Terminal-first agent UI** - fast TUI, CLI chat, and one-shot prompt mode.
- **Durable sessions** - JSONL history, branch navigation, compaction, usage records, and replayable tool output.
- **Native tool surface** - read/write/edit, shell, git, structural scan, web, memory, and mana tools exposed directly to the model.
- **Mana work graph** - tasks, dependencies, notes, decisions, facts, verification gates, workers, and run state.
- **Policy-aware execution** - modes restrict tools before the model sees them and again at execution time.
- **Workflow evidence** - runs can emit traces, evidence packets, verification status, and trust/provenance metadata.
- **Extensible by default** - Lua tools/commands/hooks today; TypeScript compatibility where implemented.

## Install

### macOS

```bash
brew tap kfcafe/tap && brew install imp
```

### Linux archives

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

### From source

```bash
git clone https://github.com/kfcafe/imp.git
cd imp
uu install --default
```

If a locally installed macOS binary is killed immediately after install:

```bash
bash tools/imp-fix-signature.sh ~/.local/imp-current/bin/imp
```

## Quick start

Authenticate with an API key:

```bash
export ANTHROPIC_API_KEY=sk-ant-...
imp
```

Or use built-in login flows:

```bash
imp login          # Anthropic OAuth
imp login openai   # OpenAI / ChatGPT OAuth
imp login kimi     # guided Kimi setup
```

Then use the shape that fits the job:

```bash
imp                              # full terminal UI
imp chat                         # lightweight CLI chat shell
imp -p "Summarize this repo"      # one-shot prompt
imp @src/main.rs "Explain this"   # prompt with file context
imp -c                            # continue recent session
imp run 12.1                      # execute a mana task directly
```

## What ships

### Interactive coding

The default `imp` command opens the terminal UI:

- streaming assistant output and live tool activity
- prompt editor with slash-command palette
- file attachment with `@`
- model and thinking controls
- session tree and branch navigation
- sidebar inspection for tool calls and outputs
- settings, personality, and secrets screens

Common controls:

| Input | Action |
|---|---|
| `/` | command palette |
| `@` | file finder / attach context |
| `Ctrl+L` | model selector |
| `Shift+Tab` | cycle thinking level |
| `/compact` | compact older branch history |
| `/settings` | edit UI/runtime settings |
| `/personality` | edit identity and behavior profile |

### One-shot and shell modes

```bash
imp -p "review the latest diff"
imp chat
```

`imp chat` keeps a persistent CLI session and supports shell-style commands such as `:help` plus `@file` attachments.

### Durable sessions

Sessions are append-only JSONL records containing:

- user and assistant messages
- tool calls and tool results
- usage records and cost metadata
- branch metadata
- compaction entries
- checkpoint and recovery records

Long sessions stay usable through `/compact`, observation masking, branch navigation, and on-disk replay/debug artifacts.

### Native tools

imp exposes a focused native tool surface to the agent. Read-only tools can run in parallel; mutable and side-effecting tools are policy checked.

| Tool | Purpose |
|---|---|
| `read` | read text files and images with range support |
| `write` | create or overwrite files |
| `edit` | exact find/replace edits |
| `multi_edit` | coordinated transactional edits |
| `bash` | shell execution with timeout/cancellation |
| `git` | status, diff, log, stage, commit, restore, worktrees |
| `scan` | tree-sitter structural code extraction/search |
| `web` | web search, page read, YouTube metadata/caption extraction |
| `ask` | structured user questions |
| `mana` | inspect/update/create/close/claim/run mana units |
| `memory` | persistent memory across sessions |
| `session_search` | search local conversation history |

### Mana: work that survives the chat

Mana is included task coordination for longer-running agent work. Think of it as a local work graph for agents: what needs to happen, why it matters, what depends on it, what was tried, and what proves it is done.

Mana records can include:

- epics and tasks
- acceptance criteria
- verify commands
- dependencies
- notes and attempts
- decisions and facts
- worker/run state

Most users can interact with mana naturally from inside imp:

```text
work on 12.1
show me the next task and work on it
create a task for the failing auth edge case
```

Direct execution is also available:

```bash
imp run 12.1
```

A good mana task has a clear proof:

```text
Title: Add validation for empty API tokens
Acceptance: Empty or whitespace-only tokens are rejected with a user-facing error.
Verify: cargo test -p imp-llm auth::token_validation
```

### Workflow runtime and evidence

The `0.2.0` workflow runtime makes agent work easier to audit and resume:

- workflow contracts derived from prompt, cwd, mana task, autonomy mode, and verification requirements
- verification gates that can block clean closeout when required checks fail
- trace events for agent lifecycle, tool execution, policy decisions, and verification
- evidence packets summarizing actions, trust/provenance, and artifacts
- run evidence HTML/JSONL artifacts for local review
- final status outcomes such as `DONE`, `DONE_WITH_CONCERNS`, `BLOCKED`, and `NEEDS_CONTEXT`

### Providers and auth

imp includes native Anthropic, OpenAI, and Google integrations, plus OpenAI-compatible providers.

```bash
export ANTHROPIC_API_KEY=...
export OPENAI_API_KEY=...
export GOOGLE_API_KEY=...
export OPENROUTER_API_KEY=...
```

Useful auth commands:

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

Secrets are stored in the OS credential store. `~/.imp/auth.json` stores metadata only.

| Platform | Store |
|---|---|
| macOS | Keychain |
| Linux | Secret Service |
| Windows | native credential store |

### Web and YouTube

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

## Modes and policy

Modes control tool visibility and execution policy.

| Mode | Purpose |
|---|---|
| `full` | normal interactive use |
| `worker` | execute a scoped task |
| `orchestrator` | plan/decompose and coordinate workers |
| `planner` | read, ask, and create structured work |
| `reviewer` | read-only code/design review |
| `auditor` | read-only inspection with mana visibility |

```bash
IMP_MODE=reviewer imp chat
IMP_MODE=worker imp run 5.1
```

Disallowed tools are omitted from the model prompt and still blocked at execution time.

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

TUI settings surfaces:

- `/settings` - display and runtime preferences
- `/personality` - identity, behavior sliders, global/project scope, profiles
- `/secrets` - provider/service credential setup

## Extensibility

### Lua extensions

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

### Slash-command skills

A skill at `.imp/skills/deploy/SKILL.md` or `~/.config/imp/skills/deploy/SKILL.md` creates `/deploy` unless a built-in or Lua command already uses that name. Use `/skill:deploy` to explicitly invoke the skill when a name is ambiguous.

Invoking a skill inserts its `SKILL.md` instructions into the next agent turn. YAML frontmatter is stripped. `$ARGUMENTS` is replaced with everything after the command name; if arguments are provided and `$ARGUMENTS` is absent, imp appends `ARGUMENTS: ...`.

### TypeScript compatibility

TypeScript extension support is a limited compatibility and forward-direction layer, not full Pi API parity.

```bash
imp import --from pi
```

Current support includes Bun-backed `.ts` entrypoints, Pi-style `registerTool(...)`, common TypeBox-style schemas, text/details tool results, and limited `session_start` lifecycle hooks for dynamic tool registration.

## Programmatic usage

`imp-core` exposes an early Rust SDK through `imp_core::sdk`.

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

| Layer | Responsibility |
|---|---|
| `imp-cli` | command parsing, setup/login, chat/headless/RPC entrypoints |
| `imp-tui` | terminal UI, editor, views, rendering, interaction state |
| `imp-core` | agent loop, tools, session persistence, context, hooks, mana integration |
| `imp-llm` | providers, streaming parsers, model metadata, auth |
| `imp-lua` | Lua extension loading, sandboxing, bridge APIs |
| `mana` | durable task graph, facts, decisions, dependencies, verification, orchestration state |

## Development

```bash
cargo test --workspace --all-targets
cargo bench -p imp-core --bench core_hot_paths
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

## Status

| Area | Status |
|---|---|
| Terminal UI and CLI chat | active surface |
| Sessions, branching, compaction | active surface |
| Native tools | active surface |
| Provider auth and secure secrets | active surface |
| Lua extensions | stable shipped extension path |
| Mana task execution | active surface |
| Workflow evidence and verification gates | active surface in `0.2.0` |
| TypeScript/Pi extension compatibility | limited compatibility layer |
| Rust SDK | preview |
| Broader orchestration/RPC boundaries | active development |

## License

imp is licensed under the Mozilla Public License 2.0 (MPL-2.0).

You may use imp commercially, embed it in proprietary products, build private tools around it, and use it internally. If you modify imp's MPL-covered source files and distribute those modified files or binaries built from them, those modified imp files must remain available under MPL-2.0. Separate applications, plugins, integrations, and larger works that use imp may remain under their own licenses.

See [LICENSE](LICENSE) for the full license text.
