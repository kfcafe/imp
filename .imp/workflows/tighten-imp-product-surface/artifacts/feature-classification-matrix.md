# Feature Classification Matrix

Status: evidence-backed classification matrix for `tighten-imp-product-surface`.

Legend:

- **keep** — core to target product.
- **remove** — remove from active product/code path after approval.
- **archive** — move docs/experiments to `~/imp-archive` or `docs/archive`.
- **improve** — keep but refactor/tighten.
- **fold** — preserve valuable behavior inside workflows or another retained primitive.
- **internal** — can remain as non-product infrastructure/dev surface.
- **decide** — needs user decision.

## Core product surfaces

| Surface | Classification | Evidence | Rationale | Next action |
|---|---:|---|---|---|
| TUI | keep/improve | `crates/imp-tui/src/app.rs`, `crates/imp-tui/src/views/*` | Primary interactive surface. Too large, but central. | Tighten commands first, then extract command/app modules. |
| One-shot prompt | keep | `crates/imp-cli/src/lib.rs` `run_print_mode` | Primary non-interactive path. | Preserve while cutting CLI chat. |
| Workflows | keep/improve | `crates/imp-core/src/workflow/*`, `crates/imp-core/src/tools/workflow.rs`, `.imp/workflows/*` | Durable orchestration primitive. | Make workflow-native; add `imp workflow run <id>`. |
| Canonical tools | keep | `crates/imp-core/src/builder.rs` `register_native_tools` registers ask/bash/edit/git/read/write/scan/web/workflow | Default tool surface is tight. | Keep. Avoid adding more defaults. |
| Skills | keep/improve | TUI `try_skill_command`, system prompt skill index, `~/.imp/skills` | Minimal extensibility story; user wants skills in prompt. | Keep names/descriptions in prompt. Avoid command palette junk drawer. |
| Sessions | keep/improve | `crates/imp-core/src/session.rs`, TUI `/new`, `/resume`, `/name`, `/tree` | Session continuity is core TUI value. | Keep; decide `/fork`, `/copy`. |
| Auth/config/providers | keep | TUI/CLI setup/login/secrets/settings, provider config | Required for usable product. | Keep; simplify surfaces later. |
| Lua extensions | keep | `crates/imp-lua`, Lua command/tool loading, `/reload` | Shipped extension support. | Keep `/reload`; docs should say Lua is current shipped extension runtime. |

## Nice-to-have / internal launch-adjacent

| Surface | Classification | Evidence | Rationale | Next action |
|---|---:|---|---|---|
| RPC | internal/keep if low-cost | `crates/imp-cli/src/lib.rs` `RpcInputCommand`, `run_rpc_mode`, workflow snapshot events | Useful for embedding/automation; not central product. | Keep if low-cost; document as advanced/internal. |
| ACP/editor adapters | archive/future | README/docs planned references | Not current implemented product. | Move to roadmap/future docs only. |
| Runtime events/state | keep/internal/improve | `RuntimeEvent`, RPC JSONL conversion, TUI event handling | Supports TUI/RPC/future workflow runner. | Keep; remove workflow-controller-specific events if controller defuncted. |
| Stats/usage/evidence CLI | internal/decide | `crates/imp-cli/src/lib.rs` commands | Useful diagnostics but not product identity. | Keep internal or hide from launch docs. |

## TUI command surface

