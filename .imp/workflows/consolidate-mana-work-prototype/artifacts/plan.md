# Consolidate mana/work/prototype responsibilities plan

## Current responsibility map

- `.imp/workflows/*/workflow.yaml` is now the dogfood workflow schema for durable imp-native workflow orchestration: steps, checks, workers, child workflow calls, results, and closeout.
- `crates/imp-core/src/tools/workflow.rs` is the workflow-specific tool surface for listing, showing, validating, running next actions, and updating workflow artifacts.
- `crates/imp-core/src/tools/mana.rs` remains the mana graph/native-run tool surface. It owns mana unit discovery, next/list/status/update/run flows, and canonical mana worker dispatch integration.
- `crates/imp-core/src/mana_worker.rs` is the canonical single-mana-unit worker runtime.
- `crates/imp-core/src/tools/prototype.rs` is a disposable experiment runner. Its current `WorkStore` is a no-op compatibility stub, so prototype observations are not yet durably recorded through imp workflows.
- `crates/imp-core/src/workflow_profiles.rs` defines interactive workflow modes (`plan`, `review`, `verify`, `implement`, `research`, `debug`) with roles/tools/instructions. These are prompt/runtime modes, not durable workflow records.
- `crates/imp-core/src/agent/workflow_integration/mana_compat.rs` is transitional glue that treats mana/work tool results as durable workflow progress signals.
- `crates/imp-core/src/workflow/child_workflow.rs` and `crates/imp-core/src/agent/subagent.rs` define child workflow/subagent contracts, but actual workflow tool dispatch remains advisory.

## Consolidation direction

1. Keep workflow artifacts as the process ledger for imp-native orchestration.
2. Keep mana as the durable project graph and existing worker substrate for mana units.
3. Keep workflow profiles as UX modes that can bootstrap or steer a workflow, not as competing durable state.
4. Keep prototype as isolated experiment execution; add workflow result recording later instead of reviving a separate work store.
5. Treat `mana_compat` as migration glue to shrink over time as workflow-native events/checks mature.
6. Prefer bridging, not merging: workflow steps may reference mana units/runs, prototype observations, or child workflow runs as evidence/check artifacts.

## Implementation slice for this workflow

This slice should be documentation/consolidation rather than broad code movement:

1. Add a design note documenting the responsibility boundaries above.
2. Update this workflow's results with inspected files and consolidation decisions.
3. Do not refactor `mana`, `prototype`, or workflow profile code in this slice unless verification reveals an immediate conflict.
4. Verify with the existing workflow-focused test suite.

## Follow-up candidates

- Add workflow check kinds for prototype observations and child workflow results.
- Add a workflow-native event/progress signal so `mana_compat` does less inference from tool result shapes.
- Add a durable prototype recording path under `.imp/workflows/<id>/artifacts/` or `.imp/runs/`.
- Add role-aware mapping from workflow workers to `ChildWorkflowRunner` once assignment contracts are stable.
