# Implement workflow run engine plan

## Goal

Add a first `workflow run` action that inspects the selected workflow, validates it, and returns the next runnable workflow action for the agent/runtime to perform.

## V1 scope

Keep this slice advisory and deterministic. `workflow run` should not execute shell commands or dispatch real workers yet. It should return a structured next action based on current workflow state.

## Behavior

Input:

```json
{
  "action": "run",
  "id": "implement-workflow-run-engine"
}
```

Output examples:

```text
Next workflow action: run step execute [build]
Worker: builder
Checks: implementation_ready
```

or:

```text
No runnable steps. Blocked by pending checks: workflow_run_results_ready
```

## Selection rules

- Validate the workflow first in strict mode.
- If diagnostics exist, return a validation-blocked next action.
- Find runnable steps using `next_runnable_steps`.
- Prefer the first runnable step in workflow order.
- Include step kind, worker, checks, child workflow id, and dependencies in details.
- Do not mutate workflow state in this slice.
- Do not run commands in this slice.
- Do not dispatch workers in this slice.

## Parser/schema adjustment

The current `next_runnable_steps` helper only checks dependency status. For run output, keep it conservative but add a richer helper or local renderer that explains why no step is runnable.

## Tests

- `workflow run` returns a runnable step for a pending workflow.
- `workflow run` reports validation diagnostics for an invalid temp workflow.
- `workflow run` reports no runnable steps when all remaining steps are blocked by dependencies.

## Verification

```sh
cargo test -p imp-core workflow_run
cargo test -p imp-core workflow_
```
