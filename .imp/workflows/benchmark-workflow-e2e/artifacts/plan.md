# Benchmark workflow E2E plan

Goal: prove the native workflow engine can drive a real workflow lifecycle before the API feature begins.

## Scenario

Use this workflow itself as the benchmark target, with a harmless fixture artifact at:

- `.imp/workflows/benchmark-workflow-e2e/artifacts/fixture.txt`

The fixture starts as `pending` and is changed to `complete` during the benchmark. The workflow records every important transition through the native `workflow` tool update path, then verifies validation, events, failure behavior, and closeout gating.

## Expected lifecycle

1. Validate initial workflow state.
2. Prepare fixture artifact.
3. Use `workflow run benchmark-workflow-e2e` after each major transition to confirm the next expected step.
4. Use `workflow update` to mark steps/checks complete.
5. Confirm `.imp/workflows/benchmark-workflow-e2e/events.jsonl` receives append-only records for updates.
6. Attempt an invalid update and confirm it fails without mutating `workflow.yaml`.
7. Confirm closeout remains gated until required verify/review checks are complete.
8. Write final `results.md`, mark aggregate/final checks, validate final workflow state, and close.

## Evidence to capture

- validation command results
- run-engine next actions observed
- event log line count and representative records
- invalid update failure output
- fixture final content
- final workflow status and remaining concerns
