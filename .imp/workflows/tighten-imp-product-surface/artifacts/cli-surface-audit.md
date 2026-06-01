# CLI Surface Audit

Status: focused CLI audit for `tighten-imp-product-surface`.

## Target CLI boundary

Keep launch CLI narrow:

```sh
imp                  # TUI
imp -p "..."         # one-shot prompt
imp workflow run <id> # explicit workflow execution
```

Optional/advanced if low-cost:

```sh
imp --mode rpc
```

Remove/archive:

```sh
imp chat
```

Do not add `/work` or broad workflow CLI subcommands in this workflow.

## Evidence inspected

Files/docs:

- `crates/imp-cli/src/lib.rs`
- `README.md`
- `docs/rpc.md`
- `docs/index.md`
- `docs/architecture.md`
- `docs/rebuild/imp-cli-interactive-shell.md`
- `docs/workflow-first-ux.md`
- `docs/imp-next-workflow-runtime.md`
- `docs/design/oss-launch-checklist.md`
- `docs/design/imp-native-workflow-engine.md`

Key code evidence:

- `Cli` includes `print`, `mode`, provider/model/thinking/auth args.
- `Commands::Chat` exists.
- `Commands::Personality` exists.
- `Commands::Eval` exists.
- `run_print_mode()` exists.
- `run_rpc_mode()` exists.
- `RpcInputCommand` supports prompt/cancel/steer/followup.
- `ChatShellCommand` supports help/status/new/resume/compact/settings/personality/setup/view/model/thinking/quit.
- `parse_chat_shell_command()` supports `:` and slash compatibility.
- Chat shell help explicitly says slash-prefixed forms still work during migration.

## Current CLI command surface

From `enum Commands` and nearby code, current CLI includes or references:

- chat
- tui
- view
- settings
- personality
- setup
- login
- secrets
- mana behind feature
- stats
- usage
- evidence
- eval
- import
- install-local
- web-login

Mode flag includes:

- interactive
- chat
- rpc
- json

## Classification matrix

| Surface | Classification | Evidence | Rationale | Next action |
|---|---:|---|---|---|
| default `imp` TUI | keep | CLI default/TUI command path | Primary product. | Preserve. |
| `imp -p` one-shot | keep | `run_print_mode()` | Primary non-interactive path. | Preserve. |
| piped stdin print mode | keep/improve | stdin piped without `-p` runs print mode | Useful Unix behavior. | Keep if simple. |
| `imp workflow run <id>` | improve/add | absent human CLI; model workflow tool has run action | Needed for explicit automation/subagents. | Add minimal command, no flags. |
| `--mode rpc` / `--mode json` | internal/keep if low-cost | `run_rpc_mode`, RPC parser/tests/docs | Nice-to-have for embedding. | Keep advanced/internal; avoid expanding. |
| `imp chat` | remove/archive | `Commands::Chat`, `run_chat_shell`, `ChatShellCommand` | Second grammar; user does not want to cater to CLI chat. | Remove after one-shot/TUI safe. |
| `--mode chat` | remove | mode path branches to chat shell | Same as CLI chat. | Remove with chat shell. |
| `Commands::Personality` | remove | personality CLI builder/source editor | Personality backend targeted for removal. | Remove with personality. |
| `Commands::Eval` | internal/fold/remove | CLI eval command and docs | Not launch surface; workflow evidence can replace. | Decide internal vs fold/remove. |
| `Commands::View` | decide/internal | `imp view` used for sessions/tree/logs/checkpoints | Could be useful diagnostics; not core launch. | Keep internal if low-cost, or fold into TUI. |
| settings/setup/login/secrets CLI | keep/decide | CLI commands exist | Useful auth/config fallback outside TUI. | Keep unless TUI fully covers setup. |
| stats/usage/evidence | internal/decide | CLI commands/reports | Diagnostics/dev utility. | Keep internal or hide docs. |
| import/install-local/web-login | internal/decide | CLI commands | Useful project/install utilities but not core product thesis. | Keep if low-cost; avoid launch emphasis. |
| mana CLI feature | remove/archive | `mana-ui` feature namespace | Old durable primitive. | Remove from product docs; isolate feature. |

## CLI chat removal details

### Code to remove/archive

Primary code:

- `Commands::Chat`
- `ChatShellCommand`
- `parse_chat_shell_command()`
- `handle_chat_shell_command()`
- `run_chat_shell()`
- chat shell tests around `parse_chat_shell_command_*`
- `ShellLiveness` only if used solely by chat/print liveness; inspect before deletion
- `--mode chat` branch

Help text to remove:

```text
Shell commands
  :help
  :status
  :new
  :resume
  :compact (planned)
  :settings
  :personality
  :setup
  :view
  :model
  :thinking
  :quit

Compatibility
  Slash-prefixed forms like `/help` and `/view` still work during migration,
  but `:` is the preferred shell grammar.
```

