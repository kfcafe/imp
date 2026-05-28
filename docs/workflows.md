# Workflows

imp workflows are local project artifacts for planned, multi-step work. They keep the plan, execution state, checks, prototype results, events, and closeout notes in files under the project.

Primary implementation areas:

- `crates/imp-core/src/workflow/schema.rs`
- `crates/imp-core/src/tools/workflow.rs`
- `crates/imp-core/src/workflow/controller.rs`
- `crates/imp-core/src/workflow/child_workflow.rs`

## Layout

```text
.imp/workflows/<id>/
├── workflow.yaml
├── events.jsonl
├── results.md
└── artifacts/
```

`workflow.yaml` is the contract. `events.jsonl` is append-only update history. `results.md` is the human-readable closeout record. `artifacts/` holds plans, outputs, fixture files, review notes, or other supporting evidence.

## Schema

Common top-level fields:

- `schema`
- `id`
- `title`
- `status`
- `kind`
- `parent`
- `settings`
- `spec`
- `context`
- `steps`
- `prototypes`
- `checks`
- `workers`
- `results`
- `closeout`

`spec.acceptance` records user-facing completion criteria. `steps` record the work sequence. `checks` record the verification gates or artifacts that prove a step. `closeout` records terminal status requirements.

## Status values

Workflow, step, and check status values are schema-validated. Current workflows commonly use:

```text
todo
pending
passed
done
done_with_concerns
blocked
needs_context
```

Invalid status updates are rejected before `workflow.yaml` is written. Oversized workflow YAML is rejected before parsing. Successful updates validate the prospective workflow, open/preflight `events.jsonl`, replace `workflow.yaml`, then append the event; this is safer than mutating state without an event sink, but it is not a full crash-proof two-file transaction.

## Tool actions

The native `workflow` tool supports:

```text
list
show
validate
run
update
```

`validate` parses and checks workflow structure. `run` returns the next actionable step. `update` mutates an allowed path and appends an event.

## Lifecycle

```text
inspect → validate → run → update → events → prototype/verify → review → closeout
```

A typical agent loop is:

1. inspect workflow context
2. run `workflow validate`
3. run `workflow run` to select the next step
4. do the work
5. update step/check statuses with reasons
6. verify command/artifact evidence
7. write `results.md`
8. close the workflow with a terminal status

## Events

Each successful update appends a JSON line to `events.jsonl`. Events include the action, path, value, reason, and timestamp. This makes workflow progress inspectable outside the chat transcript.

## Prototyping

Prototype work belongs in the workflow when an implementation decision needs evidence. A prototype entry should state:

- question
- hypothesis
- status
- criteria
- evidence required
- follow-up work

Prototype artifacts should be disposable unless explicitly promoted into production code or documentation.

## Verification and closeout

Checks can represent commands, artifacts, context review, aggregate gates, or manual review. Closeout should not rely only on a narrative claim; it should point to completed checks and a results artifact.

Terminal outcomes used by imp work include:

```text
DONE
DONE_WITH_CONCERNS
BLOCKED
NEEDS_CONTEXT
```

## Current limitations

- Storage is local and file-backed.
- API-addressable workflows are planned, not shipped.
- Direct mutation hardening is still evolving; workflow update should continue to get stricter around premature closeout states.
