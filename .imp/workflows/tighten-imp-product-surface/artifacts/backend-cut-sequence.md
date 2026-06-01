# Backend Cut Sequence

Status: draft cut sequence for `tighten-imp-product-surface`.

This artifact turns the audit into an ordered implementation plan. It is planning only; no code has been removed.

## Sequencing principles

1. Cut broad/default product surface before deleting deep backend code.
2. Prefer changes that reduce build/docs/command bloat with low behavior risk first.
3. Replace old backend responsibilities with workflow-native or prompt-builder seams before removing large modules.
4. Keep TUI and one-shot working after every slice.
5. Avoid compatibility shims unless they protect a still-targeted surface.
6. Archive historical docs/code before destructive deletion when the material has design value.

## Required verification baseline

Use narrow checks per slice, then periodic broader checks.

Minimum recurring checks:

```sh
workflow validate tighten-imp-product-surface
cargo check -p imp-core -p imp-tui -p imp-cli
```

When touching prompt/controller code:

```sh
cargo test -p imp-core system_prompt --lib
cargo test -p imp-core workflow::controller --lib
```

When touching CLI parsing:

```sh
cargo test -p imp-cli --lib
```

When touching TUI commands:

```sh
cargo check -p imp-tui
```

## Phase 0 — approval and final decisions

Before implementation, confirm:

- `/fork`: keep or remove.
- `/copy`: keep or remove.
- `/status`: command or visible UI state only.
- RPC: launch-facing advanced docs or internal only.
- memory: remove entirely from prompt/product surface or keep as plain optional files/context.
- archive destination: outside-repo `~/imp-archive` vs in-repo `docs/archive` for each doc group.

No code should be removed until these are resolved or explicitly deferred.

## Phase 1 — low-risk product/build surface cuts

### 1. Remove `imp-gui` from default members

Files:

- `Cargo.toml`

Change:

- remove `crates/imp-gui` from `default-members`.
- keep as workspace member initially.

Classification:

- remove from default / optional experiment.

Verify:

```sh
cargo metadata --no-deps --format-version=1
cargo check --workspace --exclude imp-gui
```

Risk:

- low; affects default build surface only.

### 2. Tighten visible TUI command palette

Files:

- `crates/imp-tui/src/views/command_palette.rs`
- `crates/imp-tui/src/app.rs`

Change:

- visible default commands become retained command list.
- remove workflow-profile commands from default top-level slash surfacing.
- do not add `/work`.
- decide treatment for `/fork`, `/copy`, `/status` first.

Classification:

- remove/fold old command surface.

Verify:

```sh
cargo check -p imp-tui
```

Risk:

- medium because `app.rs` dispatch is large and command discovery includes built-ins, Lua, skills, and workflow profiles.

### 3. Remove unfinished/defunct commands from help/discovery

Files:

- `crates/imp-tui/src/app.rs`
- `crates/imp-tui/src/views/command_palette.rs`

Targets:

- `/session`
- `/restore-checkpoint`
- `/checkpoints` if not retained
- `/hotkeys` if not retained

Classification:

- remove/incomplete product surface.

Verify:

```sh
cargo check -p imp-tui
```

Risk:

- low/medium.

## Phase 2 — minimal prompt builder and prompt-layer cuts

### 1. Introduce simple prompt config, not presets

Files:

- `crates/imp-core/src/config.rs`
- `crates/imp-core/src/system_prompt.rs`
- `crates/imp-core/src/builder.rs`

Target config shape:

```toml
[prompt]
system = "You are imp, a practical and helpful software engineer. Use available tools, skills, and workflows when they help. Read files before editing them."
tools = true
skills = true
project_instructions = true
environment = true
append = ["~/.imp/prompt.md", ".imp/prompt.md"]
```

Avoid:

- presets
- compact/full modes
- personality/memory/project_facts/guardrails/mode_doctrine/workflow_doctrine toggles

Classification:

- improve prompt builder.

Verify:

```sh
cargo test -p imp-core system_prompt --lib
```

Risk:

- medium/high because prompt behavior changes agent quality.

### 2. Remove personality/soul prompt layer from default

Files:

- `crates/imp-core/src/system_prompt.rs`
- `crates/imp-core/src/resources.rs`
- `crates/imp-core/src/personality.rs` later
- `crates/imp-core/src/config.rs`

Change:

- generic prompt appendices replace `soul.md`/personality layer.
- `~/.imp/prompt.md` and `.imp/prompt.md` can contain any custom identity/soul text.

Classification:

- remove personality concept / improve prompt config.

Verify:

```sh
cargo test -p imp-core system_prompt --lib
cargo check -p imp-core
```

Risk:

- medium; many config/UI paths may reference personality.

### 3. Remove verbose default doctrine from system prompt

Files:

