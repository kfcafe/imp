# Implement workflow update events plan

## Goal

Add a small, safe mutation layer for workflow artifacts and append-only workflow events.

## V1 scope

Implement update support narrowly enough to let the model update workflow status/check fields without hand-editing YAML:

- append structured events to `.imp/workflows/<id>/events.jsonl`;
- apply a simple object-path update to `workflow.yaml`;
- validate the workflow after mutation;
- reject goal/acceptance/check weakening for now rather than trying to model approval flows;
- keep writeback minimal and deterministic.

## Tool action

Add `workflow.update` to the existing `workflow` tool.

Input shape:

```json
{
  "action": "update",
  "id": "prototype-workflow-tool",
  "path": "steps.verify.status",
  "value": "done",
  "reason": "cargo test -p imp-core workflow_tool passed"
}
```

## Initial supported paths

Only support scalar status updates and simple append-safe fields first:

- `status`
- `steps.<step_id>.status`
- `checks.<check_id>.status`
- `prototypes.<prototype_id>.status`
- `spec.acceptance.<acceptance_id>.status`

Do not support arbitrary structural mutation yet.

## Event format

Append one JSON object per update:

```json
{
  "timestamp": "2026-05-26T00:00:00Z",
  "action": "update",
  "path": "steps.verify.status",
  "value": "done",
  "reason": "cargo test -p imp-core workflow_tool passed"
}
```

## Safety rules

- `id`, `path`, `value`, and `reason` are required for update.
- Only known status paths are allowed.
- Status values must be valid for the target kind.
- Validate after applying the update.
- If validation fails, do not write `workflow.yaml`.
- Append event after successful validation and atomic workflow write.
- Do not implement approval-requiring changes in v1.

## Implementation plan

1. Add status serialization support where needed.
2. Add a small YAML mutation helper in `tools/workflow.rs` or a workflow schema/update module.
3. Add atomic workflow write helper.
4. Add `events.jsonl` append helper.
5. Add `workflow update` action.
6. Add tests using a temporary workflow directory, not the dogfood workflows.

## Verification

```sh
cargo test -p imp-core workflow_update
cargo test -p imp-core workflow_
```
