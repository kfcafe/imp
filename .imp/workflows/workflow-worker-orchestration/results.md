# Workflow worker orchestration results

## Summary

Implemented the first worker-orchestration slice by strengthening advisory `workflow run` output. Worker-backed runnable steps now include a structured worker assignment contract that can be used as a bounded subagent handoff without spawning background workers yet.

## Changed

- Added `WorkflowWorkerAssignmentContract` to workflow tool run details.
- Included workflow id, step id/kind, objective, normalized role, worker id, result path, checks, dependencies, writable scope, code-write flag, worktree hint, responsibilities, and instructions.
- Mapped workflow `builder` role to the existing `coder` role vocabulary for future child-workflow/subagent integration.
- Added focused test assertions for the structured contract on a dogfood workflow fixture.

## Verification

- `cargo test -p imp-core tools::workflow::tests::workflow_run_returns_next_runnable_step` — passed.
- `cargo test -p imp-core workflow` — passed: 106 passed, 0 failed.

## Concerns and next steps

- This remains advisory; it does not spawn child agents, create worktrees, or run checks automatically.
- Next useful slice is to decide whether workflow-native worker spawning should use `ChildWorkflowRunner`, the runtime-local `SubagentCoordinator`, or the canonical `mana_worker` path for mana-backed tasks.
