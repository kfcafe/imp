# Executable Runner Dogfood

Command run:

```sh
cargo run -p imp-cli -- workflow run executable-runner-dogfood
```

Observed output:

```text
Workflow `executable-runner-dogfood` ran 1 command check(s) for step `verify_runner`; step is `done`.
- dogfood_command: passed (exit 0)
```

State changes verified:

- `.imp/workflows/executable-runner-dogfood/workflow.yaml` updated `checks.dogfood_command.status` to `passed`.
- `.imp/workflows/executable-runner-dogfood/workflow.yaml` updated `steps.verify_runner.status` to `done`.
- `.imp/workflows/executable-runner-dogfood/events.jsonl` contains run events for both status changes.

Limitations observed:

- Acceptance status is not yet automatically derived from passed checks.
- Workflow status/closeout is not yet automatically completed.
