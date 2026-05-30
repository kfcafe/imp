# Workflow tool audit edge cases

## Baseline real-artifact trials

- `workflow list`: passed. Listed 24 workflows after adding this audit workflow.
- `workflow validate --strict`: passed. 24 workflows ok, 0 diagnostics.
- `workflow show audit-and-repair-workflows --strict`: passed. Rendered active status, steps, pending checks.
- `workflow run audit-and-repair-workflows --strict`: passed. Selected `trial_read_paths` as next runnable step with dependency/check context.
- `workflow show define-workflow-schema --draft`: passed. Rendered completed workflow with parent and child workflow reference.
- `workflow validate define-workflow-schema --draft`: passed.

## Edge cases exercised in tests

- Invalid update value is rejected before writing YAML or events.
- Event log unavailable (`events.jsonl` is a directory) is rejected before replacing `workflow.yaml`.
- Validation diagnostics block `run` and are returned as structured output instead of panicking.
- Missing/running dependencies produce no-runnable/blocked-step diagnostics.

## Findings

- Confirmed defect: `workflow update` previously replaced YAML before opening the append-only event log. If event append/open failed, state could mutate without audit history. Fixed in `crates/imp-core/src/tools/workflow.rs` by opening the event log before replacing YAML and adding regression coverage.
- Workflow artifact gotcha: `.imp/` is ignored by git status, so new workflow artifacts do not appear in normal status output. Operators should explicitly inspect `.imp/workflows/...` paths or use the workflow tool.

## Remaining audit targets

- Policy/mode behavior for workflow actions.
- Path traversal/id handling for selected workflow ids.
- `update` event write failure after YAML rename (disk-full/write-error class) remains theoretically non-atomic because POSIX cannot guarantee both files without a journal; current fix covers preflight/open failures.

## Additional finding: workflow id path traversal

- Confirmed defect: explicit workflow `id` values were joined directly onto `.imp/workflows`. Absolute ids and `../` traversal could address workflow-like files outside the workflow root.
- Fix: `workflow_id_root` now rejects absolute ids and any non-normal path components before `show`, `run`, or `update` loads a selected workflow.
- Regression: `workflow_rejects_absolute_or_parent_directory_ids` covers show/run/update rejection.
