# imp-native Workflow Engine

Status: design draft for imp workflow planning

## Summary

The north star is an imp-native workflow engine: a schema-first, programmatic orchestration layer built into imp. It can plan, enforce, run, update, verify, review, delegate bounded worker agents, assemble context, and block invalid closeout.

A workflow is a durable project artifact and executable contract, not just a prompt wrapper. The canonical artifact is `.imp/workflows/<slug>/workflow.yaml`. Generated runtime artifacts live beside it: `events.jsonl`, `trace.jsonl`, `state.json`, `results.md`, and `artifacts/`.

The current `/plan` direction should become workflow-backed. `/status` should render workflow state. `/run` should continue the active workflow. Existing imp concepts currently called workflow should be consolidated: prompt-wrapper workflow profiles and runtime workflow contracts become one workflow system.

## Product decisions

- Product surface: `imp workflows`, exposed to the model as a first-class `workflow tool` and to users mostly through `/plan`, `/status`, and `/run`.
- The `workflow` tool may eventually replace normal imp use of `mana`, `work`, and `prototype`.
- Workflow instances are compact YAML with map-based IDs.
- Global/project config and reusable profiles are TOML.
- Global config lives in `~/.imp/config.toml`; project config overrides global config.
- Project profiles live under `.imp/workflows/profiles/`.
- Multiple workflows may exist in one project; one workflow is active per imp session unless selected otherwise.
- Workflows can call other workflows as normal steps; child workflows should live as sibling directories under `.imp/workflows/` and are called subworkflows only relationally.
- Code-changing and prototype workflows may get branches/worktrees, controlled by settings and refined through prototypes.
- Parent agent owns orchestration and review, but should not directly write production code/tests by default.
- Parent agent may edit non-code artifacts such as `workflow.yaml`, `results.md`, design docs, and planning artifacts.
- Builder workers write production code and tests.
- Workers are reusable objects (`builder`, `reviewer`, etc.) and can be given workflow-backed assignments.
- Prototyping is a workflow feature, not a role: workflows can test approaches, discard prototypes, and record selected approach/rationale.
- Goal/acceptance changes require approval.
- Adding checks is allowed; removing or weakening required checks requires approval.
- Final answers are constrained by workflow state: workflow validates claims and renders the factual skeleton; the agent adds prose.

## Research survey

### GitHub Spec Kit

Spec Kit is the closest conceptual neighbor for structured software work. It uses explicit phase artifacts such as constitution, specify, plan, tasks, and implement. imp should take the durable phased-artifact idea, but enforce checks at runtime instead of treating the spec as advisory prose.

### LangGraph / Deep Agents

LangGraph contributes explicit state graphs, durable execution, interrupts, human-in-loop, memory, observability, and subagent/subgraph patterns. imp should take the state-machine and resumability lessons without requiring users to hand-program arbitrary graphs for normal coding tasks.

### CrewAI Flows and Crews

CrewAI separates deterministic Flows from autonomous Crews. imp should mirror that split: the workflow engine is deterministic control; bounded workers are autonomous agents for research, planning, building, verification, review, and integration.

### Microsoft Agent Framework / AutoGen

Microsoft Agent Framework and AutoGen contribute production orchestration patterns: sequential/concurrent/handoff workflows, checkpointing, streaming, human-in-loop, time travel, and observability. imp should adopt the layered architecture lesson: schema, workflow engine, runtime adapters, policy/tool layer, trace/results layer, worker orchestration, and UI surfaces.

### GitHub Agentic Workflows, Goose, and Gemini CLI

GitHub Agentic Workflows shows that repo behavior can be described above CI YAML. Goose and Gemini CLI reinforce local operator UX: checkpointing, resume, custom commands, status visibility, headless structured output, and extension/MCP integration. imp should compete by making workflows enforceable and inspectable, not just reusable prompts.

## Workflow tool

The model should interact with workflows through a native `workflow` tool. This is more important than a large slash-command surface because it lets the model programmatically create, inspect, run, update, and close workflows while the engine enforces schema and checks.

Minimal candidate actions:

```text
plan       create or revise a workflow from a goal
show       show status, next actions, blocked checks, workers, and results
update     mutate workflow artifacts directly within allowed permissions
run        return the next action and dispatch workers when allowed
close      attempt closeout and render final factual skeleton
```

Additional actions such as `list`, `select`, and `archive` can be added if the minimal surface becomes awkward with multiple workflows.

The tool owns workflow artifacts directly, similar to how mana mutates its work graph. It may write:

```text
.imp/workflows/**/workflow.yaml
.imp/workflows/**/events.jsonl
.imp/workflows/**/results.md
.imp/workflows/**/artifacts/**
```

It should not write production code/tests directly. Builder workers do that under workflow policy.

