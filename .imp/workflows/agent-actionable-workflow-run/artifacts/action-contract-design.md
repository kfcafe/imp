# Agent-actionable workflow run design

## Problem

`workflow run` can now orchestrate command checks across dependent steps, but it stops at non-command steps because those steps do not tell the runtime how to execute agent judgment work.

A runnable step is only fully actionable when it declares one of:

- pending command checks;
- a child workflow reference;
- a worker/agent action contract;
- an explicit decision/approval boundary that should block.

A plain context/plan/build/review step with only review/artifact checks is underspecified for automation.

## Proposed schema extension

Add an optional `action` field to `WorkflowStep`:

```yaml
steps:
  inspect_current_tui_surface:
    kind: context
    status: todo
    action:
      kind: agent
      role: coder
      objective: Inspect current TUI workflow rendering seams.
      instructions:
        - Read crates/imp-tui/src/app.rs workflow command handling.
        - Read crates/imp-core/src/tools/workflow.rs run result formatting.
        - Write a concise implementation note.
      write_scope:
        - .imp/workflows/tui-workflow-run-progress/artifacts/view-model.md
      completion:
        artifacts:
          - .imp/workflows/tui-workflow-run-progress/artifacts/view-model.md
        checks:
          - current_tui_workflow_surface_inspected
```

Fields:

- `kind`: `agent` or `worker`.
- `role`: agent role/profile to run, e.g. `coder`, `reviewer`, `verifier`.
- `worker`: optional named worker id for `kind: worker`.
- `objective`: required human-readable task objective.
- `instructions`: ordered execution guidance.
- `write_scope`: allowed write paths/globs for the dispatched agent.
- `completion.checks`: workflow checks expected to be passed when action is complete.
- `completion.artifacts`: artifacts expected to exist when action is complete.

## Run behavior

`workflow run` should loop as an orchestrator:

1. validate workflow;
2. find next runnable step;
3. run command checks if pending;
4. if step has an action contract, emit/dispatch an agent action contract;
5. if step has no executable path, return a clear missing-action-contract blocked state;
6. reload workflow after each completed action;
7. reconcile checks, acceptance, closeout, and workflow status.

Initial implementation may render the agent dispatch contract without spawning a nested agent. The important behavior is that the step is no longer ambiguous: it is either dispatchable or clearly blocked.

## Human-readable output

Do not show raw JSON. Render agent dispatch like:

```text
Workflow needs agent action: inspect_current_tui_surface [context]

Role: coder
Objective: Inspect current TUI workflow rendering seams.

Instructions:
- Read crates/imp-tui/src/app.rs workflow command handling.
- Read crates/imp-core/src/tools/workflow.rs run result formatting.
- Write a concise implementation note.

Allowed writes:
- .imp/workflows/tui-workflow-run-progress/artifacts/view-model.md

Completion:
- check: current_tui_workflow_surface_inspected
- artifact: .imp/workflows/tui-workflow-run-progress/artifacts/view-model.md
```

Missing contract output should be explicit:

```text
Workflow blocked: missing action contract for step inspect_current_tui_surface [context].
Add command checks, a child workflow, worker, or action contract.
```

## Validation

Strict validation should reject malformed action contracts:

- unknown `action.kind`;
- missing `objective`;
- `kind: worker` without `worker` or known worker id;
- completion checks that do not exist;
- empty action contract with no command/worker/workflow alternative.

It should not require every planned step to have an action contract immediately unless the step is runnable and otherwise non-actionable; draft workflows still need room to evolve.

## TUI implications

The TUI should consume structured run details but render a polished text/card view:

- workflow id/title/status;
- current step and role;
- step/check progress;
- recent command/agent events;
- blocked reason or completion summary;
- results path.

Users should not have to run `/workflow run`; the agent runtime should call workflow run and the TUI should display the resulting progress.
