# Prototype workflow tool results

## Summary

Implemented the first native model-facing `workflow` tool. V1 is read-only and backed by the Rust `imp.workflow/v1` parser/validator.

## Tool actions implemented

- `list`: discovers `.imp/workflows/*/workflow.yaml` under the current project and summarizes each workflow.
- `show`: renders a selected workflow's title, status, goal, acceptance progress, steps, pending checks, parent/child workflow links, results path, and validation diagnostics.
- `validate`: validates one workflow or all workflows and renders object-path diagnostics.

## Code changes made

- Added `crates/imp-core/src/tools/workflow.rs`.
- Registered `WorkflowTool` in `register_native_tools`.
- Exposed `workflow` module from `crates/imp-core/src/tools/mod.rs`.
- Reused `workflow::schema::{load_workflow, validate_workflow, ValidateOptions}` through public workflow re-exports.

## Tests added

- `workflow_tool_list_discovers_workflows`
- `workflow_tool_show_renders_status`
- `workflow_tool_validate_all_passes_for_dogfood_workflows`

## Verification performed

```sh
cargo fmt --package imp-core
cargo test -p imp-core workflow_tool
```

Result:

```text
3 passed; 0 failed; 0 ignored; 0 measured; 887 filtered out
```

The broader schema tests also remained passing during this work:

```sh
cargo test -p imp-core workflow_schema
```

Result from the last schema run:

```text
5 passed; 0 failed; 0 ignored; 0 measured; 882 filtered out
```

## Limitations

- Tool is read-only; no workflow mutation yet.
- No `plan`, `update`, `run`, or `close` actions yet.
- No event log integration yet.
- No slash command integration yet.
- No TUI-specific rendering yet.
- `.imp/workflows/**` remains ignored by git by project choice.

## Recommended next workflow

Continue with:

```text
.imp/workflows/implement-workflow-update-events/workflow.yaml
```

Next goal: implement safe workflow artifact mutation and append-only `events.jsonl` support before adding `workflow run`.
