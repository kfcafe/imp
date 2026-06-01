# Agent-actionable workflow run results

Implemented the action-contract and display-ready workflow run slice.

## Schema/action contract behavior

Workflow steps now support optional `action` contracts with:

- action kind: `agent` or `worker`;
- role and optional worker;
- objective;
- instructions;
- write scope;
- completion checks and artifacts.

Validation now rejects malformed action contracts, including empty objectives, missing/unknown worker references for worker actions, and unknown completion checks.

## Run dispatch behavior

`workflow run` now distinguishes:

- command-check orchestration across dependent runnable steps;
- agent-action contracts for non-command agent work;
- missing-action-contract blocked states for underspecified runnable non-command steps;
- existing worker/child workflow dispatch summaries.

Output remains human-readable rather than raw JSON.

## TUI progress implications

The TUI workflow output renderer now consumes the structured workflow run details and renders workflow cards for inline and sidebar output. This gives agent-triggered workflow tool output the same polished display path as `/workflow run` debug output.

## Verification

Passed:

- `cargo test -p imp-core workflow_ --lib`
- `cargo test -p imp-tui workflow_run_ --lib`
- `cargo test -p imp-cli workflow_cli_lists_shows_validates_runs_and_updates --test workflow_cli`
- `cargo check -p imp-tui`
- `cargo run -q -p imp-cli -- workflow validate agent-actionable-workflow-run`
- `cargo run -q -p imp-cli -- workflow validate tui-workflow-run-progress`

## Remaining concerns

Full nested agent execution from an `agent_action` contract is still a runtime follow-up. The current slice makes non-command steps explicit, dispatchable, test-covered, and display-ready; it does not yet spawn a nested agent automatically.