This tool could replace current agent-facing `mana`, `work`, and `prototype` responsibilities for imp-native work:

- `work` replacement: local project artifacts, status, events, results, and resume.
- `prototype` replacement: prototype experiments become workflow step items with structured results.
- `mana` replacement for everyday imp execution: bounded execution, verification, review, and closeout live in imp. Mana remains optional indexing/aggregation if needed.

User-facing commands stay small:

```text
/plan    calls workflow.plan
/status  calls workflow.show
/run     calls workflow.run
```

## Artifact layout

```text
.imp/workflows/
  <slug>/
    workflow.yaml
    events.jsonl
    trace.jsonl
    results.md
    state.json
    artifacts/
  profiles/
    bugfix.toml
    feature.toml
    refactor.toml
  archive/
    <slug>/
```

Use `results.md` instead of `evidence.md` because it can hold proof, verification summaries, prototype outcomes, review findings, concerns, and final closeout.

## Live state split

Use an event-sourced split:

- `workflow.yaml` is the canonical reviewable workflow contract. It contains spec, acceptance, steps, checks, dependencies, worker model, settings, coarse status, and checkpoint summary.
- `events.jsonl` is append-only execution/decision history: transitions, check results, worker dispatches, prototype outcomes, approvals, retries, and closeout attempts.
- `trace.jsonl` is optional low-level runtime/tool telemetry and is not committed by default unless config says so.
- `state.json` is an optional derived cache, never the source of truth.
- `results.md` is the human-facing proof/summary artifact generated from workflow state, events, traces, and selected artifacts.

Runtime rule for the current native tool: validate the prospective `workflow.yaml`, open/preflight `events.jsonl`, atomically replace `workflow.yaml`, then append the update event. This prevents common unaudited mutations where the event log cannot be opened, but it is not a full two-file transaction if the process crashes after YAML replacement and before the event write completes. A future journaled update path should make crash recovery stronger.

## Compact schema shape

Use map-based IDs to reduce repetition and make duplicate IDs impossible by structure.

```yaml
schema: imp.workflow/v1
id: add-workflows-to-imp
title: Add workflows to imp
status: active

spec:
  goal: Build imp-native workflow support.
  acceptance:
    workflow_tool_exists:
      text: workflow tool exists
      status: todo
      checks: [workflow_tool_tests]
    plan_is_workflow_backed:
      text: /plan creates or updates workflow.yaml
      status: todo

steps:
  prototype_schema:
    kind: workflow
    workflow: define-workflow-schema
    status: running
    checks: [schema_results_ready]

  build_workflow_tool:
    kind: build
    worker: builder
    depends_on: [prototype_schema]
    status: todo
    checks: [schema_selected]

prototypes:
  concise_yaml:
    question: Can concise map-based YAML be tasteful and enforceable?
    status: selected

checks:
  schema_selected:
    kind: approval
    question: Which schema style should v1 use?
    status: pending
  schema_results_ready:
    kind: artifact
    path: .imp/workflows/define-workflow-schema/results.md
    status: pending

workers:
  builder:
    role: builder
    writes: [code, tests]
    worktree: workflow
```

Acceptance criteria are objects so each criterion can carry its own verification state (`todo`, `done`, `blocked`, etc.) and links to checks.

## Dependencies and enforcement

Workflows support dependencies between steps, checks, worker runs, prototype experiments, and called workflows. The engine selects next runnable items by checking dependencies, required checks, write permissions, worktree requirements, and blockers.

The model can propose transitions, but the engine decides legality. Required verification checks block clean DONE unless they pass. Failed checks route automatically into diagnosis/retry until retry policy is exhausted. Closeout requires validated results and cannot claim unverified facts.

Use `checks` rather than `gates`; it is familiar, less bureaucratic, and maps well to CI/review/context/approval requirements. Reserve `guards` for hard policy enforcement if needed later.

## Steps and workflow calls

Use `steps` as the core unit rather than `phases`. A step can represent context, planning, prototyping, building, verification, review, a decision, closeout, worker dispatch, or a call to another workflow.

A workflow call is just a step:

```yaml
steps:
  define_schema:
    kind: workflow
    workflow: define-workflow-schema
    checks: [child_workflow_results_ready]
```

Child workflows should be stored as siblings:

```text
.imp/workflows/add-workflows-to-imp/workflow.yaml
.imp/workflows/define-workflow-schema/workflow.yaml
```

The child can link back to its parent:

```yaml
parent:
  workflow: add-workflows-to-imp
  step: define_schema
```

## Profiles

Project-level reusable profiles live under `.imp/workflows/profiles/<name>.toml`. Built-in step kinds for v1:

```text
spec
context
plan
prototype
build
verify
review
workflow
decision
closeout
```

Arbitrary custom step kinds should be allowed eventually, but v1 should let custom steps compose built-in check primitives rather than define arbitrary enforcement code.

