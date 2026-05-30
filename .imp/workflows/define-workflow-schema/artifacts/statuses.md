# Workflow status vocabulary

This document defines the initial status vocabulary for `imp.workflow/v1`.

## General principle

Use human-readable lifecycle words. Keep status values small and unsurprising. Statuses should be strict enough for validation but not so granular that agents constantly churn them.

## Workflow statuses

- `planned`: workflow exists but execution has not started.
- `active`: workflow is currently being worked.
- `waiting`: workflow is waiting on user input, approval, worker result, or external condition.
- `blocked`: workflow cannot continue without a concrete unblocker.
- `done`: workflow completed cleanly.
- `done_with_concerns`: workflow produced useful output but some required work/checks remain incomplete, skipped, or explicitly concerning.
- `needs_context`: workflow needs user/project context before it can proceed.
- `cancelled`: workflow was intentionally stopped.
- `failed`: workflow attempted execution and failed without a clean recovery path.

## Step statuses

- `todo`: step is not ready or not started.
- `ready`: dependencies are satisfied and the step can run.
- `active`: step is currently being worked.
- `waiting`: step is waiting on approval, worker result, command, or external condition.
- `blocked`: step cannot proceed until a blocker is resolved.
- `done`: step completed.
- `done_with_concerns`: step completed with explicit concerns.
- `skipped`: step was intentionally skipped with recorded reason.
- `failed`: step failed.

## Check statuses

- `pending`: check has not been satisfied yet.
- `passed`: check is satisfied.
- `failed`: check ran or was evaluated and failed.
- `blocked`: check cannot currently be evaluated.
- `skipped`: check was intentionally skipped with approval or recorded concern.

Checks use `passed` rather than `done` because checks assert conditions.

## Prototype statuses

- `proposed`: prototype question is captured but not selected for execution.
- `active`: prototype is being worked.
- `supported`: prototype supports its hypothesis.
- `refuted`: prototype refutes its hypothesis.
- `inconclusive`: prototype did not produce enough evidence.
- `selected`: prototype result was chosen as the path forward.
- `discarded`: prototype result was not chosen and can be discarded except for summary/results.

## Acceptance statuses

- `todo`: acceptance criterion is not yet satisfied.
- `done`: acceptance criterion is satisfied.
- `blocked`: acceptance criterion is blocked.
- `needs_context`: acceptance criterion needs user/project context.
- `skipped`: acceptance criterion was explicitly skipped; clean DONE should usually be blocked unless approved.

Acceptance criteria can link to checks. The acceptance status should summarize the criterion; checks provide the proof.

## Terminal status rule

The workflow engine may only allow clean `done` when closeout requirements pass. Otherwise it must choose or report one of `done_with_concerns`, `blocked`, `needs_context`, `cancelled`, or `failed`.
