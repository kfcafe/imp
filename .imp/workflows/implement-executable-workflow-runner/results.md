# Implement Executable Workflow Runner Results

## Summary

Implemented the first executable `workflow run` path for native imp workflows.

`workflow run` now:

- selects the next runnable workflow step;
- executes pending command checks attached to that step;
- records each command check as `passed` or `failed`;
- updates the step to `done` when all executed command checks pass;
- updates the step to `failed` when any executed command check fails;
- appends `events.jsonl` entries for check and step status changes;
- returns a concise run summary with check status and exit code.

If the next runnable step has no pending executable command checks, the tool preserves the existing advisory behavior and returns the next actionable step.

## Commits

- `9ad1769 Run command checks from workflow run`
- `7b808a8 Update workflow step status after command checks`
- `7c8e86f Document executable workflow run behavior`

Related product-surface cleanup commits completed before this runner workflow:

- `c0855bd Tighten imp product surface`
- `e70ed58 Remove personality TUI internals`
- `9df91ec Remove hidden improve TUI internals`
- `2fe4d52 Archive stale product surface docs`

## Verification

Final verification passed:

```sh
cargo fmt --check -p imp-core
cargo check -p imp-core
cargo test -p imp-core workflow --lib
cargo run -p imp-cli -- workflow run executable-runner-dogfood
```

Observed:

- `cargo test -p imp-core workflow --lib`: 119 passed.
- Dogfood workflow initially executed a command check and advanced its step to `done`.
- Dogfood rerun reported `No runnable workflow steps`, confirming persisted completion.

## Dogfood Artifact

See `.imp/workflows/implement-executable-workflow-runner/artifacts/dogfood-run.md`.

## Remaining Follow-ups

- Derive acceptance/check aggregate status automatically after command checks pass.
- Complete workflow status/closeout automatically when closeout requirements are satisfied.
- Reuse or consolidate with `VerificationGateRunner` artifact capture if richer stdout/stderr artifacts are needed for workflow checks.
- Add timeout and output truncation controls for `checks.<id>.command` execution in the native workflow tool path.
