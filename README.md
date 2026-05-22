# imp

**A terminal-native coding agent with durable local work.**

imp is an open-source coding agent for interactive development, one-shot automation, persistent sessions, secure local tools, and structured work that should survive beyond a chat transcript.

If you already use Claude Code, Codex, OpenCode, or Pi, imp is aimed at a different tradeoff: keep the agent in the terminal, keep the runtime inspectable, expose strong native tools directly to the model, and leave local records of work, checks, and evidence. Agents can use `bash`, but imp also gives them narrower tools for reading files, exact edits, git, structural code search, web/GitHub search, user questions, prototypes, and durable work. Those tools are easier to constrain, audit, and recover from than a shell-only workflow.

Core capabilities:

- terminal UI, CLI chat, and one-shot prompt mode
- durable JSONL sessions with branching, compaction, replayable tool output, and usage metadata
- native tools for files, edits, shell, git/worktrees, structural code search, web/GitHub search, memory, and user prompts
- native imp-work for tasks, epics, memory, decisions, context packs, runs, checks, prototypes, and handoff
- verification gates, traces, evidence packets, and structured run outcomes
- provider support for Anthropic, OpenAI/ChatGPT, Google, OpenAI-compatible APIs, and other hosted model providers
- OS-backed secret storage
- runtime modes, autonomy controls, tool allow/deny lists, and write-path constraints
- Lua tools, slash commands, and hooks
- early Rust SDK for embedding imp sessions

```bash
brew tap kfcafe/tap && brew install imp
```

## Why use imp?

imp is not trying to be a hosted autonomous engineer or an AI IDE. It is a local agent workbench for developers who want control over the runtime.

Use imp when you want:

- a terminal-native agent instead of an editor- or SaaS-first workflow
- model/provider flexibility, including BYOK and OpenAI-compatible providers
- durable local sessions and work records instead of disposable chats
- tool execution that can be constrained by mode, allow/deny lists, write paths, autonomy, and hooks
- native tools that are more structured than asking the model to do everything through `bash`
- verification commands and evidence artifacts for serious changes
- hooks, Lua extensions, and an early Rust SDK for customization/embedding

Compared with common alternatives:

| Tool | Typical shape | imp's different tradeoff |
|---|---|---|
| Claude Code | polished proprietary terminal agent | open-source, local work records, broader provider support, hackable runtime |
| Codex CLI | OpenAI-first terminal agent | provider-flexible, durable imp-work, explicit evidence/policy surfaces |
| OpenCode | open-source terminal agent | imp emphasizes native durable work, evidence, policy, and structured tool surfaces |
| Pi | agent/runtime experimentation | imp is the Rust-native terminal product with native tools, sessions, and imp-work |
| Cursor-style editors | AI editor experience | imp stays terminal-first and editor-agnostic |
| Factory/Devin-style platforms | hosted/team agent platform | imp is local-first and inspectable, with hosted sync/team features planned separately |

## What runs locally, and what leaves your machine?

imp runs the agent runtime, tool execution, sessions, work records, hooks, and extensions locally. Model prompts and tool observations needed for a turn are sent to the configured model provider. Web search/read tools call the configured web provider or target URL. Local shell commands and file edits run on your machine.

Secrets are stored through the OS credential store. `~/.imp/auth.json` stores metadata, not secret values. You can also use environment variables for provider keys.

Important local paths:

| Path | Purpose |
|---|---|
| `~/.config/imp/config.toml` | user config |
| `<project>/.imp/config.toml` | project config |
| `~/.imp/auth.json` | auth metadata; secret values live in OS credential storage |
| `~/.imp/work` | global project-scoped imp-work store |
| `.imp/` | optional project-local config/extensions and future project assets |

## Quick start

Authenticate with an API key:

```bash
export ANTHROPIC_API_KEY=sk-ant-...
imp
```

Or use a built-in login flow:

```bash
imp login          # Anthropic OAuth
imp login openai   # OpenAI / ChatGPT OAuth
imp login kimi     # guided Kimi setup
```

Common entrypoints:

```bash
imp                              # fullscreen terminal UI
imp chat                         # lightweight CLI chat shell
imp -p "Summarize this repo"      # one-shot prompt
imp @src/main.rs "Explain this"   # prompt with file context
imp -c                            # continue the most recent session
imp --list-models                 # list available models
```

Useful constrained automation flags:

```bash
imp -p "fix the failing parser test" \
  --autonomy local-auto \
  --verify "cargo test -p imp-core parser" \
  --allow-write crates/imp-core
```

