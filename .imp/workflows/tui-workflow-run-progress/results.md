# TUI workflow run progress results

Implemented the workflow-progress display slice.

## UI behavior changed

Workflow tool output now renders through a shared workflow card formatter for both inline tool output and the sidebar. `workflow run` results are presented as structured, human-readable progress instead of raw JSON.

Covered run states:

- command orchestration: workflow id/status, step/check counts, completed steps;
- agent action: workflow id/status, step, role, objective, instructions, allowed writes, completion checks/artifacts;
- missing action contract: workflow id/status, step, blocked reason;
- worker/child run step;
- validation blocked;
- no runnable steps.

## Agent workflow run path

Agent-triggered workflow tool output uses the same `styled_workflow_output` path as sidebar rendering, so inline/interleaved chat output appears as a workflow card above the prompt area rather than plain/raw output.

## Slash command behavior

`/workflow run` remains available as a debug path. The workflow tool output it produces is rendered with the same workflow card formatter when displayed by the TUI tool-output views.

## Verification

Passed:

- `cargo test -p imp-tui workflow_run_ --lib`
- `cargo check -p imp-tui`
- `cargo test -p imp-core workflow_ --lib`
- `cargo test -p imp-cli workflow_cli_lists_shows_validates_runs_and_updates --test workflow_cli`
- `cargo run -q -p imp-cli -- workflow validate tui-workflow-run-progress`
- `cargo run -q -p imp-cli -- workflow validate agent-actionable-workflow-run`

## Remaining concerns

The TUI card consumes workflow tool results once they exist. Fully automatic nested agent execution from an `agent_action` contract is still a runtime orchestration follow-up; this slice makes those contracts explicit and displayable.
