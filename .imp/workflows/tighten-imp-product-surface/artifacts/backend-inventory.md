# Backend Dependency Inventory

Status: draft backend inventory for `tighten-imp-product-surface`.

Purpose: map bloated or experimental user-facing surfaces to their supporting backend code, then classify each as keep, remove, archive, fold into workflows, or decide later.

## Classification key

- **Keep** — still core to target product.
- **Remove** — remove command/product surface and supporting backend when dependencies allow.
- **Archive** — move nontrivial historical/design/experimental code or docs to `~/imp-archive` before deleting active path.
- **Fold into workflow** — preserve useful behavior as workflow-native data/control, not a separate command/product concept.
- **Decide** — needs explicit product call before removal.

## Core surfaces to keep

### TUI runtime/session backend — Keep

Key files:

- `crates/imp-tui/src/app.rs`
- `crates/imp-tui/src/interactive.rs`
- `crates/imp-tui/src/event_source.rs`
- `crates/imp-tui/src/views/chat.rs`
- `crates/imp-tui/src/views/sidebar.rs`
- `crates/imp-tui/src/views/session_picker.rs`
- `crates/imp-tui/src/views/tree.rs`
- `crates/imp-core/src/session.rs`
- `crates/imp-core/src/imp_session.rs`
- `crates/imp-core/src/session_index.rs`

Rationale:

- TUI is the primary product surface.
- `/resume`, `/new`, `/name`, `/tree`, and session continuity are core.

Cleanup note:

- TUI app file is too large; extract after command/backend cuts reduce complexity.

### One-shot prompt backend — Keep

Key files:

- `crates/imp-cli/src/lib.rs` — `run_print_mode`
- `crates/imp-core/src/builder.rs`
- `crates/imp-core/src/agent/*`

Rationale:

- One-shot is the primary non-interactive path.

Cleanup note:

- Preserve `imp -p`/prompt behavior while cutting CLI chat.

### Workflow backend — Keep, but make workflow-native

Key files:

- `crates/imp-core/src/workflow/`
- `crates/imp-core/src/tools/workflow.rs`
- `.imp/workflows/*`
- `docs/workflows.md`

Rationale:

- Workflow is the durable orchestration primitive.

Problems to fix:

- Controller and integration are still mana-shaped.
- Schema includes prototypes as first-class entries.
- Workflow profiles leak into top-level slash commands.

Disposition:

- **Keep core workflow.**
- **Fold profile/task shortcut behavior into workflows.**
- **Rewrite controller/integration away from mana roots/units over time.**

### Lua extension backend — Keep

Key files:

- `crates/imp-lua/src/*`
- `crates/imp-core/src/tools/lua.rs`
- `crates/imp-core/src/tools/mod.rs` Lua loader hooks
- `crates/imp-tui/src/app.rs` Lua command/tool loading and `/reload`
- `docs/extensions-lua.md`

Rationale:

- Current shipped extension support is Lua.
- Aligns with minimal extensibility/Pi influence.

Cleanup note:

- Keep `/reload` because it supports config/extensions.
- Do not market TypeScript extensions as shipped unless retained deliberately.

## High-priority removal/fold candidates

### `imp-gui` crate — Remove from default, decide archive/delete later

Key files:

- `Cargo.toml` default-members and members
- `crates/imp-gui/Cargo.toml`
- `crates/imp-gui/src/lib.rs`
- `crates/imp-gui/src/main.rs`
- GUI/wireframe docs such as `docs/tui-workflow-wireframes.md` and root HTML artifacts if still present

Current issue:

- `imp-gui` is in `default-members`, so it participates in default workspace builds despite not being target product.

Recommended first cut:

- Remove `crates/imp-gui` from `default-members`.

Later:

- If not active, archive crate to `~/imp-archive/imp-gui/` or `~/imp-archive/experiments/imp-gui/` and remove from workspace members.

Classification:

- **Remove from default now.**
- **Archive/delete later.**

### Personality / soul editor backend — Remove or replace with minimal prompt config

User signal:

- User suspects `/personality` is unnecessary and backend may be removable.

Key files:

- `crates/imp-core/src/personality.rs` (~977 lines)
- `crates/imp-core/src/system_prompt.rs` personality/soul prompt layer
- `crates/imp-core/src/resources.rs` project soul discovery helpers
- `crates/imp-tui/src/views/personality.rs`
- `crates/imp-tui/src/app.rs`
  - `UiMode::Personality`
  - `open_personality`
  - `handle_personality_key`
  - `save_personality`
  - `/personality` dispatch/help
