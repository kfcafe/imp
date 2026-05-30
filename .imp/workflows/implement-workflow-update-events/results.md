# Implement workflow update events results

## Summary

Implemented the first narrow `workflow update` action. It can safely update status fields in workflow artifacts and append an event to `events.jsonl` after successful validation.

## Code changes made

- Extended the native `workflow` tool with action `update`.
- Added support for status updates at these paths:
  - `status`
  - `steps.<step_id>.status`
  - `checks.<check_id>.status`
  - `prototypes.<prototype_id>.status`
  - `spec.acceptance.<acceptance_id>.status`
- Added YAML mutation helper for known status paths only.
- Added post-update typed parse and strict validation before writing.
- Added atomic workflow write via temporary file + rename.
- Added append-only `events.jsonl` writes after successful workflow replacement.
- Added tests for successful update and failed invalid status update.

## Verification performed

```sh
cargo fmt --package imp-core
cargo test -p imp-core workflow_
```

Result:

```text
46 passed; 0 failed; 0 ignored; 0 measured; 846 filtered out
```

## Safety behavior

- `id`, `path`, `value`, and `reason` are required.
- Only status paths are supported.
- Invalid status values fail before writing.
- Failed updates do not write `workflow.yaml` and do not append events.
- Successful updates append one JSON object per line to `events.jsonl`.
- Approval-requiring mutations are intentionally not supported in this first slice.

## Limitations

- Update only supports status fields.
- No structural add/remove operations yet.
- No approval flow yet.
- No event replay yet.
- No workflow run engine yet.
- Writeback uses serde_yaml formatting, so future work may need canonical formatting/taste improvements.

## Next workflow

Continue with:

```text
.imp/workflows/implement-workflow-run-engine/workflow.yaml
```

Goal: implement `workflow run` so the tool can return next actions and eventually dispatch workers/checks.
