# imp

Local terminal coding agent in Rust. imp runs through the TUI, one-shot prompts, or a JSONL RPC protocol. It uses structured tools, durable sessions, and file-backed workflows for planned, inspectable development work.

## Install

```bash
brew tap kfcafe/tap && brew install imp
```

```bash
imp                           # TUI
imp tui                       # TUI, explicit
imp -p "Summarize this repo"   # one-shot prompt
imp --mode rpc                # JSONL RPC protocol
imp -c                        # continue latest session
```

## Features

Runtime features:

- terminal UI
- one-shot prompt mode
- JSONL RPC mode
- provider-flexible model runtime
- structured native tools
- durable JSONL sessions
- branchable conversation history
- context compaction
- verification gates
- trace/evidence artifacts
- OS-backed secret storage
- runtime policy for tools, writes, autonomy, and hooks

Workflow features:

Workflows are local project records for multi-step work. They keep the plan, execution state, verification, prototype evidence, and closeout notes together instead of leaving them only in the conversation.

- workflow artifacts under `.imp/workflows`
- YAML workflow schema
- schema-checked workflow updates
- append-only workflow events
- next-action selection
- acceptance/check tracking
- prototype experiments with evidence and follow-ups
- results and closeout records

Extension features:

- Lua tools
- Lua slash commands
- Lua hooks
- extension capability policy
- preview Rust SDK

## Local data and provider traffic

Local execution:

- agent runtime
- TUI and RPC surfaces
- tool execution
- file reads/writes/edits
- shell commands
- git operations
- workflow files and event logs
- session JSONL records
- Lua hooks/extensions

Provider traffic:

- prompts
- selected context
- tool observations used for a turn
- web-search/read requests when web tools are used

Secret values are stored in the OS credential store. `~/.imp/auth.json` stores metadata.

| Path | Contents |
|---|---|
| `~/.config/imp/config.toml` | user config |
| `<project>/.imp/config.toml` | project config |
| `~/.imp/auth.json` | auth metadata |
| `.imp/workflows/` | workflow YAML, events, results, artifacts |

## Providers

Provider families:

- Anthropic
- OpenAI / ChatGPT
- Google
- OpenAI-compatible APIs
- Moonshot / Kimi
- Z.AI / GLM
- DeepSeek
- Groq
- Cerebras
- xAI
- Mistral
- Together
- OpenRouter
- Fireworks

API-key configuration:

```bash
export ANTHROPIC_API_KEY=...
export OPENAI_API_KEY=...
export GOOGLE_API_KEY=...
export OPENROUTER_API_KEY=...
```

Credential commands:

```bash
imp login
imp login openai
imp login kimi
imp secrets list
imp secrets doctor
```

## Tools

| Tool | Feature |
|---|---|
| `read` | ranged file/image reads |
| `write` | file creation/overwrite |
| `edit` / `multi_edit` | exact and transactional edits |
| `bash` | shell commands with timeout/cancellation |
| `git` | status, diff, log, stage, commit, restore, worktrees |
| `scan` | tree-sitter code search/extraction |
| `web` | web/GitHub search and page reads |
| `ask_user` | structured user prompts |
| `prototype` | disposable experiments with evidence |
| `workflow` | workflow list/show/validate/run/update |
| `memory` | persistent agent memory |

The model uses native tools instead of relying only on shell commands. This gives imp narrower, policy-checkable operations for common development tasks.

Tool execution rules:

- read-only tools can run in parallel
- mutable tools are serialized
- runtime policy checks tool visibility and execution
- write-path policy checks file mutations
- autonomy policy controls unattended action level

## Workflows

Workflows are YAML-backed project artifacts for work that needs more structure than a single prompt. A workflow can describe the plan, the required context, the execution steps, the checks, any prototype experiments, and the evidence needed to close the work.

Workflow root:

```text
.imp/workflows/<id>/
├── workflow.yaml
├── events.jsonl
├── results.md
└── artifacts/
```

Workflow schema fields:

- metadata
- parent workflow reference
- settings
- goal/user value
- non-goals
- acceptance criteria
- context requirements
- steps
- dependencies
- checks
- prototype experiments and their evidence
- workers
- results
- closeout rules

Workflow tool actions:

```text
list
show
validate
run
update
```

Workflow lifecycle:

```text
inspect → validate → run → update → events → prototype/verify → review → closeout
```

Update behavior:

- validates the edited workflow
- writes allowed status/path changes
- appends `events.jsonl`
- rejects invalid status values
- keeps workflow state file-backed and reviewable

