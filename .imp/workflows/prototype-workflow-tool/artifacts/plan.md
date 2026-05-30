# Prototype workflow tool plan

## Goal

Add a minimal native model-facing `workflow` tool backed by the Rust workflow schema parser.

## V1 actions

Only implement read-only actions:

- `list`: discover `.imp/workflows/*/workflow.yaml` under the current project and summarize each workflow.
- `show`: show one workflow's title, status, goal, acceptance progress, steps, pending/failed checks, parent link, workflow-call children, and results path.
- `validate`: validate one workflow or all workflows and return diagnostics.

## Non-goals

- No `plan` action yet.
- No `update`/writeback yet.
- No event log mutation yet.
- No `run` engine yet.
- No slash command integration yet.

## Tool shape

Tool name: `workflow`

Input shape:

```json
{
  "action": "list" | "show" | "validate",
  "id": "optional workflow id",
  "mode": "strict" | "draft"
}
```

Defaults:

- `action`: required.
- `id`: optional for `list`; optional for `validate` where absent means validate all; required for `show` unless exactly one workflow exists.
- `mode`: `strict` by default.

## Output expectations

`list` should be concise and useful for model planning:

```text
Workflows:
- prototype-imp-workflow-engine [active] Prototype imp workflow engine
- prototype-workflow-tool [planned] Prototype workflow tool
```

`show` should be a status/scratchpad view:

```text
Workflow: prototype-workflow-tool [planned]
Goal: Add minimal native workflow tool actions: list, show, and validate.
Acceptance: 0/2 done
Steps:
- plan [todo]
- execute [todo]
Checks:
- plan_ready [pending]
Results: .imp/workflows/prototype-workflow-tool/results.md
```

`validate` should return OK or object-path diagnostics.

## Implementation notes

- Add `crates/imp-core/src/tools/workflow.rs`.
- Register `WorkflowTool` in `register_native_tools`.
- Reuse `workflow::schema::{load_workflow, validate_workflow, ValidateOptions}`.
- Keep output text-first with structured JSON details for later UI use.
- Read from `.imp/workflows` relative to `ToolContext.cwd`.
- Treat missing `.imp/workflows` as empty list, not an error for `list`.

## Tests

Add tests around helper functions where possible rather than full runtime tool plumbing:

- list discovers workflow fixtures;
- show renders parent/step/check information;
- validate all returns OK for current dogfood workflows;
- validate returns diagnostics for a temporary broken workflow.

## Verification

```sh
cargo test -p imp-core workflow_tool
cargo test -p imp-core workflow_schema
```
