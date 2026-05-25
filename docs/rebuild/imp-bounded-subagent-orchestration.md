# imp Bounded Subagent Orchestration

Status: target design for mana unit 365.9.

This document defines how imp should autonomously orchestrate bounded work while keeping durable work orchestration outside the core runtime. It updates the earlier workflow direction: imp should not own the durable work graph, but it should be capable of coordinating helper agents inside a run.

## Boundary

imp owns **runtime orchestration inside a run**.

mana or another harness owns **durable work orchestration across runs**.

A simple rule:

- If child work is bounded by the current run and returns results to the parent, it belongs in imp.
- If child work must survive the parent run, appear on a board, be scheduled independently, hold a lease, or own close/fail/review lifecycle, it belongs in mana or another harness.

This means imp can be an autonomous runtime conductor without becoming the system of record for projects.

## Bounded subagents

A bounded subagent is a child execution created by a parent imp run to answer, inspect, implement, verify, review, or synthesize a limited piece of work.

A bounded subagent:

- has a parent run;
- has a role and objective;
- receives scoped context and tool permissions;
- emits runtime events;
- returns a structured outcome to the parent;
- is cancelled when the parent run is cancelled unless explicitly detached by a host;
- does not own durable task lifecycle by default.

Examples:

- code-search helper for a large repository question;
- verifier agent that runs checks and summarizes failures;
- reviewer agent that critiques a diff;
- implementation helper limited to a path set;
- planning helper that decomposes a local objective;
- documentation helper that audits public API changes.

## Durable workers

A durable worker is different from a bounded subagent. Durable workers are created and tracked by mana or another host.

A durable worker:

- may outlive the parent run;
- may be visible on a board or run monitor;
- may claim files or tasks;
- may be retried by a scheduler;
- has durable state, logs, artifacts, and review lifecycle;
- maps to work graph concepts such as task, run, lease, close, fail, reopen, or review.

imp can execute durable worker assignments, but the durable lifecycle belongs to the host.

## Workflows as runtime recipes

imp workflows should behave like Goose recipes: optional runtime recipes that guide what imp can do, not a mandatory center of the runtime.

A runtime recipe may define:

- role prompts;
- when to spawn bounded subagents;
- context partitioning rules;
- tool sets and permissions;
- verification expectations;
- evidence requirements;
- merge/synthesis rules;
- retry/escalation conditions;
- output shape.

A runtime recipe must not own:

- durable task IDs;
- project streams;
- board status;
- scheduling/leases;
- close/fail/reopen lifecycle;
- long-running cross-session worker state.

Those belong to mana or another harness.

## Subagent execution packet

A parent imp run should create a bounded subagent from an explicit packet:

- `parent_run_id`: runtime-local parent identifier;
- `child_run_id`: runtime-local child identifier;
- `role`: verifier, reviewer, searcher, implementer, planner, synthesizer, or custom;
- `objective`: concise task for the child;
- `context`: scoped files, snippets, messages, docs, memory, or artifacts;
- `workspace`: cwd/worktree/path bounds;
- `tools`: enabled tools and permissions;
- `policy`: write/sandbox/approval/model limits;
- `output_contract`: expected outcome fields or schema;
- `resource_limits`: timeout, token budget, tool-call budget, concurrency group;
- `merge_policy`: how the parent should consume the result.

The packet should be expressible through the same durable runtime API shape as prompt/session/task/run execution, but it is scoped to the parent runtime unless a host promotes it.

## Runtime events

Bounded subagents should emit normalized runtime events so CLI, TUI, RPC, and a host GUI can observe them.

Minimum events:

- `subagent_started`;
- `subagent_message` or message deltas;
- `subagent_tool_started`;
- `subagent_tool_completed`;
- `subagent_artifact`;
- `subagent_blocked`;
- `subagent_cancelled`;
- `subagent_completed`;
- `subagent_failed`;
- `subagent_merged`.

These are runtime events, not durable work graph transitions. A host may persist or translate them into durable run/step records, but imp should not require that.

## Outcomes and merge behavior

Every bounded subagent should return a structured outcome:

- status: success, incomplete, blocked, failed, cancelled;
- summary;
- evidence/artifacts;
- files changed or inspected;
- verification results;
- blockers/questions;
- follow-ups;
- usage/cost/diagnostics;
- confidence or risk labels where useful.

The parent run decides how to merge the outcome according to the recipe or default policy.

Merge policies:

- `inform`: parent uses the result as context only;
- `verify`: parent treats result as verification evidence;
- `review`: parent treats result as critique that may require changes;
- `apply`: parent applies child-produced edits or artifacts;
- `synthesize`: parent combines multiple child outcomes;
- `escalate`: parent asks the user or host for a decision.

## Cancellation, retry, and resource limits

Bounded subagents must be constrained.

Defaults:

- parent cancellation cancels children;
- child timeouts produce incomplete/failed outcomes, not silent drops;
- retries are bounded and recorded as events;
- child writes obey parent policy unless narrowed further;
- child context should be smaller than parent context unless explicitly allowed;
- parent is responsible for final synthesis and user-facing result.

Resource limits should include:

- max wall time;
- max model tokens;
- max tool calls;
- max parallel children;
- path/write bounds;
- model/provider budget if available.

## Host promotion

A host such as mana may promote a bounded subagent into durable work when the work becomes too large or needs independent lifecycle.

Promotion should be explicit. The parent imp run may request promotion, but mana owns the durable record after promotion.

Promotion inputs:

- child objective;
- current context/evidence;
- suggested acceptance criteria;
- path locks or workspace needs;
- reason for durable promotion;
- parent run reference.

After promotion, imp may continue as a worker assigned by mana, but the work graph is no longer imp-owned.

## Relationship to current workflow code

The current `WorkflowRuntimeLayer` should evolve toward two concerns:

1. **Recipe/runtime orchestration support** that belongs in imp:
   - runtime contracts;
   - verification/evidence expectations;
   - subagent role/merge policies;
   - bounded child execution;
   - runtime events and outcomes.

2. **Mana/work-graph compatibility** that should move out or become explicit host integration:
   - mana root bootstrap;
   - child mana unit creation;
   - mana graph closeout;
   - imp-work run-state nudges;
   - durable task lifecycle.

## Next implementation slices

1. Split `WorkflowRuntimeLayer` internally into recipe/runtime support and mana-work-graph compatibility modules without behavior changes.
2. Introduce runtime DTOs for bounded subagent input, events, outcome, merge policy, and resource limits.
3. Add a no-op/default recipe layer for standalone imp runs and make mana/work-graph compatibility explicitly enabled by host/builder configuration.

## Done criteria for this model

This model is working when:

- standalone imp can spawn bounded helper agents inside a run;
- bounded subagents are visible through runtime events;
- parent runs receive structured child outcomes;
- recipes can define subagent roles and merge behavior;
- durable work graph promotion is explicit and host-owned;
- imp remains useful without mana or imp-work enabled.
