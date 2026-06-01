# Workflow validation rules

This document defines the first validation rules for `imp.workflow/v1`.

## File-level rules

- `workflow.yaml` must parse as YAML.
- Top-level required fields must exist.
- `schema` must be `imp.workflow/v1`.
- `id` should match the workflow directory name.
- Map-based collections must be YAML maps: `steps`, `checks`, `workers`, `prototypes`, `spec.acceptance`.

## Reference rules

- Every `step.depends_on[]` entry must reference an existing step id.
- Every `step.checks[]` entry must reference an existing check id.
- Every `step.prototypes[]` entry must reference an existing prototype id.
- Every `step.worker` entry must reference an existing worker id.
- Every `check.requires[]` entry must reference an existing check id.
- Every `spec.acceptance.*.checks[]` entry must reference an existing check id.
- Every `closeout.done.requires[]` entry must reference an existing check id unless it is a built-in closeout predicate such as `no_unapproved_goal_or_acceptance_changes`.
- A `kind: workflow` step must provide `workflow` and the referenced workflow directory should exist when validation is not in draft mode.
- A child workflow with `parent` must point to an existing parent workflow and parent step, and the parent step must call the child workflow id.

## Status rules

- Workflow, step, check, prototype, and acceptance statuses must be from their allowed status sets.
- A step should not be `done` if any required checks listed on that step are not `passed`, unless the step is `done_with_concerns` or `skipped` with a reason.
- A workflow should not be `done` unless closeout requirements pass.
- A workflow may be `done_with_concerns` when useful artifacts exist but required checks or downstream implementation remain incomplete.

## Shape rules

- `spec.goal` is required and must be non-empty.
- `spec.acceptance` must be a map of acceptance objects.
- Every acceptance object must have `text` and `status`.
- `steps.*.kind` and `steps.*.status` are required.
- `checks.*.kind` and `checks.*.status` are required.
- `prototypes.*.question` and `prototypes.*.status` are required.
- `workers.*.role` is required.
- `results.path` is required.
- `closeout.done.requires` is required for workflows that can finish as `done`.

## Dependency rules

- Step dependency graph must be acyclic.
- Check dependency graph through `requires` must be acyclic.
- The engine should compute runnable steps by requiring all `depends_on` steps to be terminal-success statuses and all required step checks to be `passed` or not yet required depending on step kind.

## Mutation rules

- The agent may add checks, steps, prototypes, context entries, and results.
- Changing `spec.goal` requires approval.
- Removing or weakening `spec.acceptance` requires approval.
- Removing or weakening required checks requires approval.
- The workflow tool should append an event before mutating canonical state.

## Closeout rules

- Clean `done` requires every `closeout.done.requires` check/predicate to pass.
- Final answer claims must be supported by `results.md`, check results, or referenced artifacts.
- If required checks are pending/failed/skipped, closeout must be `done_with_concerns`, `blocked`, `needs_context`, `cancelled`, or `failed`.
