# Executable workflow runner contract

## Current state

`workflow run` is advisory. It validates a workflow, chooses the first runnable step from `next_runnable_steps`, and returns either validation diagnostics, a `RunStep` payload, or blocked-step reasons. Worker-backed steps include a structured worker assignment contract, but the runner does not mutate workflow state, execute command checks, dispatch workers, or write results.

The existing verification runner executes command-style verification gates in runtime verification models, captures stdout/stderr/status artifacts, and marks gates passed/failed/blocked. Workflow checks are separate YAML `checks` entries with `kind`, `status`, `command`, and artifact metadata.

## Target behavior for this slice

`workflow run` should remain safe and bounded, but become executable for workflow-native steps that can be completed locally without hidden destructive actions.

### Step selection

- Validate the workflow first, using the selected validation mode.
- If validation fails, return `validation_blocked` and do not mutate the workflow.
- Select the first runnable step from `next_runnable_steps`.
- Respect dependencies exactly as today; do not run blocked steps.
- Treat terminal steps/checks as immutable unless the user explicitly uses `workflow update`.

### Executable verify steps

A `kind: verify` step is executable when every attached check is either:

- a `kind: command` check with a non-empty `command`; or
- already passed/skipped.

Execution flow:

1. Mark the step `active` and append an event.
2. Run pending command checks in listed order.
3. For each command check, capture stdout, stderr, exit code, duration, and status artifacts under the workflow directory.
4. Mark the check `passed` on exit code 0.
5. Mark the check `failed` on non-zero exit.
6. Mark the check `blocked` on timeout or command-spawn failure.
7. If all attached checks pass or are already skipped/passed, mark the step `done`.
8. If any check fails, mark the step `blocked` or `failed` and stop the workflow run without advancing later steps.

### Executable build steps

A `kind: build` step with a worker assignment should not silently spawn a background agent in this slice. Instead, `workflow run` may transition the step to `active`, render the worker assignment, append an event, and return an executable assignment payload for the host/agent to perform. This makes the state transition durable without pretending the worker completed.

A future runner can replace this handoff with real child-agent execution once the host has a safe worker spawning path.

### Checks and status updates

- Status writes should use the existing workflow YAML update path so events and schema behavior stay consistent.
- Event records should include action, path, value, reason, and timestamp.
- Automatic status updates must never mark acceptance or closeout checks complete without evidence.

### Policy boundaries

The runner must stop and return a blocked/user-decision action when:

- validation fails;
- the runnable step requires approval by workflow policy;
- a check is non-command and not already complete;
- a command check is missing its command;
- a command fails or times out;
- the step would require destructive git or filesystem behavior not represented as an explicit command check.

### Results and closeout

`workflow run` should not invent results. Closeout remains explicit: after checks pass and result artifacts exist, closeout steps can be marked only when required checks are passed.

## Initial implementation plan

1. Add workflow-check command execution helpers in `crates/imp-core/src/tools/workflow.rs` or a small workflow runner module.
2. Reuse the shell execution/capture pattern from `workflow/verification_runner.rs`, but write artifacts under `.imp/workflows/<id>/artifacts/checks/<check-id>/`.
3. Add `WorkflowNextAction` variants/details for executed verify results and active worker assignments.
4. Mutate workflow YAML for check and step statuses through a small typed helper, appending events with existing event format.
5. Add tests for:
   - command verify success marks check passed and step done;
   - command verify failure marks check failed and blocks step;
   - build step with worker marks step active and returns assignment;
   - dependency-gated steps are not run;
   - validation/approval blockers do not mutate state.