Slash commands:

| Command | Feature |
|---|---|
| `/plan` | workflow planning |
| `/status` | workflow/session status |
| `/run` | next workflow action |

Current storage is local and file-backed. API-addressable workflows are planned.

## Sessions

Session records:

- messages
- tool calls
- tool results
- usage metadata
- cost metadata
- branch metadata
- compaction entries
- checkpoint/recovery records

Evidence surfaces:

- `--verify` commands
- trace events
- evidence packets
- final outcomes: `DONE`, `DONE_WITH_CONCERNS`, `BLOCKED`, `NEEDS_CONTEXT`

```bash
imp evidence list
imp evidence latest
```

## TUI controls

| Input | Feature |
|---|---|
| `/` | command palette |
| `@` | file context attachment |
| `Ctrl+L` | model selector |
| `Shift+Tab` | thinking level |
| `/compact` | context compaction |
| `/settings` | settings editor |
| `/secrets` | credential manager |

## RPC mode

`--mode rpc` starts a JSON-lines stdin/stdout protocol for host applications. It is the non-TUI integration surface for embedding imp in another process.

Input command types:

```text
prompt
steer
followup
cancel
```

Output includes agent, tool, stream, runtime event, and runtime state payloads. `--runtime-json` emits the shared runtime event/state shape alongside legacy JSON fields.

```bash
imp --mode rpc --runtime-json
```

## Policy

```bash
IMP_MODE=reviewer imp -p "inspect this diff"
imp -p "inspect this diff" --deny-tool bash --deny-write '**'
imp -p "fix the failing test" --allow-write crates/imp-core --verify "cargo test -p imp-core"
```

Policy controls:

```bash
--allow-tool read --allow-tool git
--deny-tool bash
--allow-write crates/imp-core
--deny-write '**/*.lock'
--autonomy safe
--verify "cargo test"
```

Autonomy modes:

```text
suggest
safe
local-auto
worktree-auto
allow-all-local
allow-all
ci
```

## Configuration

Precedence:

1. built-in defaults
2. `~/.config/imp/config.toml`
3. `<project>/.imp/config.toml`
4. environment variables
5. CLI flags

```toml
model = "sonnet"
thinking = "medium"
max_turns = 100
max_tokens = 2048

[web]
search_provider = "exa"

[ui]
notify_on_agent_complete = true
```

## Extensions

Stable extension runtime: Lua.

Load paths:

```text
~/.config/imp/lua/
<project>/.imp/lua/
```

Extension features:

- tools
- slash commands
- hooks
- capability policy for shell/filesystem/HTTP/secrets/native tools

```lua
imp.register_command("greet", {
    description = "Say hello",
    handler = function(args) return "Hello, " .. (args or "world") end
})
```

## Rust SDK

Preview API: `imp_core::sdk`.

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

## Crates

```text
imp-cli   CLI entrypoint, TUI launch, one-shot/headless/RPC modes
imp-core  agent loop, tools, sessions, workflows, policy, SDK
imp-llm   providers, streaming, auth, model metadata
imp-lua   Lua extension runtime
imp-tui   terminal UI
```

## Status

Active:

- TUI
- one-shot prompts
- JSONL RPC mode
- native tools
- durable sessions
- file-backed workflows
- verification/evidence
- provider auth
- OS-backed secrets
- policy controls
- Lua extensions

Preview/planned:

- Rust SDK preview
- limited TypeScript/Pi extension compatibility
- MCP planned
- `.imp/agents` planned
- ACP/editor adapters planned
- hosted sync/team collaboration planned
- workflow API planned

Compatibility/legacy:

- legacy `mana` integration is optional and compatibility-oriented

## Technical docs

- [Docs index](docs/index.md)
- [Workflows](docs/workflows.md)
- [RPC protocol](docs/rpc.md)
- [Native tools](docs/tools.md)
- [Runtime policy](docs/policy.md)
- [Sessions and evidence](docs/sessions.md)
- [Lua extensions](docs/extensions-lua.md)
- [Architecture](docs/architecture.md)

## Development

```bash
cargo test --workspace --all-targets
cargo bench -p imp-core --bench core_hot_paths
```

```bash
bash tools/run-leaks.sh
bash tools/run-miri.sh
bash tools/run-asan.sh
bash tools/run-tsan.sh
bash tools/run-stress.sh
```

## License

MPL-2.0. See [LICENSE](LICENSE).