Docs to archive/update:

- `docs/rebuild/imp-cli-interactive-shell.md`
- `docs/workflow-first-ux.md` sections using `imp chat`
- `docs/imp-next-workflow-runtime.md` sections using `imp chat`
- `docs/design/oss-launch-checklist.md` chat launch checks
- README references if any

### Risk

Medium/high because `imp-cli/src/lib.rs` is 6.5k lines and chat helpers may share setup/model/session utilities with one-shot/TUI/RPC.

Mitigation:

- remove in a focused pass after `imp workflow run <id>` plan is accepted.
- preserve shared helpers for one-shot/RPC.
- run `cargo test -p imp-cli --lib`.

## `imp workflow run <id>` plan

### Initial CLI shape

```sh
imp workflow run <id>
```

No flags initially.

### Code options

Option A — quickest/thin wrapper:

- add `Commands::Workflow(WorkflowCli)` in `crates/imp-cli/src/lib.rs`.
- add `WorkflowCommand::Run { id: String }`.
- call existing workflow tool run/next-action logic or shared helper if already extractable.
- print next action/result clearly.

Option B — better architecture:

- extract workflow run selection from `crates/imp-core/src/tools/workflow.rs` into `crates/imp-core/src/workflow/runner.rs`.
- CLI and model tool both call runner service.

Recommendation:

- If extraction is small, do Option B first.
- If extraction grows, do Option A with honest output and schedule extraction next.

### Expected first output

```text
Workflow: tighten-imp-product-surface
Step: backend_inventory
Action: next
Next: write artifacts/backend-inventory.md
```

Long-term output once execution exists:

```text
Workflow: tighten-imp-product-surface
Step: backend_inventory
Result: done
Verification: workflow validate passed
Next: decide_target_surface
```

### Tests

Add CLI parse tests:

- parses `workflow run tighten-imp-product-surface`
- rejects missing id
- does not require flags

Add behavior tests if runner can be isolated without LLM/provider.

## RPC details

RPC mode supports JSONL commands:

- prompt
- cancel
- steer
- followup

It converts `AgentEvent` to legacy JSON plus runtime event/state when `--runtime-json` is enabled.

Recommendation:

- keep RPC if low-cost.
- document as advanced/internal, not core CLI launch.
- avoid exposing workflow controller snapshots as stable public API.
- ACP should build later on runtime events/state, not on CLI chat.

## Personality CLI removal details

Current personality CLI has:

- `Commands::Personality`
- `PersonalityScopeCli`
- `PersonalityModeCli`
- builder/source render functions
- save/load of global/project soul content

Target replacement:

- no personality CLI.
- generic prompt appendices:
  - `~/.imp/prompt.md`
  - `.imp/prompt.md`

Docs/help should say users can edit those files directly.

## Eval CLI decision

Current CLI eval command may be useful internal QA, but it should not be a product surface.

Options:

1. remove CLI eval entirely after workflow evidence exists.
2. keep as hidden/internal dev command not documented for launch.
3. fold into `workflow` evidence/check artifact handling.

Recommendation:

- remove TUI `/eval` first.
- keep CLI eval internal until workflow evidence replacement is clear.

## `imp view` decision

`imp view` appears useful for sessions/tree/logs/checkpoints. It may overlap with TUI `/resume`/`/tree` and docs.

Recommendation:

- keep internal/advanced initially if low-cost.
- remove checkpoint restore/product claims until real restore exists.
- do not emphasize `imp view` in launch README unless needed.

## Docs cleanup targets

Remove/archive or rewrite mentions of:

- `imp chat`
- `:commands`
- slash compatibility in CLI shell
- `:personality`
- `:compact` planned
- `imp workflow status/evidence/blockers/resume/decisions` broad future commands
- ACP as launch-facing surface

Keep/update mentions of:

- `imp`
- `imp -p`
- `imp workflow run <id>` once implemented
- `imp --mode rpc` under advanced/internal if retained

## Verification after CLI cuts

Minimum:

```sh
cargo test -p imp-cli --lib
cargo check -p imp-cli
cargo check -p imp-core -p imp-tui
```

If RPC touched:

```sh
cargo test -p imp-cli parse_rpc --lib
```

If prompt/one-shot touched:

```sh
cargo test -p imp-core system_prompt --lib
```

## Open decisions

1. Should `imp view` remain as internal/advanced CLI?
2. Should `imp eval` remain hidden/internal until workflow evidence replacement exists?
3. Should setup/login/secrets/settings remain as CLI commands, or should TUI be the only guided setup path?
4. Should RPC be documented in README or only `docs/rpc.md`?
