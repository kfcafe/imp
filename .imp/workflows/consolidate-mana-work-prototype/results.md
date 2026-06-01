# Consolidate mana work prototype results

## Summary

Audited the current mana, work/prototype, workflow-profile, child-workflow, and workflow tool responsibilities around imp-native workflows. This slice consolidated the intended boundaries in documentation rather than moving code.

## Work completed

- Inspected existing implementation areas:
  - `crates/imp-core/src/tools/workflow.rs`
  - `crates/imp-core/src/tools/mana.rs`
  - `crates/imp-core/src/mana_worker.rs`
  - `crates/imp-core/src/tools/prototype.rs`
  - `crates/imp-core/src/workflow_profiles.rs`
  - `crates/imp-core/src/agent/workflow_integration/mana_compat.rs`
  - `crates/imp-core/src/workflow/child_workflow.rs`
  - `crates/imp-core/src/agent/subagent.rs`
- Wrote `.imp/workflows/consolidate-mana-work-prototype/artifacts/plan.md` with the current responsibility map and consolidation direction.
- Added `docs/design/imp-workflow-responsibility-boundaries.md` as the durable design note.

## Consolidation decisions

- `.imp/workflows` remains the imp-native process ledger.
- Mana remains the durable graph and canonical mana-unit worker substrate.
- Workflow profiles remain UX modes, not persistent workflow state.
- Prototype execution remains isolated; future workflow integration should record observations as workflow artifacts/check evidence.
- `mana_compat` remains migration glue and should shrink as workflow-native progress events/checks mature.
- `workflow run` remains advisory until assignment contracts, verification gates, and installed-runtime behavior are stable.

## Verification

- `workflow validate` — 11 workflows ok, 0 diagnostics.
- `cargo test -p imp-core tools::workflow::tests::workflow_run_returns_next_runnable_step` — passed: 1 passed, 0 failed.

## Concerns and next steps

- No broad code refactor was done in this slice by design.
- Follow-up candidates:
  - workflow check/evidence kinds for prototype observations;
  - explicit workflow-native progress events to replace parts of `mana_compat` inference;
  - role-aware child workflow spawning from `WorkflowWorkerAssignmentContract`;
  - mana-backed workflow step references when a step corresponds to a mana unit.