- `crates/imp-core/src/system_prompt.rs`
- tests in same file

Change:

- one-line identity plus tools/skills/project instructions/appendices/environment.
- remove global workflow doctrine, status-label doctrine, mode sermons, guardrail prompt injection, memory/user profile injection from default path.
- runtime-specific prompts remain scoped to explicit task/workflow execution.

Classification:

- improve prompt minimalism.

Verify:

```sh
cargo test -p imp-core system_prompt --lib
```

Risk:

- high; should be reviewed manually with prompt snapshots.

## Phase 3 — CLI workflow runner surface

### 1. Add minimal `imp workflow run <id>`

Files:

- `crates/imp-cli/src/lib.rs`
- likely new module if extracting from large file is acceptable:
  - `crates/imp-cli/src/workflow.rs` or `crates/imp-cli/src/commands/workflow.rs`
- workflow core service extraction if needed:
  - `crates/imp-core/src/workflow/runner.rs`

Change:

- add only `imp workflow run <id>`.
- no flags initially.
- strict validation by default.
- first implementation may print next runnable action honestly if full execution service is not ready.

Classification:

- keep/improve workflow automation.

Verify:

```sh
cargo test -p imp-cli --lib
cargo check -p imp-cli
workflow validate tighten-imp-product-surface
```

Risk:

- medium; `imp-cli/src/lib.rs` is large and currently owns many modes.

### 2. Extract shared workflow run/next-action service

Files:

- `crates/imp-core/src/tools/workflow.rs`
- `crates/imp-core/src/workflow/*`

Change:

- make CLI and model tool share core workflow run selection logic without JSON-tool plumbing.
- this enables future real execution/subagents.

Classification:

- improve architecture.

Verify:

```sh
cargo test -p imp-core workflow --lib
cargo test -p imp-cli --lib
```

Risk:

- medium.

## Phase 4 — defunct ambient workflow controller

### 1. Disable ambient controller for normal TUI/chat/one-shot

Files:

- `crates/imp-core/src/agent/run_loop.rs`
- `crates/imp-core/src/agent/workflow_integration/*`
- `crates/imp-core/src/workflow/controller.rs`
- TUI/RPC event rendering where snapshots are surfaced

Change:

- normal TUI/chat/one-shot should not receive hidden workflow controller continuation prompts.
- keep strictness only for explicit workflow-run/headless worker execution if still needed.

Classification:

- remove ambient rigidity / improve runtime.

Verify:

```sh
cargo test -p imp-core workflow::controller --lib
cargo test -p imp-core agent --lib
cargo check -p imp-tui -p imp-cli
```

Risk:

- high; touches run-loop behavior.

### 2. Move strict prompts into workflow runner

Files:

- new/existing workflow runner service
- `crates/imp-core/src/workflow/controller.rs` as temporary source of policies

Change:

- step-scoped workflow-run prompts replace ambient hidden follow-up prompts.
- runner records workflow step/check results.

Classification:

- fold controller value into workflow runner.

Verify:

```sh
cargo test -p imp-core workflow --lib
cargo check -p imp-cli
```

Risk:

- high.

### 3. Remove mana-shaped controller state

Files:

- `crates/imp-core/src/workflow/controller.rs`
- `crates/imp-core/src/agent/workflow_integration/mana_compat.rs`
- event/snapshot serialization code

Change:

- remove `mana_root_id`, `active_unit_id`, mana graph closeout semantics from workflow-native controller/runner.

Classification:

- remove/fold mana compatibility.

Verify:

```sh
cargo test -p imp-core workflow --lib
cargo check -p imp-core -p imp-cli -p imp-tui
```

Risk:

- high.

## Phase 5 — remove CLI chat

Files:

- `crates/imp-cli/src/lib.rs`
- docs/README references
- tests in `imp-cli`

Remove:

- `Commands::Chat`
- `ChatShellCommand`
- `parse_chat_shell_command`
- `run_chat_shell`
- chat shell tests
- slash compatibility in CLI chat

Keep:

- TUI launch
- one-shot print mode
- optional RPC mode
- setup/login/secrets/settings if still needed as CLI commands

Classification:

- remove product surface.

Verify:

```sh
cargo test -p imp-cli --lib
cargo check -p imp-cli
```

Risk:

- medium/high due to large `lib.rs` and shared helpers.

## Phase 6 — remove personality command/backend

Files:

- `crates/imp-core/src/personality.rs`
- `crates/imp-core/src/config.rs`
- `crates/imp-core/src/system_prompt.rs`
- `crates/imp-core/src/resources.rs`
- `crates/imp-tui/src/views/personality.rs`
- `crates/imp-tui/src/app.rs`
- `crates/imp-tui/src/views/command_palette.rs`
- `crates/imp-cli/src/lib.rs`
- docs/proposals/soul-md-design-2026-04-05.md

