# CLI Workflow Surface Plan

Status: draft target CLI surface for `tighten-imp-product-surface`.

## Product intent

`imp workflow run <id>` should be the minimal human/automation entrypoint for workflow execution.

This CLI should support:

- cron/automation invoking a workflow by id
- manual terminal execution outside the TUI
- eventual subagent/worker execution
- durable multi-step coding sessions without relying on ambient chat/controller prompts

It should not become another broad command surface.

## Current state

The model-facing native `workflow` tool already supports:

- `list`
- `show`
- `validate`
- `run`
- `update`

Evidence:

- `crates/imp-core/src/tools/workflow.rs`
- `WorkflowAction::{List, Show, Validate, Run, Update}`
- `run_action()` validates a workflow and returns the next runnable step/worker assignment.

Current `workflow run` behavior is not a full autonomous runner. It currently returns the next workflow action, including optional worker assignment data.

The human CLI in `crates/imp-cli/src/lib.rs` has many subcommands, but no obvious first-class `imp workflow run <id>` namespace yet.

## Target minimal CLI

Initial product surface:

```sh
imp workflow run <id>
```

That is enough for the first version.

Expected behavior:

1. Load `.imp/workflows/<id>/workflow.yaml` from the current project.
2. Validate the workflow strictly.
3. Select the next runnable step/check using existing workflow logic.
4. Execute or prepare execution for that step according to current workflow-run capability.
5. Persist meaningful status/result updates.
6. Print a concise human result.
7. Exit non-zero on validation errors, blockers, failed checks, or runtime errors.

## Important semantic distinction

`workflow(action="run")` as a model tool currently means:

> inspect workflow and return the next runnable action/assignment.

`imp workflow run <id>` should eventually mean:

> actually advance the workflow by executing the next step/check, possibly by launching a scoped agent worker.

For the first implementation slice, it is acceptable for `imp workflow run <id>` to wrap the current next-action behavior if we name/output it honestly. But the target is an executor, not only an inspector.

## Out of scope for first CLI surface

Avoid adding flags until the basic path is real.

Do not start with:

```sh
--mode autonomous
--max-hours
--max-turns
--parallel
--worker
--json
--watch
--cron
--continue
--all
--dry-run
```

These can come later only if proven useful.

## Future flags, explicitly deferred

Possible later additions:

```sh
imp workflow run <id> --json
imp workflow run <id> --once
imp workflow run <id> --max-turns N
imp workflow run <id> --max-duration 4h
imp workflow run <id> --strict
imp workflow run <id> --worker <step>
```

But each should require a real use case.

## Relationship to TUI

Normal TUI should stay natural and lightweight.

- TUI can create/update/show workflows.
- TUI should not rely on an ambient strict workflow controller.
- TUI can eventually offer a command or keybinding that calls the same workflow runner.
- `/work` is not part of this workflow; do not add it now.

## Relationship to ambient workflow controller

Target direction:

- defunct ambient workflow controller as the main source of strictness.
- move strict orchestration into explicit workflow execution, starting with `imp workflow run <id>` and the native workflow run action.

In other words:

```text
normal chat/TUI = model-trusting, minimal prompt
explicit workflow run = rigid, durable, resumable orchestration
```

## Relationship to subagents

The workflow schema already supports worker assignment data in `tools/workflow.rs`:

- worker id
- role
- objective
- result path
- checks
- dependencies
- writable scope
- worktree
- responsibilities
- instructions

Target runner behavior:

1. select runnable step
2. if step has worker assignment, spawn a scoped agent/subagent
3. pass only the workflow id, step id, objective, checks, writable scope, and result path
4. worker writes result/artifacts
5. runner validates/checks result and updates workflow

This is future target behavior, not necessarily first slice.

## Happy path examples

### Manual execution

```sh
imp workflow run tighten-imp-product-surface
```

Possible output:

```text
Workflow: tighten-imp-product-surface
Step: backend_inventory
Action: execute
Result: wrote artifacts/backend-inventory.md
Verification: workflow validate passed
Next: decide_target_surface
```

### Cron execution

Cron can call the same minimal command:

```sh
cd /path/to/project && imp workflow run nightly-maintenance
```

The command should be deterministic enough to stop with a useful exit code and result summary.

## First implementation slice proposal

1. Add `Commands::Workflow(WorkflowCli)` in `crates/imp-cli/src/lib.rs`.
2. Add only one subcommand:

```rust
WorkflowCommand::Run { id: String }
```

3. Reuse existing workflow load/validate/next-action functions or the existing workflow tool internals.
4. Print the current next-action result clearly.
5. Do not add autonomous loops yet.
6. Do not add flags yet.
7. Add tests for CLI parsing and output behavior.

## Medium-term implementation direction

After minimal CLI exists:

1. Extract workflow run logic from `tools/workflow.rs` into a core service so both model tool and CLI can reuse it without going through tool JSON plumbing.
2. Add a real workflow runner that can execute one step by launching a scoped agent session.
3. Move strict closeout/verification prompts into step-scoped runner prompts.
4. Remove or disable ambient workflow controller in normal TUI.
5. Add subagent execution when worker assignments are ready.

## Open design questions

1. Should the first `imp workflow run <id>` execute one step, or only return the next runnable action until the runner service exists?
2. Should `imp workflow run <id>` default to strict validation only, or allow draft workflows initially?
3. Should CLI output be human-only at first, or should `--json` be included immediately for cron?

Current preference:

- Execute one step eventually, but first slice may return next action honestly.
- Strict validation by default.
- Human-only first; add `--json` later when automation format is designed.