| Command/surface | Classification | Evidence | Rationale | Next action |
|---|---:|---|---|---|
| `/new` | keep | `builtin_commands`, `execute_command` | Core session control. | Retain. |
| `/resume` | keep | `builtin_commands`, CLI chat has resume too | User explicitly says important. | Retain. |
| `/model` | keep | `builtin_commands`, TUI/CLI model paths | Core runtime control. | Retain. |
| `/compact` | keep | `builtin_commands`, TUI compaction | Core context control. | Retain. |
| `/quit` | keep | `builtin_commands` | Basic UI control. | Retain. |
| `/loop` | keep | `builtin_commands`, loop status labels | User wants loop. | Retain. |
| `/stop` | keep | `builtin_commands` | Necessary runtime control. | Retain. |
| `/reload` | keep | `builtin_commands`, Lua/config reload | Supports extensibility. | Retain. |
| `/setup` | keep | `builtin_commands`, CLI setup | Product setup. | Retain. |
| `/secrets` | keep | `builtin_commands`, secrets UI | Auth/config. | Retain. |
| `/login` | keep | `builtin_commands`, OAuth/provider login | Auth. | Retain. |
| `/name` | keep | `builtin_commands` | Useful session labeling. | Retain. |
| `/tree` | keep | `builtin_commands`, session tree | Core session continuity. | Retain. |
| `/settings` | keep | `builtin_commands`, settings view | Config. | Retain. |
| `/fork` | decide | `builtin_commands`, session fork commands | Could be core if session branching matters. | Ask/decide before cut. |
| `/copy` | decide | `builtin_commands` | Useful affordance but not target-critical. | Ask/decide. |
| `/status` | decide/remove | `builtin_commands` | State should likely be visible in TUI, not a command. | Prefer remove if TUI state covers it. |
| `/plan`, `/run`, `/debug`, `/review`, `/verify` | fold/remove | `try_workflow_command`, workflow profile registry | Task shortcuts feel bloated; workflows/natural language should absorb. | Stop surfacing profile commands. |
| `/workflow`, `/workflows` | decide/future | TUI `workflow_command` | Future aliases to same workflow control; no `/work` now. | Do not add `/work`; maybe keep internal workflow inspect until CLI runner exists. |
| `/improve*` | fold/remove | TUI improve constants/commands, worktree/changelog paths | Parallel product mode. Worktree safety belongs in workflow runner policy. | Remove commands; fold useful sandboxing later. |
| `/eval` | fold/remove/internal | TUI save eval candidate; CLI eval command | Product bloat; useful failure capture belongs in workflow evidence/internal dev. | Remove TUI command; decide CLI internal. |
| `/mana`, `/scope` | fold/remove | TUI command palette/help; mana active scope | Old durable primitive. | Remove visible commands; isolate compatibility. |
| `/memory` | remove/decide | TUI `handle_memory_command`, memory stores | Product bloat; prompt appendices can replace ordinary context. | Remove command; decide whether any memory backend remains. |
| `/checkpoints`, `/restore-checkpoint` | remove/decide | `restore-checkpoint` only inspects/not wired | Incomplete visible feature. | Remove from product until real restore exists. |
| `/hotkeys` | remove/decide | TUI help command | Help/docs affordance, not essential slash command. | Remove or fold into help/settings. |
| `/export` | remove/decide | TUI `export_conversation` | Useful but non-core. | Decide; likely remove from default palette. |
| `/session` | remove | Help says defunct in prior audit | Defunct compatibility surface. | Remove. |
| `/personality` | remove | `builtin_commands`, TUI view/backend | Personality backend targeted for removal. | Remove command/backend. |

## CLI surface

| Surface | Classification | Evidence | Rationale | Next action |
|---|---:|---|---|---|
| `imp` TUI | keep | CLI default/TUI command | Primary product. | Keep. |
| `imp -p` one-shot | keep | `run_print_mode` | Primary non-interactive path. | Keep. |
| `imp workflow run <id>` | improve/add | no current human CLI namespace; model tool has WorkflowAction::Run | Needed for explicit workflow execution/cron/subagents. | Add minimal command, no flags. |
| `imp chat` / chat shell | remove | `Commands::Chat`, `ChatShellCommand`, `parse_chat_shell_command`, `run_chat_shell`, tests | Second grammar and parity burden. | Remove/archive. |
| CLI personality | remove | `Commands::Personality`, chat command personality, `run_personality_mode` | Personality product removal. | Remove with backend. |
| CLI eval | internal/fold/remove | `Commands::Eval`, `EvalCommand` | Could be internal dev, but not launch surface. | Decide: hide/internal or fold into workflows. |
| CLI RPC | internal/keep if low-cost | `RpcInputCommand`, JSONL protocol | Nice-to-have launch-adjacent. | Keep advanced/internal. |
| CLI setup/login/secrets/settings | keep | commands in `imp-cli/src/lib.rs` | Auth/config support. | Keep. |
| CLI stats/usage/evidence/import/install-local | internal/decide | commands in `imp-cli/src/lib.rs` | Utility/internal; not core product identity. | Keep hidden/internal or archive docs. |

## Prompt/runtime prompt surfaces

