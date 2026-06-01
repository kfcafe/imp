# Benchmark workflow end-to-end

Status: done_with_concerns.

## Commands and tool actions run

- `workflow run benchmark-workflow-e2e` selected expected steps through context, plan, fixture preparation, validation, run/update exercise, negative case, and closeout gate verification.
- `workflow validate benchmark-workflow-e2e` passed after fixture preparation, after negative-case/gate probing, and after final acceptance updates.
- `workflow update ...` was used for all workflow state mutations.
- Invalid update probe: attempted to set `steps.exercise_negative_case.status = not_a_status`; the workflow tool rejected it with schema validation failure.

## State transitions observed

- `context` -> `done`
- `plan_benchmark` -> `done`
- `prepare_fixture` -> `done`
- `validate_initial_state` -> `done`
- `exercise_run_and_update` -> `done`
- `exercise_negative_case` -> `done`
- `verify_closeout_gate` -> `done`
- `review` -> `done_with_concerns`
- `closeout` -> `done_with_concerns`
- workflow status -> `done_with_concerns`

## Acceptance result

- Acceptance criteria: 6/6 done.
- Final validation: passed.

## Events appended

- Event log: `.imp/workflows/benchmark-workflow-e2e/events.jsonl`
- Event count at final results refresh: 33
- Tail inspection showed append-only records with path, value, reason, action, and timestamp.

## Fixture result

- Fixture path: `.imp/workflows/benchmark-workflow-e2e/artifacts/fixture.txt`
- Final fixture content: `complete`

## Negative case result

The invalid status update failed before persistence. A SHA-256 check of `workflow.yaml` before and after the invalid update confirmed the file was unchanged.

## Closeout gate result

A closeout gate probe temporarily set `steps.closeout.status = done`, then `workflow run benchmark-workflow-e2e` still selected `verify_closeout_gate` because required verify/review work was incomplete. Closeout was reset to `todo`, proving the run engine did not treat early closeout as sufficient while required lifecycle work remained.

## Concerns and follow-up work

- The workflow tool permits direct status mutation of closeout itself; the run engine still recovered by selecting the incomplete prerequisite step. A future hardening pass could reject premature closeout mutation directly in `workflow update`.
- API work can now begin, but should account for this closeout-mutation hardening concern.
