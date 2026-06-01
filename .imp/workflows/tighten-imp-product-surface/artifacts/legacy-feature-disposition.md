# Legacy Feature Disposition Plan

Status: focused disposition plan for personality/soul, mana, improve, eval, prototype, and memory.

## Purpose

These features are the main sources of conceptual bloat outside the command surface. This artifact maps them to code/doc footprints and classifies each as remove, archive, fold, improve, internal, or decide.

## Summary table

| Feature | Classification | Target |
|---|---:|---|
| personality/soul | remove/archive | replace with generic prompt appendices |
| mana/imp-work | fold/remove/archive | workflows become durable primitive |
| improve mode | fold/remove | useful sandboxing becomes workflow-runner policy |
| eval candidates | fold/internal/remove | workflow evidence/artifacts or internal dev only |
| prototype tool | fold/remove | workflow artifacts/evidence/prototype steps if still needed |
| memory | remove/decide | no default prompt/product surface; maybe plain appendix/context later |

## Personality / soul

### Evidence

Code:

- `crates/imp-core/src/personality.rs`
- `crates/imp-core/src/config.rs` imports/stores `PersonalityConfig`
- `crates/imp-core/src/system_prompt.rs` imports personality, injects soul/personality identity and working-style lines
- `crates/imp-core/src/resources.rs` discovers global/project `soul.md`
- `crates/imp-core/src/storage.rs` exposes soul paths
- `crates/imp-core/src/builder.rs` discovers soul and passes personality/soul to prompt assembly
- `crates/imp-tui/src/views/personality.rs`
- `crates/imp-tui/src/views/mod.rs` exports personality view
- `crates/imp-tui/src/views/command_palette.rs` exposes `/personality`
- `crates/imp-tui/src/app.rs` opens/handles/saves personality UI
- `crates/imp-cli/src/lib.rs` imports personality helpers and implements `Commands::Personality`, `ChatShellCommand::Personality`, `run_personality_mode()`

Docs:

- `docs/proposals/soul-md-design-2026-04-05.md`
- `docs/rebuild/imp-cli-interactive-shell.md` personality shell references
- `docs/rebuild/imp-normalized-storage-contract.md` soul references
- `docs/rebuild/imp-durable-storage-surface-audit.md` soul references

### Disposition

Classification: **remove/archive**.

Target:

- Remove `/personality`.
- Remove CLI personality command and chat shell personality path.
- Remove TUI personality view and app state.
- Remove config-backed personality profile/sliders.
- Remove `soul.md` as a product concept.
- Archive soul/personality design docs.

Replacement:

- Generic prompt appendices:
  - `~/.imp/prompt.md`
  - `.imp/prompt.md`

Users can put any desired “soul” text there manually. No special parser, builder, or sliders.

### Safe order

1. Add prompt appendix support.
2. Stop injecting personality/soul in default prompt.
3. Remove UI/CLI commands.
4. Remove config fields/tests.
5. Remove `personality.rs`, soul discovery/path helpers, and personality view.
6. Archive docs.

### Risk

High: personality touches config, system prompt, builder, TUI, CLI, resources, tests.

## Mana / imp-work

### Evidence

Code/features:

- root `Cargo.toml` includes `mana-core` workspace dependency.
- `crates/imp-core/Cargo.toml` features `mana-api`, `mana-tool`.
- `crates/imp-cli/Cargo.toml` feature `mana-ui`.
- `crates/imp-tui/Cargo.toml` feature `mana-ui`.
- `crates/imp-core/src/tools/mana.rs` is large compatibility/backend tool.
- `crates/imp-core/src/mana_run_state.rs`
- `crates/imp-core/src/mana_worker.rs`
- `crates/imp-core/src/mana_review.rs`
- `crates/imp-core/src/mana_prompt_context.rs`
- `crates/imp-core/src/mana_next/*`
- `crates/imp-core/src/agent/mana_loop.rs`
- `crates/imp-core/src/agent/workflow_integration/mana_compat.rs`
- `crates/imp-core/src/contracts.rs` describes mana worker contracts.
- `crates/imp-tui/src/views/mana_navigator.rs`
- `crates/imp-tui/src/views/settings.rs` contains many mana settings.
- `crates/imp-tui/src/views/editor.rs` has mana scope/run labels.
- `crates/imp-cli/src/lib.rs` has `mana-ui` namespace/headless worker paths/tests.
- `crates/imp-core/src/builder.rs` injects mana prompt context/facts/project memory status under feature.