| Surface | Classification | Evidence | Rationale | Next action |
|---|---:|---|---|---|
| one-line identity | improve/add | `system_prompt.rs` currently large identity/operating rules | User wants minimal prompt. | Implement prompt config with one-line default. |
| tools in prompt | keep | tool registry descriptions | Needed for model/tool use. | Keep simple. |
| skills in prompt | keep | user wants skills included | Core extensibility. | Keep names/descriptions. |
| project instructions | keep | AGENTS/resources layer | Strong coding-agent behavior. | Keep. |
| prompt appendices | improve/add | replace soul/personality | Simple customization without backend bloat. | Add `~/.imp/prompt.md`, `.imp/prompt.md`. |
| personality/soul layer | remove | `personality.rs`, `resources::discover_project_soul`, TUI/CLI editors | Product/backend bloat. | Remove after appendix replacement. |
| memory/user profile injection | remove/decide | `memory.rs`, `tools/memory.rs`, system prompt layers | Hidden prompt bloat. | Remove default; maybe optional file/context later. |
| project facts/mana status | remove | prompt audit/mana context modules | Mana-era bloat. | Remove from default prompt. |
| guardrails/mode/workflow doctrine | remove/fold | `system_prompt.rs`, `AgentMode::instructions`, workflow controller prompts | Too much global doctrine. | Scope to explicit task/workflow runner if needed. |

## Workflow/runtime orchestration

| Surface | Classification | Evidence | Rationale | Next action |
|---|---:|---|---|---|
| Workflow tool/schema | keep/improve | `tools/workflow.rs`, `workflow/schema.rs` | Durable primitive. | Keep and simplify around artifacts/checks/results. |
| Workflow controller ambient | fold/remove | `workflow/controller.rs`, `agent/run_loop.rs`, `mana_compat.rs` | Useful strictness, wrong ambient/mana shape. | Defunct for normal TUI; move value into runner. |
| `imp workflow run <id>` runner | improve/add | `cli-workflow-surface.md`, current model tool run | Needed for automation/subagents. | Add thin CLI then evolve runner. |
| Subagents | improve/future | worker assignment data in workflow tool | Desired long-run automation. | Keep as workflow-runner future, not ambient chat. |
| Run artifacts `.imp/runs` | improve | `.imp` ~413M, 818+ historical run dirs from prior audit | Storage bloat. | Add retention/global storage later. |

## Legacy/experimental backend concepts

| Feature | Classification | Evidence | Rationale | Next action |
|---|---:|---|---|---|
| mana/imp-work | fold/remove/archive | `tools/mana.rs` 5.5k lines, `mana_*`, TUI `/mana`/`/scope` | Old durable primitive; workflows replace. | Remove visible surface, isolate backend, archive docs. |
| improve mode | fold/remove | TUI improve worktree/changelog/merge paths | Useful sandboxing, wrong product layer. | Fold worktree policy into workflow runner later. |
| eval candidates | fold/internal/remove | `eval_candidate.rs`, CLI/TUI eval | Useful QA data, wrong product layer. | Fold into workflow evidence or keep internal. |
| prototype tool | fold/remove | `tools/prototype.rs`, workflow prototype fields | Experiments are workflow artifacts/evidence. | Remove standalone tool after schema review. |
| memory | remove/decide | `memory.rs`, `tools/memory.rs`, TUI `/memory` | Special memory product likely unnecessary. | Remove command; decide backend. |
| imp-gui | remove/archive | workspace/default member, `crates/imp-gui` | Not launch target. | Remove from default members first. |

## Docs/artifacts

| Area | Classification | Evidence | Rationale | Next action |
|---|---:|---|---|---|
| README | improve | advertises planned/legacy/prototype concepts | Launch docs should be tight. | Rewrite after surface decisions. |
| root prototypes/docs | archive | root html/md review/rebuild artifacts | Repo clutter. | Move to `~/imp-archive`. |
| rebuild/mana/imp-work docs | archive | `docs/rebuild`, `docs/mana-next-*`, `docs/design/imp-work-*` | Historical design, not current product. | Archive or move under docs/archive. |
| Lua docs | keep/improve | `docs/extensions-lua.md` | Current extension runtime. | Keep current. |
| RPC docs | internal/decide | `docs/rpc.md` | Nice-to-have. | Mark advanced/internal if kept. |
| ACP/MCP/future docs | archive/future | planned references | Not current product. | Move to roadmap/future. |

## Suggested immediate approval package

Approve or edit these before implementation:

1. retained command list and `/fork` `/copy` `/status` decisions.
2. one-line prompt and appendix replacement for personality/soul.
3. `imp workflow run <id>` as sole initial workflow CLI surface.
4. remove CLI chat.
5. remove `imp-gui` from default members.
6. archive old docs/root clutter to `~/imp-archive`.