Remove:

- `/personality`
- TUI personality UI mode/view
- CLI personality command/chat command
- config personality profiles/sliders if no longer referenced
- soul.md discovery/product concept

Replace with:

- generic prompt appendix files from prompt config.

Classification:

- remove/archive.

Verify:

```sh
cargo test -p imp-core system_prompt --lib
cargo check -p imp-core -p imp-tui -p imp-cli
```

Risk:

- high; backend is cross-cutting.

## Phase 7 — remove/fold improve, eval, prototype, memory surfaces

### Improve mode

Remove/fold:

- `/improve*` commands
- improve safe/worktree context injection
- improve settings if no longer used

Fold useful sandbox/worktree behavior into explicit workflow runner later.

Verify:

```sh
cargo check -p imp-tui -p imp-core
```

Risk:

- medium.

### Eval candidates

Remove/fold:

- TUI `/eval`
- CLI eval command if not kept internal
- eval candidate docs/product framing

Fold useful data into workflow evidence/artifacts.

Verify:

```sh
cargo check -p imp-core -p imp-cli -p imp-tui
```

Risk:

- medium.

### Prototype tool

Remove/fold:

- `tools/prototype.rs` if no longer registered/needed
- workflow schema prototype fields only after schema migration decision
- docs product references

Fold into workflow artifacts/evidence.

Verify:

```sh
cargo test -p imp-core workflow --lib
cargo check -p imp-core
```

Risk:

- medium/high if workflow schema/tests depend on prototypes.

### Memory command/tool

Remove/fold:

- `/memory`
- `MemoryTool` if not useful as internal optional context
- prompt memory/user profile injection by default

Potentially keep plain memory files/context as optional appendices.

Verify:

```sh
cargo test -p imp-core system_prompt --lib
cargo check -p imp-core -p imp-tui
```

Risk:

- medium.

## Phase 8 — isolate/remove mana compatibility

Files:

- `crates/imp-core/src/tools/mana.rs`
- `crates/imp-core/src/mana_*.rs`
- `crates/imp-core/src/agent/mana_loop.rs`
- `crates/imp-core/src/agent/workflow_integration/mana_compat.rs`
- `crates/imp-tui/src/views/mana_navigator.rs`
- `crates/imp-cli/src/lib.rs` `mana-ui` paths
- Cargo features across crates

Change:

- remove `/mana` and `/scope` first.
- then isolate feature-gated mana backend as non-default/internal.
- eventually archive/remove if workflows fully replace it.

Classification:

- remove/fold into workflows/archive compatibility.

Verify:

```sh
cargo check -p imp-core -p imp-tui -p imp-cli
cargo check -p imp-core --no-default-features
```

Risk:

- high; many feature-gated paths and tests may rely on compatibility.

## Phase 9 — docs/archive cleanup

Files:

- README
- docs/*
- root prototype/design docs

Change:

- rewrite README to launch product.
- move root prototypes/docs per `archive-plan.md`.
- move old rebuild/mana/imp-work/personality/CLI-chat/eval/prototype docs.
- update links.

Verify:

```sh
rg -n "imp-work|mana-next|/personality|imp chat|prototype results|eval candidates|/improve|/mana|/scope" README.md docs
workflow validate tighten-imp-product-surface
```

Risk:

- low behavior risk, medium documentation/link risk.

## Phase 10 — storage/runtime cleanup

### `.imp/runs` retention/global storage

Change:

- define retention policy.
- avoid project-local `.imp/runs` ballooning.
- add prune/GC later if useful.

Classification:

- improve runtime/storage hygiene.

Risk:

- medium; run artifacts may be useful for debugging/RPC.

### Runtime/RPC/ACP boundary

Change:

- keep runtime events/state if they support TUI/RPC/workflow runner.
- avoid GUI/ACP-specific product commitments until needed.
- document RPC as optional advanced if kept.

Classification:

- keep/internal/improve.

Risk:

- medium.

## Recommended first implementation batch

After user approval, implement in this order:

1. Remove `imp-gui` from default members.
2. Tighten visible TUI command palette to retained commands.
3. Add simple prompt config target or at least reduce system prompt toward one-line identity.
4. Add `imp workflow run <id>` minimal CLI shell.
5. Remove CLI chat.
6. Remove personality UI/backend.

Rationale:

- Batch starts with high product-value cuts.
- It avoids deep mana/controller deletion until workflow runner has a replacement seam.
- It creates prompt/workflow architecture before deleting the backends that depend on old prompt/controller semantics.

## Current blocking decisions

- `/fork`, `/copy`, `/status` fate.
- RPC launch-facing vs internal.
- memory survival as optional file/context or complete removal.
- whether first prompt implementation should be pure one-line immediately or staged behind config.
- archive destination per doc group.
