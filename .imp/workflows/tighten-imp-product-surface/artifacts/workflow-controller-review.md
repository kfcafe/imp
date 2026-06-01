# Workflow Controller Review

Status: draft first-principles review for `tighten-imp-product-surface`.

## Question

What should happen to the current ambient workflow controller if imp's tight product is:

```text
normal TUI/chat = natural, model-trusting coding agent
explicit workflow run = durable, rigid, automatable orchestration
```

## Current controller in one sentence

`WorkflowRunController` is an ambient runtime policy object that watches agent turns/tool results and can force follow-up turns or downgrade closeout status when it thinks durable workflow obligations are incomplete.

## Evidence inspected

Key files:

- `crates/imp-core/src/workflow/controller.rs`
- `crates/imp-core/src/agent/workflow_integration/mana_compat.rs`
- `crates/imp-core/src/agent/run_loop.rs`
- `crates/imp-core/src/agent/events.rs`
- `crates/imp-core/src/runtime.rs`
- `crates/imp-cli/src/lib.rs` RPC/headless serialization of workflow controller snapshots
- `crates/imp-tui/src/app.rs` workflow controller snapshot event label

Current controller state includes:

- `workflow_id`
- `mana_root_id`
- `active_unit_id`
- `child_runs`
- `graph_closeout_required`
- `direct_closeout_required`
- budget/counters
- bootstrap state
- graph shape
- planning state
- closeout state

Current decision points:

- bootstrap required -> inject workflow bootstrap prompt
- budget exhausted -> stop blocked
- explicit closeout blocker -> stop blocked
- child runs running/unknown -> inject supervision prompt
- awaiting decomposition -> inject decomposition prompt
- graph closeout required -> inject graph closeout prompt
- direct closeout required -> inject direct closeout prompt
- closeout not ready -> inject closeout prompt
- otherwise stop done

## What it would be like if it did not exist

If ambient controller disappeared from normal TUI/chat:

Positive effects:

- normal conversation becomes more natural
- no hidden workflow continuation prompts
- no workflow/mana closeout status overriding ordinary replies
- no mana root/unit vocabulary in normal runtime
- less behavioral rigidity from global run loop
- simpler mental model: workflows move when the model/user explicitly uses workflow actions

Lost behavior:

- ambient bootstrap/decomposition/supervision nudges disappear
- direct edit/write changes no longer automatically mark workflow closeout incomplete
- tool results no longer auto-convert mana/work actions into controller obligations
- final status no longer gets downgraded for incomplete workflow closeout
- workflow controller snapshot events/artifacts disappear or become workflow-run-specific

Assessment:

The lost behavior is valuable for explicit autonomous workflow execution, but harmful as ambient chat policy.

## Why current controller feels wrong for target product

### It is mana-shaped

Fields and prompts are still centered on mana:

- `mana_root_id`
- `active_unit_id`
- `bind_mana_root`
- `record_mana_graph_changed`
- `record_mana_orchestration_started`
- `durable workflow bootstrap requires a bound mana root`
- prompt text references mana graph/run_state/logs

This conflicts with target direction: workflow is the durable primitive; mana is compatibility/history.

### It is ambient

The controller participates in every agent run via:

- `begin_workflow_turn()` before turns
- `workflow_controller_continue_decision()` after model/tool decisions
- `enforce_workflow_closeout_status()` at final status
- persisted snapshots in run artifacts

This makes workflow strictness leak into normal chat/TUI behavior.

### It injects hidden user prompts

Continuation happens by pushing hidden follow-up prompts into the message list. Even with improved wording, this creates a model-facing behavioral leash.

### It couples workflow policy to final chat status

`enforce_closeout_status()` can convert a proposed done/done-with-concerns into blocked or concern-bearing statuses based on controller state.

This is useful in batch automation, but it makes normal chat feel like a workflow engine.

## First-principles target

Strict workflow behavior should live in an explicit workflow runner, not global chat.

Target split:

```text
Normal TUI/chat:
  minimal prompt
  no ambient workflow controller
  model can use workflow tool when helpful
  TUI can display workflow state gently

Explicit workflow run:
  runtime loads workflow
  validates workflow
  selects next runnable step/check
  builds a step-scoped prompt
  runs one worker/step
  records artifacts/status/checks
  enforces closeout for that run
```

## Proposed replacement: workflow runner/action

`imp workflow run <id>` and the native workflow run action should become the boundary for rigid orchestration.

Runner responsibilities:

1. load `.imp/workflows/<id>/workflow.yaml`
2. validate schema
3. choose next runnable step/check
4. create a scoped worker prompt
5. execute one step or return an assignment when execution service is not ready
6. record status/events/artifacts
7. run checks/verification if declared
8. return concise result and next action

Strictness should be local to that run.

## Subagent path

For long-running independent workflows and cron jobs, subagents should be children of the workflow runner.

Future runner behavior:

1. select runnable worker assignment
2. spawn scoped agent/subagent
3. pass workflow id, step id, objective, writable scope, checks, result path
4. worker writes result/artifacts
5. parent runner validates/checks and updates workflow

This preserves the dream of multi-hour autonomous coding without making normal chat rigid.

## Defunct strategy

Do not delete immediately without a replacement seam. Defunct in stages:

### Stage 1 — stop growing it

- no new ambient controller features
- no new mana-root controller features
- keep current tests passing while planning runner replacement

### Stage 2 — isolate ambient controller behind an execution mode

- disable ambient controller for normal TUI/chat
- keep it only for explicit workflow/headless worker runs if still needed
- make controller snapshot events optional/internal

### Stage 3 — move strict behavior into workflow runner

- extract next-action/run-selection service from `tools/workflow.rs`
- add `imp workflow run <id>`
- step-scoped prompts replace ambient follow-up prompts
- runner records workflow results/checks directly

### Stage 4 — remove mana-shaped controller

- remove `mana_root_id`/`active_unit_id` from workflow controller state
- remove `mana_compat.rs` controller obligations or archive compatibility
- remove run artifact `workflow_controller.json` if superseded by workflow events/results

## What should remain

Keep concepts, not the current ambient implementation:

- budgets for explicit workflow runs
- child worker tracking for explicit workflow runs
- blockers
- step/check closeout
- run evidence
- runtime events/state snapshots

Remove or relocate:

- ambient hidden follow-up prompts
- mana root/unit bootstrap
- closeout status override for normal chat
- controller as always-on agent turn policy

## Implications for system prompt

The global prompt should not carry workflow doctrine.

Target global prompt stays one line:

```text
You are imp, a practical and helpful software engineer. Use available tools, skills, and workflows when they help. Read files before editing them.
```

Workflow-specific instructions should be generated only by the workflow runner for a concrete step.

## Open questions

1. Should the first defuncting step merely disable ambient controller in TUI, or also in one-shot?
2. Should RPC expose workflow controller snapshots after the controller is no longer ambient?
3. Should existing run artifacts with `workflow_controller.json` be ignored, migrated, or left as historical files?
4. Should explicit `imp workflow run <id>` advance exactly one step per invocation, or may it loop internally once runner is real?

Current recommendation:

- disable ambient controller for normal TUI and one-shot.
- reserve strict behavior for explicit workflow runner/subagent execution.
- leave old artifacts historical; no migration needed.
- first `imp workflow run <id>` advances one meaningful step per invocation.
