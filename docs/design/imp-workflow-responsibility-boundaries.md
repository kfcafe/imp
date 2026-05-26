# Imp workflow responsibility boundaries

This note records the current consolidation boundary between imp-native workflows, mana work, prototype experiments, workflow profiles, and child/subagent infrastructure.

## Current owners

### Imp workflow artifacts

`.imp/workflows/<id>/workflow.yaml` is the process ledger for imp-native orchestration. It owns:

- workflow goal, acceptance, and closeout state;
- ordered steps and dependencies;
- check status and evidence references;
- worker declarations and child workflow calls;
- results artifact paths and closeout requirements.

`crates/imp-core/src/tools/workflow.rs` is the tool surface for this ledger: list, show, validate, run the next advisory action, and update workflow state.

### Mana graph and worker runtime

Mana remains the durable project graph and existing work-unit runtime. `crates/imp-core/src/tools/mana.rs` owns mana discovery, unit operations, native run orchestration, and transitional compatibility with existing mana workflows.

`crates/imp-core/src/mana_worker.rs` is the canonical single-unit mana worker runtime. Workflow-native orchestration should bridge to it when a workflow step is explicitly mana-unit-backed, rather than duplicating mana assignment loading.

### Prototype experiments

`crates/imp-core/src/tools/prototype.rs` owns disposable code experiments in isolated scratch directories. Its current `WorkStore` is a no-op compatibility stub, so prototype observations are not yet persisted into workflow state. Future workflow integration should record prototype observations as workflow artifacts/check evidence instead of creating a separate durable work store.

### Workflow profiles

`crates/imp-core/src/workflow_profiles.rs` owns interactive UX modes such as `plan`, `review`, `verify`, `implement`, `research`, and `debug`. These modes wrap prompts with role/tools/instructions. They are not durable workflow records and should not compete with `.imp/workflows` for process state.

### Child workflows and subagents

`crates/imp-core/src/workflow/child_workflow.rs` and `crates/imp-core/src/agent/subagent.rs` define child workflow/subagent contracts and lifecycle vocabulary. `workflow run` currently remains advisory, but now emits worker assignment contracts that can later feed these execution paths.

### Transitional mana compatibility

`crates/imp-core/src/agent/workflow_integration/mana_compat.rs` infers durable workflow progress from mana/work tool result shapes. Treat this as migration glue. As workflow-native checks/events mature, this inference layer should shrink.

## Consolidation decisions

1. Keep `.imp/workflows` as the imp-native process ledger.
2. Keep mana as the durable graph and canonical mana-unit worker substrate.
3. Keep workflow profiles as UX modes, not persistent workflow state.
4. Keep prototype execution isolated; record useful observations through workflow artifacts/checks later.
5. Bridge workflow steps to mana/prototype/child-run evidence explicitly instead of merging all systems into one tool.
6. Keep `workflow run` advisory until assignment contracts, verification gates, and installed-runtime behavior are stable.

## Follow-up seams

- Add workflow check kinds or evidence refs for prototype observations.
- Add explicit workflow-native progress events to replace parts of `mana_compat` inference.
- Add role-aware child workflow spawning from `WorkflowWorkerAssignmentContract`.
- Add mana-backed workflow step references when a workflow step corresponds to a mana unit.
