# Status Reconciliation Dogfood

Command run:

```sh
cargo run -p imp-cli -- workflow run reconciliation-dogfood
```

Observed output:

```text
Workflow `reconciliation-dogfood` ran 1 command check(s) for step `verify_reconciliation`; step is `done`.
- dogfood_command: passed (exit 0)
Reconciled: spec.acceptance.command_check_reconciles.status, status
```

State verified in `.imp/workflows/reconciliation-dogfood/workflow.yaml`:

- `status`: `done`
- `spec.acceptance.command_check_reconciles.status`: `done`
- `steps.verify_reconciliation.status`: `done`
- `checks.dogfood_command.status`: `passed`

Events verified in `.imp/workflows/reconciliation-dogfood/events.jsonl`:

- `checks.dogfood_command.status` run event
- `steps.verify_reconciliation.status` run event
- `spec.acceptance.command_check_reconciles.status` reconciliation event
- root `status` reconciliation event