## Worker model

V1 assumes one active parent agent owns the workflow. The parent coordinates, updates the workflow, dispatches worker agents, reviews results, and handles closeout. The parent should not directly write production code or tests by default.

Worker objects define reusable capabilities and constraints:

```yaml
workers:
  builder:
    role: builder
    writes: [code, tests]
    worktree: workflow
  reviewer:
    role: reviewer
    writes: []
```

Workers can receive full workflow-backed assignments rather than thin task slices. That should not be materially harder if the workflow runtime is designed well, and it gives workers the same structured context/checks/closeout as the parent.

## Prototyping as a workflow feature

A workflow can include prototype experiments when the right approach is uncertain. Prototype code is disposable by default. The workflow records question, hypothesis, experiment, result, recommendation, selected approach, and rationale. Prototype results feed decisions and plan updates.

## Relationship to mana/work/prototype

This direction redefines the tool and mana/imp split:

- imp owns bounded programmatic workflow execution;
- imp workflows are durable project artifacts when committed;
- the workflow tool can absorb normal imp-native `work` and `prototype` responsibilities;
- the workflow tool can absorb everyday imp execution responsibilities currently routed through `mana`;
- mana, if retained, owns optional indexing, aggregation, GUI/board views, and long-horizon project graph behavior;
- mana should not be required for normal imp workflow execution.

## Existing workflow terminology to revisit

Audit and consolidate:

- `crates/imp-core/src/workflow/*`: align useful contract/check/worktree/worker concepts with schema-first `workflow.yaml`.
- `crates/imp-core/src/workflow_profiles.rs`: convert prompt-wrapper profiles into workflow profiles/templates.
- TUI `/workflow` and `/workflows` commands: simplify around `/plan`, `/status`, `/run`.
- `WorkflowRunController`: evolve into workflow engine or retire mana/imp-work-specific assumptions.
- `ChildWorkflow*`: likely rename UX to worker runs or workflow calls depending on context.
- Existing `mana`, `work`, and `prototype` tool responsibilities: evaluate replacement by native workflow tool.

## Git tracking policy

The repo currently ignores `.imp/`, so workflow artifacts under `.imp/workflows` are local-only unless ignore rules change. To make selected workflows durable project artifacts, add negative ignore rules for workflow source files while keeping traces/cache local by default:

```gitignore
.imp/*
!.imp/workflows/
!.imp/workflows/**/
!.imp/workflows/**/workflow.yaml
!.imp/workflows/**/results.md
!.imp/workflows/profiles/**
.imp/workflows/**/trace.jsonl
.imp/workflows/**/state.json
.imp/workflows/**/artifacts/**
```

This should be implemented deliberately later; this design turn does not change ignore policy.

## Staged implementation plan

1. Create a workflow specifically for defining/refining the workflow schema.
2. Keep `.imp/workflows/prototype-imp-workflow-engine/workflow.yaml` as the first dogfood artifact.
3. Prototype schema validation.
4. Prototype Rust DTOs/state machine against that exact artifact.
5. Add check validation, dependency validation, and legal transition tests.
6. Add the native model-facing `workflow` tool.
7. Wire `/plan`, `/status`, and `/run` as wrappers around the workflow tool.
8. Add workflow status rendering in TUI/headless output.
9. Add worker-run dispatch for builder/verifier/reviewer.
10. Add branch/worktree creation settings.
11. Migrate existing workflow profile/slash-command system into workflow profiles.
12. Evaluate replacing existing `mana`, `work`, and `prototype` tools with workflow-tool actions.

## Test plan

- schema validation tests;
- invalid duplicate ids rejected or impossible by map structure;
- missing dependency references rejected;
- missing check references rejected;
- goal/acceptance mutation requires approval;
- acceptance criteria can carry verification status and linked checks;
- required check failure blocks DONE;
- parent cannot build/write code or tests when settings disallow it;
- parent can write allowed non-code artifacts;
- builder worker can write code and tests in allowed worktree;
- prototype experiment can be discarded while preserving result summary;
- workflow call step can track child workflow result;
- final summary cannot claim unverified facts;
- `workflow plan` creates/updates a valid workflow;
- `workflow show` lists multiple workflows and active workflow state;
- `workflow run` returns next action and can dispatch workers;
- `/plan` creates/updates a valid workflow through the tool;
- `/status` renders current step, blocked checks, active workers, and next action.

## Open questions

- What is the first minimal Rust schema validation implementation?
- Which existing workflow structs can be reused versus renamed or retired?
- How much of `mana`, `work`, and `prototype` should be retired once workflow-tool parity exists?
- What exact setting controls branch/worktree creation for prototype and build steps?
- Should `/workflow` exist as a public advanced command or remain hidden/debug-only?