- `crates/imp-tui/src/views/command_palette.rs` `/personality`
- `crates/imp-cli/src/lib.rs`
  - `Commands::Personality`
  - `ChatShellCommand::Personality`
  - `run_personality_mode`
  - soul path/load/save/tunable helpers
- docs/proposals:
  - `docs/proposals/soul-md-design-2026-04-05.md`

Behavior today:

- `soul.md` can customize identity/tunables.
- TUI and CLI can edit global/project soul.
- System prompt injects personality/soul content.

Concern:

- Duplicates system prompt doctrine.
- Adds a large UI/backend for questionable product value.
- Conflicts with desire for minimal configurable prompt.

Recommended path:

1. Add/plan `PromptConfig` for minimal identity/append/prepend.
2. Remove `/personality` command and TUI/CLI editors.
3. Decide whether to keep simple `soul.md` loading as a generic prompt appendix, or remove all personality-specific parsing/tunables.
4. Archive `docs/proposals/soul-md-design-2026-04-05.md` and maybe the old `personality.rs` implementation if deleted.

Classification:

- **Remove user-facing command/editor.**
- **Likely remove backend after prompt config replacement.**
- **Archive design doc.**

### CLI chat shell — Remove or freeze unsupported

Key files:

- `crates/imp-cli/src/lib.rs`
  - `Commands::Chat`
  - `ChatShellCommand`
  - `parse_chat_shell_command`
  - `handle_chat_shell_command`
  - `run_chat_shell` / interactive loop around line 4900+
  - CLI chat tests around line 5713+
- `docs/rebuild/imp-cli-interactive-shell.md`
- README/docs references if present

Behavior today:

- Adds `:` grammar plus slash compatibility.
- Has partial parity issues (`:compact` planned/unavailable).
- Includes `:personality`, `:view`, `:settings`, `:setup`, model/thinking commands.

Concern:

- User does not want to cater to CLI chat.
- TUI + one-shot are enough.
- CLI chat adds grammar/docs/test/product burden.

Recommended path:

1. Remove `imp chat` command or mark hidden/internal for one release only if necessary.
2. Remove chat shell parser/handler/tests.
3. Preserve one-shot `run_print_mode` and TUI startup.
4. Archive CLI shell design doc.

Classification:

- **Remove.**
- **Archive docs.**

### Mana compatibility backend — Fold into workflow or remove

Key files:

- `crates/imp-core/src/tools/mana.rs` (~5,561 lines, behind `mana-tool`)
- `crates/imp-core/src/mana_worker.rs`
- `crates/imp-core/src/mana_run_state.rs`
- `crates/imp-core/src/mana_review.rs`
- `crates/imp-core/src/mana_prompt_context.rs`
- `crates/imp-core/src/mana_next/*`
- `crates/imp-core/src/agent/mana_loop.rs`
- `crates/imp-core/src/agent/workflow_integration/mana_compat.rs`
- `crates/imp-tui/src/views/mana_navigator.rs`
- `crates/imp-tui/src/views/settings.rs` mana settings fields
- `crates/imp-tui/src/app.rs`
  - `/mana`
  - `/scope`
  - mana run/scope/status/stop handling
  - active mana scope context injection
- `crates/imp-cli/src/lib.rs`
  - `mana-ui` feature command namespace
  - headless mana worker mode
- Cargo features:
  - `imp-core/mana-api`
  - `imp-core/mana-tool`
  - `imp-cli/mana-ui`
  - `imp-tui/mana-ui`
- docs:
  - `docs/mana-next-*`
  - mana-heavy rebuild/proposal docs

Behavior today:

- Mana is compatibility-oriented but still deeply shapes workflow controller, settings, CLI, and prompt follow-ups.

Concern:

- Product target says workflows replace mana/imp-work as visible durable orchestration.
- Mana markdown files can remain as historical data; active compatibility need is low.

Recommended path:

1. Remove `/mana` and `/scope` from default TUI command surface.
2. Remove mana fields from default settings UI.
3. Rewrite workflow controller/integration away from mana terminology before fully removing feature-gated backend.
4. Archive mana-heavy docs.
5. Remove `mana-tool`/`mana-ui` features once workflow-native parity is sufficient.

Classification:

- **Fold durable value into workflow.**
- **Remove from default product path.**
- **Archive compatibility docs/code when cut.**

### Improve mode backend — Fold into workflow or remove

Key references:

- `crates/imp-tui/src/app.rs`
  - `/improve`
  - `/improve-safe`
  - `/improve-merge`
  - `/improve-help`
  - improve safe mode/worktree context injection
- `crates/imp-tui/src/views/settings.rs`
  - `ImproveAutoTurnBudget`
- `crates/imp-core/src/config.rs`
  - improve auto-turn budget config
- docs/rebuild/proposal references to improve/worktree workflows