Docs:

- `docs/mana-next-*`
- `docs/design/imp-work-*`
- `docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md`
- `docs/trust-labels-and-provenance.md` contains many mana/memory/eval references
- `docs/tui-workflow-wireframes.md` mana scope/run UI references
- `docs/rebuild/imp-workflow-feature-inventory.md` already says workflows should replace mana/work/prototype direction

### Disposition

Classification: **fold/remove/archive**.

Target:

- Workflows replace mana/imp-work as active durable primitive.
- Remove visible TUI commands `/mana`, `/scope`, `mana-scope`.
- Remove mana settings from default settings UI.
- Remove mana prompt context/facts/status from default prompt builder.
- Archive mana-next/imp-work docs.
- Keep feature-gated backend only temporarily if workflow runner still depends on it.

### Safe order

1. Remove visible commands/docs first.
2. Remove prompt injection/default settings UI.
3. Refactor workflow controller/runner away from mana root/unit state.
4. Remove or isolate `mana-ui` and `mana-tool` features.
5. Remove backend modules if no code depends on them.

### Risk

Very high: mana is feature-gated but cross-cutting in CLI, TUI, prompt builder, workflow controller, docs, tests.

## Improve mode

### Evidence

Code:

- `crates/imp-tui/src/app.rs`
  - `IMPROVE_CHANGELOG_PATH`
  - `IMPROVE_SANDBOX_METADATA_PATH`
  - `improve_safe_mode_prompt()`
  - `improve_code_mode_prompt()`
  - `ImproveSandbox`
  - `create_improve_sandbox()`
  - `write/read/validate_improve_sandbox_metadata()`
  - `run_improve_merge_command()`
  - `improve_status_label()`
  - `queue_improve_mode_continuation_if_ready()`
  - `improve_merge_command()`
  - `ensure_improve_sandbox()`
  - `set_improve_mode()`
  - `/improve*` dispatch/help
- `crates/imp-tui/src/views/command_palette.rs` exposes `/improve*`
- `crates/imp-tui/src/views/editor.rs` has improve status label
- `crates/imp-tui/src/views/sidebar.rs` and `tool_output.rs` tests include improve/prototype sidebars
- `crates/imp-tui/src/views/settings.rs` has `improve_auto_turn_budget`
- `crates/imp-core/src/config.rs` has improve UI config

Docs:

- root/README and workflow docs reference improve/prototype/eval indirectly
- `docs/tui-workflow-wireframes.md` has improve/workflow-first design notes

### Disposition

Classification: **fold/remove**.

Target:

- Remove `/improve*` commands and Improve mode UI/status.
- Preserve useful idea—sandboxed worktree with changelog/review/merge—as future workflow-runner execution policy, not TUI command family.

### Safe order

1. Remove command palette/help/dispatch.
2. Remove Improve mode auto-loop/state from TUI.
3. Remove improve settings.
4. Move sandbox/worktree logic to workflow runner only if needed.
5. Archive old improve docs/notes.

### Risk

High: Improve is deeply embedded in `app.rs` and TUI status/editor/settings tests.

## Eval candidates

### Evidence

Code:

- `crates/imp-core/src/eval_candidate.rs`
- `crates/imp-core/src/eval_candidate_closeout.rs`
- `crates/imp-cli/src/lib.rs` eval command/list/show/save
- `crates/imp-tui/src/app.rs` `/eval` command and tests
- `crates/imp-core/src/agent` closeout paths may create eval candidate refs

Docs:

- `docs/eval-candidates.md`
- `docs/mana-next-workflow-ledger.md` eval candidate mapping
- `docs/trust-labels-and-provenance.md` eval write provenance references

### Disposition

Classification: **fold/internal/remove**.

Target:

- Remove TUI `/eval` command from product surface.
- Keep CLI eval only if useful as hidden/internal dev command while workflow evidence matures.
- Fold failure capture/regression cases into workflow evidence/artifacts/checks.

### Safe order