## What imp provides

### Terminal UI and CLI modes

- fullscreen TUI with streaming output and live tool activity
- CLI chat shell via `imp chat`
- one-shot prompt mode via `imp -p`
- file attachment with `@path`
- model and thinking controls
- session resume, branch navigation, and compaction
- settings, personality, and secrets screens

Common TUI controls:

| Input | Action |
|---|---|
| `/` | command palette |
| `@` | file finder / attach context |
| `Ctrl+L` | model selector |
| `Shift+Tab` | cycle thinking level |
| `/compact` | compact older branch history |
| `/settings` | edit UI/runtime settings |
| `/personality` | edit identity and behavior profile |
| `/secrets` | manage provider/service credentials |

### Durable sessions

Sessions are append-only JSONL records containing:

- user and assistant messages
- tool calls and tool results
- usage and cost metadata
- branch metadata
- compaction entries
- checkpoint and recovery records

Long sessions stay usable through compaction, observation masking, branch navigation, and on-disk replay/debug artifacts.

### Native tools

imp exposes a small native tool surface to the agent. Read-only tools can run in parallel; mutable and side-effecting tools are checked by runtime policy.

| Tool | Purpose |
|---|---|
| `read` | read text files and images with range support |
| `write` | create or overwrite files |
| `edit` | exact find/replace edits, including anchored edits |
| `multi_edit` | coordinated transactional edits across one or more files |
| `bash` | shell execution with timeout/cancellation |
| `git` | status, diff, log, stage, commit, restore, worktrees |
| `worktree` | create, list, and remove git worktrees |
| `scan` | tree-sitter structural code extraction/search |
| `web` | web search, page read, GitHub search/read, YouTube metadata/transcripts |
| `ask_user` | structured user questions, including multi-select prompts |
| `work` | native imp-work tasks, memory, context, runs, verification, and handoff |
| `prototype` | bounded disposable code experiments with structured evidence |
| `memory` | persistent memory across sessions |
| `session_search` | search local conversation history |

The legacy `mana` tool and `imp mana` command still exist for migration and compatibility, but new workflow work is moving to native imp-work.

### imp-work: durable local work

imp-work is imp's native durable work system. It is used for work that needs state, context, verification, or handoff beyond the current conversation.

imp-work includes:

- tasks, epics, subtasks, and dependencies
- durable memory, decisions, and follow-ups
- context packs for prepared worker/prototype runs
- runs, attempts, leases, and path locks
- verification checks and structured outcomes
- prototype observations and promoted learnings
- project stream history for continuity across follow-up work

Normal imp-work storage is global and project-scoped under `~/.imp/work`, keyed by canonical project root. Project-local `.imp/work` stores are migration input only.

From chat, you can ask naturally:

```text
create a task for the failing auth edge case
show me the next ready task
work on the next task and verify it
record that we decided to keep provider config local-first
run a prototype to check whether this parser approach works
```

### Workflow evidence and verification

imp can produce local run artifacts for review and handoff:

- verification gates from `--verify`
- trace events for agent lifecycle, tool execution, policy decisions, and checks
- evidence packets summarizing actions, artifacts, verification, and remaining concerns
- final outcomes such as `DONE`, `DONE_WITH_CONCERNS`, `BLOCKED`, and `NEEDS_CONTEXT`

Inspect evidence from the CLI:

```bash
imp evidence list
imp evidence latest
```

### Providers, auth, and secrets

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
imp login kimi          # guided Kimi setup

imp secrets moonshot    # store an API key securely
imp secrets list        # list configured providers/services
imp secrets show exa    # show metadata, not secret values
imp secrets doctor      # verify secure-storage references
```

Supported provider families include Anthropic, OpenAI/ChatGPT/Codex, Google, Moonshot/Kimi, Z.AI/GLM, DeepSeek, Groq, Cerebras, xAI, Mistral, Together, OpenRouter, Fireworks, and compatible APIs.

Secrets are stored in the OS credential store. `~/.imp/auth.json` stores metadata only.

| Platform | Store |
|---|---|
| macOS | Keychain |
| Linux | Secret Service |
| Windows | native credential store |

### Import from other agents

imp can import skills and configuration from supported local agent setups:

```bash
imp import --from claude --dry-run
imp import --from codex --dry-run
imp import --from pi --dry-run
```

### Web and GitHub search

The `web` tool supports Tavily, Exa, Linkup, Perplexity, and GitHub search/read.

```bash
export TAVILY_API_KEY=tvly-...
export EXA_API_KEY=exa-...
export IMP_WEB_PROVIDER=exa