Behavior today:

- A parallel product mode for autonomous improvement, sandboxing, merge/clean/help.

Concern:

- Top-level improve mode competes with workflows.
- Worktree/sandboxing is useful but should be workflow execution policy, not a slash command family.

Recommended path:

1. Remove `/improve*` commands from palette/help/dispatch.
2. Remove improve-specific settings field unless workflow runtime still uses it.
3. Preserve useful sandbox/worktree execution as workflow option if needed.

Classification:

- **Fold into workflow.**
- **Remove top-level command/backend mode.**

### Eval candidate backend — Remove or keep internal dev tool only

Key files:

- `crates/imp-core/src/eval_candidate.rs`
- `crates/imp-core/src/eval_candidate_closeout.rs`
- `crates/imp-cli/src/lib.rs`
  - `EvalCommand`
  - save/list/show candidate commands
- `crates/imp-tui/src/app.rs`
  - `/eval`
- docs:
  - `docs/eval-candidates.md`

Behavior today:

- Records failure cases/candidates for future evaluation.

Concern:

- Product surface is bloated.
- Workflow artifacts/checks can capture regressions/failures more coherently.

Recommended path:

1. Remove `/eval` from TUI.
2. Decide whether CLI `imp eval` remains internal hidden dev command.
3. Fold useful failure records into workflow closeout/evidence schema.
4. Archive docs if command removed.

Classification:

- **Remove from product surface.**
- **Fold into workflow evidence or keep internal only.**

### Prototype tool/backend — Fold into workflow or remove

Key files:

- `crates/imp-core/src/tools/prototype.rs`
- `crates/imp-core/src/workflow/schema.rs` `prototypes` field
- `crates/imp-core/src/tools/workflow.rs` prototype update paths
- `docs/workflows.md` and README prototype references

Behavior today:

- Runs bounded disposable code experiments and records structured prototype observations.
- Workflow schema treats prototypes as first-class.

Concern:

- Prototype is a useful work pattern, but not a separate product primitive.
- Ordinary `bash`/files plus workflow evidence can cover experiments.

Recommended path:

1. Stop advertising prototypes as a top-level workflow concept.
2. Remove `PrototypeTool` unless a strong model-facing need remains.
3. Convert workflow prototype fields to generic artifacts/evidence/checks in a later schema migration if desired.

Classification:

- **Fold into workflow artifacts/evidence.**
- **Likely remove standalone tool.**

### Memory command/tool/backend — Decide; likely make prompt-config/data only

Key files:

- `crates/imp-core/src/memory.rs`
- `crates/imp-core/src/tools/memory.rs`
- `crates/imp-core/src/learning.rs`
- `crates/imp-core/src/system_prompt.rs` memory/user profile layers
- `crates/imp-tui/src/app.rs`
  - `/memory`
  - `handle_memory_command`
- README/docs/tools references
- docs/proposals/imp-memory-architecture-and-mana-boundary.md

Behavior today:

- Persistent memory/user profile can be surfaced/injected.
- `/memory` slash command lets user inspect/add memory.

Concern:

- User-visible `/memory` is likely product bloat.
- Prompt memory layers add hidden behavior/prompt bulk.

Recommended path:

1. Remove `/memory` from TUI product surface.
2. Add prompt config to disable memory/user profile by default in minimal preset.
3. Keep raw memory files only if useful as optional context.
4. Revisit `MemoryTool` after prompt config lands.

Classification:

- **Decide.**
- **Likely remove command, keep optional data layer initially.**

### Workflow profile slash commands — Fold into workflows, stop top-level surfacing

Key files:

- `crates/imp-core/src/workflow_profiles.rs`
- `crates/imp-tui/src/views/command_palette.rs`
  - `merge_workflow_commands`
- `crates/imp-tui/src/app.rs`
  - `workflow_registry`
  - `try_workflow_command`
  - `show_workflow_profile`
- docs/workflow-profiles.md

Behavior today:

- Built-in profiles like plan/review/verify/implement/research/debug can appear as slash commands.

Concern:

- `/plan`, `/run`, `/debug`, `/review`, `/verify` feel bloated.
- Profiles should guide workflow creation/execution internally or through future `/work` aliases, not as many top-level commands.

Recommended path:

1. Disable profile commands in default slash palette/dispatch.
2. Keep workflow profile registry only if workflow runtime uses it internally.
3. In future, route `/work`/`/workflow` aliases to active workflow control, not every profile.

Classification:

- **Fold into workflow.**
- **Remove top-level slash surfacing.**

### Checkpoints/restore backend — Decide; likely hide/remove until real restore

Key references:

- `crates/imp-tui/src/app.rs`
  - `/checkpoints`
  - `/restore-checkpoint`
  - message says TUI restore is not wired yet
- `crates/imp-cli/src/lib.rs`
  - `imp view checkpoints`
- session checkpoint APIs in `crates/imp-core/src/session.rs`

Concern:

- Exposing incomplete restore feels unfinished.

Recommended path:

- Remove `/restore-checkpoint` from product surface until real restore works.
- Keep passive checkpoint records if session/runtime needs them.
- Maybe keep `view checkpoints` as internal diagnostics.

Classification:

- **Remove/hide command.**
- **Keep backend if used by sessions.**

### Autonomy/queue/clean/hotkeys/export/status commands — Mostly remove or relocate

Key references:

- `crates/imp-tui/src/views/command_palette.rs`
- `crates/imp-tui/src/app.rs`
- `crates/imp-core/src/config.rs` autonomy policy
- `crates/imp-core/src/policy.rs`, `reference_monitor.rs`

Disposition by command:

- `/autonomy` — remove command; keep policy/config if runtime needs it.
- `/queue` — remove if `/loop` + `/stop` and visible TUI state are enough.
- `/clean` — remove if only improve/sandbox cleanup.
- `/hotkeys` — remove or replace with docs/help overlay if actively useful.
- `/export` — decide; likely non-core.
- `/status` — decide; maybe TUI should show state without command.

Classification:

- **Remove product commands.**
- **Keep underlying runtime policy where core.**

## Prompt/config backend

### System prompt assembly — Keep, make configurable/minimal

Key files:

- `crates/imp-core/src/system_prompt.rs`
- `crates/imp-core/src/builder.rs`
- `crates/imp-core/src/config.rs`
- `crates/imp-core/src/agent/mod.rs` runtime follow-up prompts
- `crates/imp-core/src/agent/loop_policy.rs`
- `crates/imp-core/src/workflow/controller.rs`

Recommended path:

1. Introduce `PromptConfig` with presets/layers.
2. Add minimal default.
3. Make personality/memory/facts/skills/mode verbosity configurable.
4. Make workflow controller strictness configurable.

Classification:

- **Keep and refactor.**

### Workflow controller — Keep, redesign lighter and workflow-native

Key files:

- `crates/imp-core/src/workflow/controller.rs`
- `crates/imp-core/src/agent/workflow_integration/*`
- `crates/imp-core/src/agent/mod.rs` next-action integration

Concern:

- User is curious about this controller; it currently drives rigid continuation.
- State is mana-shaped:
  - `mana_root_id`
  - active unit id
  - child runs
  - graph/direct closeout required
  - bootstrap/decomposition/supervision gates

Recommended path:

1. Document controller behavior separately.
2. Add `workflow.controller.mode = off|light|strict` plan.
3. Default normal TUI to light: suggest/ask instead of hidden forced continuation when ambiguous.
4. Reserve strict for explicit workflow worker/child runs.
5. Rewrite mana-root references to workflow-native step/check/artifact state.

Classification:

- **Keep concept.**
- **Refactor heavily.**

## Docs/archive candidates

Archive to `~/imp-archive` or move under `docs/archive/` before active docs cleanup:

- `docs/rebuild/*` old migration docs that assume compatibility-first direction.
- `docs/mana-next-*` if mana-next no longer target product.
- `docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md`.
- `docs/proposals/soul-md-design-2026-04-05.md` if personality removed.
- `docs/eval-candidates.md` if eval candidate product surface removed.
- GUI/wireframe docs if `imp-gui` archived.
- root HTML/draft/prototype artifacts if present.

## Recommended cut sequence

1. Remove `imp-gui` from default members.
2. Tighten TUI command palette/dispatch to retained commands; stop surfacing workflow-profile commands.
3. Remove `/personality` command/editor and replace prompt customization with planned prompt config.
4. Remove CLI chat shell.
5. Remove improve/eval/prototype product surfaces, folding valuable parts into workflows.
6. Remove `/mana` and `/scope` from TUI product path; isolate mana compatibility.
7. Add prompt config and minimal prompt preset.
8. Refactor workflow controller to light/strict modes and workflow-native state.
9. Archive docs/experiments.
10. Add `.imp/runs` retention/global-storage policy.

## Open decisions for user

1. Should `/fork` be retained as core?
2. Should `/copy` be retained as core?
3. Should `/status` be removed, retained, or replaced by always-visible TUI state?
4. Should RPC remain first-class for product launch, or become internal/advanced?
5. Should `soul.md` survive as a generic prompt appendix after removing personality UI, or should personality/soul be fully removed?
6. Should `MemoryTool` survive as optional advanced feature, or should memory be only file/context based?
