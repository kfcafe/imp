# Implement workflow run engine results

## Summary

Implemented the first advisory `workflow run` action. It validates a selected workflow and returns the next workflow action for the agent/runtime to perform without executing commands or dispatching workers yet.

## Code changes made

- Added `run` action to the native `workflow` tool.
- Added `WorkflowRunResult` and `WorkflowNextAction` result structures.
- Added validation-blocked run output.
- Added next-runnable-step output based on `next_runnable_steps`.
- Added no-runnable-steps output with dependency/blocking explanations.
- Added tests for run behavior.

## Behavior implemented

`workflow run` now:

1. Loads the selected workflow.
2. Validates it in strict/draft mode.
3. If validation fails, returns diagnostics as the next action blocker.
4. Otherwise selects the first runnable step.
5. Returns step id, kind, worker, checks, child workflow id, and dependencies.
6. Does not mutate workflow state.
7. Does not execute commands.
8. Does not dispatch workers.

## Tests added

- `workflow_run_returns_next_runnable_step`
- `workflow_run_reports_validation_diagnostics`
- `workflow_run_reports_no_runnable_steps_when_dependencies_block`

## Verification performed

```sh
cargo fmt --package imp-core
cargo test -p imp-core workflow_
```

Result:

```text
49 passed; 0 failed; 0 ignored; 0 measured; 846 filtered out
```

## Limitations

- Run is advisory only.
- It does not execute command checks.
- It does not dispatch workers.
- It does not update step status.
- It does not integrate with `/run` or TUI yet.
- Runnable-step semantics are still conservative and dependency-based.

## Next workflow

Continue with:

```text
.imp/workflows/integrate-workflow-slash-commands/workflow.yaml
```

Goal: wire `/plan`, `/status`, and `/run` user surfaces to the native workflow tool/engine.