1. Remove TUI `/eval` command and docs from launch path.
2. Keep CLI eval temporarily as internal if tests/use cases exist.
3. Design workflow evidence replacement.
4. Remove `eval_candidate*` modules if no longer needed.

### Risk

Medium: narrower than mana/personality, but tied into closeout/evidence tests.

## Prototype tool

### Evidence

Code:

- `crates/imp-core/src/tools/prototype.rs`
- `crates/imp-core/src/tools/mod.rs` tool module
- `crates/imp-core/src/workflow/schema.rs` top-level `prototypes`
- `crates/imp-core/src/tools/workflow.rs` workflow prototype update/read paths
- `crates/imp-tui/src/views/tool_output.rs` prototype card rendering
- `crates/imp-tui/src/views/tools.rs` prototype arg formatting
- `crates/imp-tui/src/views/sidebar.rs` prototype display cases

Docs:

- README workflow prototype results
- `docs/workflows.md` prototype fields/lifecycle
- `docs/rebuild/imp-workflow-feature-inventory.md` says standalone prototype should fold into workflows

### Disposition

Classification: **fold/remove**.

Target:

- Remove standalone `prototype` product/tool concept.
- Keep experiments as workflow artifacts/evidence/checks if needed.
- Remove prototype docs from launch path.

### Safe order

1. Stop advertising prototype in README/docs/tools.
2. Decide workflow schema migration from `prototypes` to generic artifacts/evidence.
3. Remove standalone `PrototypeTool` if not registered/used.
4. Remove TUI prototype-specific rendering only after tool/schema no longer emits it.

### Risk

Medium/high because workflow schema/docs/tests may depend on prototype fields.

## Memory

### Evidence

Code:

- `crates/imp-core/src/memory.rs`
- `crates/imp-core/src/tools/memory.rs`
- `crates/imp-core/src/builder.rs` loads `memory.md` and `user.md` when learning enabled
- `crates/imp-core/src/system_prompt.rs` includes memory/user profile prompt blocks
- `crates/imp-tui/src/app.rs` `/memory` CRUD command and many tests
- `crates/imp-tui/src/views/command_palette.rs` exposes `/memory`
- `crates/imp-core/src/reference_monitor.rs` and trust docs mention memory write gating
- `crates/imp-core/src/learning.rs` likely interacts with memory behavior

Docs:

- README/tools list memory as persistent agent memory
- `docs/tools.md` lists memory
- `docs/proposals/imp-memory-architecture-and-mana-boundary.md`
- `docs/proposals/mana-aware-runtime-context-read-path.md`
- `docs/trust-labels-and-provenance.md` memory provenance/write policies

### Disposition

Classification: **remove/decide**.

Target:

- Remove `/memory` command from TUI product surface.
- Do not inject memory/user profile by default.
- Decide whether to keep any memory backend as internal optional context.
- Preferred product replacement: prompt appendices and workflows, not special memory command/tool.

### Safe order

1. Remove `/memory` command and docs from product surface.
2. Disable default memory/user profile prompt injection in prompt builder.
3. Keep `memory.rs` backend temporarily if learning/reference monitor paths depend on it.
4. Remove `MemoryTool` if no longer registered/needed.
5. Archive memory architecture docs if not revived as optional feature.

### Risk

Medium/high: memory is tied to learning config, prompt assembly, reference monitor/provenance, TUI tests.

## Recommended cross-feature order

1. Add minimal prompt builder with prompt appendices.
2. Remove visible commands first:
   - personality
   - memory
   - mana/scope
   - improve/eval/prototype-facing commands
3. Remove prompt injection layers:
   - soul/personality
   - memory/user
   - mana facts/status
4. Remove CLI chat/personality.
5. Remove personality backend.
6. Fold improve/eval/prototype into workflow evidence/runner plans.
7. Defunct ambient workflow controller and mana compatibility after workflow runner seam exists.
8. Archive docs.

## Questions for user

1. Is memory fully cut from product, or kept only as plain optional files/context?
2. Should CLI eval remain hidden/internal until workflow evidence exists?
3. Is any part of improve sandboxing important enough to preserve immediately in workflow runner design?
4. Should mana feature-gated code be archived aggressively once visible surface is gone, or kept temporarily until workflow runner is mature?
