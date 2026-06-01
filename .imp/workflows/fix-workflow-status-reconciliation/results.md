# Fix Workflow Status Reconciliation Results

## Summary

Implemented narrow workflow status reconciliation after executable command checks run.

`workflow run` now derives existing workflow statuses instead of requiring manual bookkeeping:

- acceptance criteria become `done` when all referenced checks are `passed`;
- workflow status becomes `done` when closeout requirements are passed and all steps are terminal;
- reconciliation writes `events.jsonl` entries;
- run output includes a `Reconciled:` line when derived statuses change.

This streamlines the existing workflow model. It does not add new workflow concepts or orchestration layers.

## Commit

- `b5d43f2 Reconcile workflow status after command checks`

## Verification

Final review verification passed:

```sh
cargo fmt --check -p imp-core
cargo check -p imp-core
cargo test -p imp-core workflow --lib

test -s .imp/workflows/fix-workflow-status-reconciliation/artifacts/dogfood.md
yq -e '.status == "done" and .spec.acceptance.command_check_reconciles.status == "done" and .steps.verify_reconciliation.status == "done" and .checks.dogfood_command.status == "passed"' .imp/workflows/reconciliation-dogfood/workflow.yaml
```

Observed:

- `cargo test -p imp-core workflow --lib`: 119 passed.
- Dogfood workflow reconciled command check, step, acceptance, and workflow status successfully.

## Dogfood Artifact

See `.imp/workflows/fix-workflow-status-reconciliation/artifacts/dogfood.md`.

## Notes / Remaining Follow-ups

- Reconciliation currently runs after executable command checks in the native workflow tool path.
- It intentionally uses existing workflow schema fields and avoids adding new concepts.
- Future improvements can broaden reconciliation to manual `workflow update` paths if desired.
