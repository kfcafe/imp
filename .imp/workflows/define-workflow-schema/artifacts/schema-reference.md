# Workflow schema reference

This document defines the compact YAML shape for `imp.workflow/v1` strongly enough to start the first Rust parser and validator prototype.

## Design goals

- Keep the human-authored artifact tasteful and concise.
- Use map-based IDs instead of repeated `id` fields.
- Keep workflow instances readable in git diffs.
- Preserve enough structure for deterministic validation and runtime enforcement.
- Let workflows call other workflows through normal steps.
- Let acceptance criteria carry verification status directly.

## Canonical file

Each workflow instance is stored at:

```text
.imp/workflows/<workflow-id>/workflow.yaml
```

Related artifacts live beside it:

```text
.imp/workflows/<workflow-id>/events.jsonl
.imp/workflows/<workflow-id>/results.md
.imp/workflows/<workflow-id>/artifacts/
```

## Required top-level fields

```yaml
schema: imp.workflow/v1
id: define-workflow-schema
title: Define workflow schema
status: active
kind: schema_definition

settings: {}
spec: {}
context: {}
steps: {}
prototypes: {}
checks: {}
workers: {}
results: {}
closeout: {}
```

Required fields:

- `schema`: currently `imp.workflow/v1`.
- `id`: stable workflow id, matching the workflow directory name.
- `title`: human-readable title.
- `status`: workflow lifecycle status.
- `kind`: workflow kind/profile marker.
- `settings`: workflow-local settings.
- `spec`: goal, value, non-goals, acceptance, and approval policy.
- `context`: context requirements/status.
- `steps`: executable workflow steps keyed by step id.
- `prototypes`: prototype questions keyed by prototype id.
- `checks`: requirements/verification checks keyed by check id.
- `workers`: reusable worker definitions keyed by worker id.
- `results`: result artifact configuration.
- `closeout`: terminal status requirements.

Optional fields:

- `parent`: parent workflow link for a workflow called by another workflow.
- `notes`: freeform workflow notes.
- `decisions`: structured decisions, if the workflow needs them outside `results.md`.

## Map-based IDs

Collections use mapping keys as IDs:

```yaml
steps:
  refine_shape:
    kind: prototype
```

This avoids verbose repeated `id` fields and makes duplicate IDs impossible in a valid YAML mapping.

## Spec

```yaml
spec:
  goal: Refine the concise YAML schema.
  user_value: Make workflow artifacts readable and enforceable.
  non_goals:
    - Implement runtime execution.
  acceptance:
    concise_schema_shape_defined:
      text: The v1 compact YAML shape is defined.
      status: todo
      checks: [schema_reference_written]
  approval_required_for:
    - changing spec.goal
    - removing or weakening spec.acceptance
    - removing required checks
```

`acceptance` is a map so every criterion can carry its own verification state and linked checks.

## Steps

```yaml
steps:
  build_runtime:
    kind: build
    status: todo
    worker: builder
    depends_on: [define_validation]
    checks: [rust_plan_written]
```

Step fields:

- `kind`: required. Built-in kinds include `context`, `plan`, `prototype`, `build`, `verify`, `review`, `workflow`, `decision`, `closeout`.
- `status`: required.
- `depends_on`: optional list of step ids.
- `checks`: optional list of check ids.
- `worker`: optional worker id.
- `prototypes`: optional list of prototype ids for `prototype` steps.
- `workflow`: required when `kind: workflow`; references another workflow id.

## Workflow calls

A called workflow is represented as a normal step:

```yaml
steps:
  define_schema:
    kind: workflow
    workflow: define-workflow-schema
    status: todo
    checks: [child_workflow_results_ready]
```

The child workflow links back:

```yaml
parent:
  workflow: prototype-imp-workflow-engine
  step: define_schema
```

Parent and child workflows live as sibling directories under `.imp/workflows/`.

## Checks

```yaml
checks:
  rust_plan_written:
    kind: artifact
    status: pending
    path: .imp/workflows/define-workflow-schema/artifacts/rust-parser-plan.md
```

Check fields:

- `kind`: required. Initial kinds: `context`, `artifact`, `command`, `review`, `approval`, `aggregate`, `closeout`.
- `status`: required.
- `requires`: optional list of check ids for aggregate checks.
- `path`, `file`, `artifact`, `command`, `question`, `decision`: kind-specific fields.

`checks` replaces earlier `gates` terminology.

## Prototypes

```yaml
prototypes:
  concise_schema:
    question: What is the most concise tasteful YAML shape?
    hypothesis: Map-based IDs remove most repeated structure.
    status: active
    criteria:
      - taste
      - explicit_enough_for_validation
```

Prototypes are top-level so unanswered questions have durable space in the workflow. Prototype steps reference them by id.

## Workers

```yaml
workers:
  builder:
    role: builder
    writes: [code, tests]
    worktree: workflow
```

Workers are reusable objects. Steps reference workers by id. Worker assignments should eventually be workflow-backed rather than thin prompt slices.

## Results and closeout

```yaml
results:
  path: .imp/workflows/define-workflow-schema/results.md
  required_claims:
    - chosen concise YAML schema shape

closeout:
  done:
    requires:
      - required_checks_passed
      - final_claims_validated
  allowed_terminal_statuses:
    - done
    - done_with_concerns
    - blocked
    - needs_context
```

The final answer should be constrained by workflow state and generated from validated results.
