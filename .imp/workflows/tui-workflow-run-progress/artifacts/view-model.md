# Workflow progress view model

The TUI workflow progress display is driven by structured workflow tool result details and rendered as a human-readable card rather than raw JSON.

## Inputs

- `details.action == "run"`
- `details.result.id`
- `details.result.status`
- `details.result.next_action.kind`

Supported next action kinds:

- `orchestrated_command_checks`
- `agent_action`
- `missing_action_contract`
- `run_step`
- `validation_blocked`
- `no_runnable_steps`

## Display fields

Common card fields:

- workflow id
- workflow status
- state-specific summary
- original plain output as fallback/detail

Agent action cards show:

- step
- role
- objective
- instructions
- allowed writes
- completion checks
- completion artifacts

Command orchestration cards show:

- number of steps run
- number of checks run
- each completed step and status

Blocked cards show:

- blocked step
- missing action contract reason or validation state

## Rendering paths

Both inline tool output and sidebar output call the same `styled_workflow_output` renderer in `crates/imp-tui/src/views/tool_output.rs`, so agent-triggered workflow run output and `/workflow run` debug output are visually consistent.