imp web-login tavily
imp web-login exa
imp secrets exa
```

YouTube reading supports public metadata and captions/transcripts for watch, shorts, embed, and `youtu.be` URLs. It does not require `yt-dlp`, media download, or a web-search API key. Transcript extraction is best effort.

## Safety model

imp is a local coding agent with tools that can read files, edit files, run shell commands, access git, call web providers, and use extensions. Treat it like a powerful local development tool.

Controls available today:

- modes restrict which tools are shown to the model and still block disallowed tools at execution time
- `--allow-tool` / `--deny-tool` constrain tools for a run
- `--allow-write` / `--deny-write` constrain write paths for a run
- `--autonomy` sets how much imp may do without stopping for approval
- `--verify` defines commands required for closeout in automation workflows
- hooks can inspect, modify, or block tool behavior
- Lua extension capability policy controls access to shell, filesystem, HTTP, secrets, and native tools
- secrets are stored outside normal config files

Examples:

```bash
imp -p "inspect this diff" --deny-tool bash --deny-write '**'
imp -p "fix the failing test" --allow-write crates/imp-core --verify "cargo test -p imp-core"
IMP_MODE=reviewer imp chat
```

## Modes, autonomy, and policy

Modes and run policy control which tools are visible to the model and which actions are allowed at execution time.

| Mode | Purpose |
|---|---|
| `full` | normal interactive use |
| `worker` | execute scoped implementation work |
| `orchestrator` | plan/decompose and coordinate work |
| `planner` | read, ask, and create structured work |
| `reviewer` | read-only code/design review |
| `auditor` | read-only inspection with durable-work visibility |

```bash
IMP_MODE=reviewer imp chat
imp -p "inspect this diff" --deny-tool bash --deny-write '**'
```

Useful run constraints:

```bash
--allow-tool read --allow-tool git
--deny-tool bash
--allow-write crates/imp-core
--deny-write '**/*.lock'
--autonomy safe
--verify "cargo test"
```

Autonomy modes include `suggest`, `safe`, `local-auto`, `worktree-auto`, `allow-all-local`, `allow-all`, and `ci`.

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

## Extensibility

Lua is the current stable extension path. Lua extensions can register tools, slash commands, and hooks.

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
| `imp-core` | agent loop, tools, sessions, context, hooks, imp-work, policy, evidence |
| `imp-llm` | providers, streaming parsers, model metadata, auth |
| `imp-lua` | Lua extension loading, sandboxing, bridge APIs |

## Project status and limitations

imp is active software, not a finished hosted product.

Works today:

- TUI, CLI chat, one-shot prompts, session resume, and compaction
- native file/edit/bash/git/scan/web/work/prototype tools
- native imp-work for durable tasks, memory, context, runs, checks, and outcomes
- provider auth and OS-backed secret storage
- runtime modes, autonomy, tool constraints, hooks, and Lua extensions
- local evidence artifacts and verification gates

Important limitations:

- MCP support is planned, not shipped
- `.imp/agents` custom agent files are planned, not shipped
- ACP/editor adapters are planned, not shipped
- hosted sync/team collaboration is planned, not shipped
- TypeScript/Pi extension compatibility is limited; Lua is the stable extension path
- the Rust SDK is preview-level
- legacy mana commands/tools remain for compatibility and migration while new durable work uses imp-work

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
| Native imp-work | active surface |
| Workflow evidence and verification gates | active surface |
| Lua extensions | stable shipped extension path |
| Legacy mana command/tool | compatibility and migration path |
| TypeScript/Pi extension compatibility | limited compatibility layer |
| Rust SDK | preview |
| MCP, `.imp/agents`, ACP, hosted sync | planned / not shipped |

## License

imp is licensed under the Mozilla Public License 2.0 (MPL-2.0).

MPL-2.0 is intentional for imp. It is a file-level copyleft license: changes to imp's MPL-covered source files must stay available under MPL-2.0 when distributed, but separate applications, integrations, plugins, extensions, and larger works can remain under their own licenses. That fits imp's goals: the core agent/runtime stays open, while commercial use, embedding, private tools, and proprietary integrations remain allowed.

You may use imp commercially, embed it in proprietary products, build private tools around it, and use it internally. If you modify imp's MPL-covered source files and distribute those modified files or binaries built from them, those modified imp files must remain available under MPL-2.0.

See [LICENSE](LICENSE) for the full license text.
