# imp Workflow Feature Inventory for 0.3

Status: decision inventory for mana task 365.9.

This inventory reconciles the current 0.3 direction with existing imp/mana/work/prototype surfaces. The working direction comes from `.imp/workflows`, especially:

- `.imp/workflows/prototype-imp-workflow-engine/workflow.yaml`
- `.imp/workflows/prototype-imp-workflow-engine/results.md`
- `.imp/workflows/define-workflow-schema/artifacts/schema-reference.md`
- `.imp/workflows/define-workflow-schema/artifacts/validation-rules.md`
- `.imp/workflows/define-workflow-schema/artifacts/rust-parser-plan.md`
- `.imp/workflows/prototype-rust-workflow-schema-parser/workflow.yaml`

The important direction change: imp-native workflows are the intended primary orchestration capability for imp 0.3. They may replace normal imp use of mana, work, and prototype once workflow parity exists. Older mana-first docs and tasks are historical context, not normative 0.3 product direction unless explicitly revived.

## Disposition vocabulary

- **Keep**: remains a core imp runtime capability.
- **Fold into workflows**: keep the capability, but make workflow state/checks/results the source of truth.
- **Compatibility-only**: keep temporarily for old users/tests/migration, hidden or optional where possible.
- **Remove**: safe to delete from the default runtime surface now or soon.
- **Defer**: do not build or delete yet; revisit after workflow parity.

## Inventory

| Feature/surface | Current role | 0.3 disposition | Rationale | Removal/parity condition |
|---|---|---|---|---|
| Prototype tool | Bounded disposable experiments with evidence/learnings/followups | Fold into workflows; then compatibility-only; later remove standalone model-facing tool | Workflow schema already has top-level `prototypes` and `kind: prototype` steps. Prototypes are stronger when tied to workflow context, checks, results, and closeout. | Blocked on workflow prototype executor that creates sandboxes, runs commands, captures evidence, records followups, and can promote/discard artifacts. |
| imp-work / work tool | Durable tasks, epics, memory, decisions, context packs, runs, checks, handoff | Remove from default imp; keep archived/compatibility-only outside default runtime | Default imp 0.2.6 already removed imp-work from `imp-cli` dependency tree. Workflow artifacts should replace normal imp use of work/task state. | Safe to keep stripped from default. Any reintroduction must be explicit migration/import only. |
| `imp run` / WorkRun path | Native WorkRun planning/execution CLI | Remove from default; do not revive except as migration/import | Workflow `steps`, `checks`, `workers`, and `results.md` are the new execution model. | Already removed from default; avoid adding new WorkRun-specific surfaces. |
| mana integration | Optional mana command/tool/UI integration | Compatibility-only / optional adapter | 0.3 should not depend on mana for normal execution. mana may remain useful for old graphs or external experiments. | Keep behind `mana-ui` / `mana-tool`; default dependency tree must stay free of `mana-core` and `mana-cli`. |
| mana-first 365 child specs | Prior target architecture around mana harness | Defer/supersede for 0.3 | The active workflow artifacts contradict mana-first acceptance. Continuing those specs would create stale product direction. | Create a superseding workflow epic or rewrite 365 before doing more mana-harness spec work. |
| Runbooks | Executable plans previously discussed as mana feature | Fold into workflows | A workflow is already an executable plan with steps, checks, workers, context, and closeout. A separate runbook concept adds naming/abstraction debt. | Use “workflow template/profile” if reusable runbook-like behavior is needed. |
| Freeform planning notes | Human/agent scratch planning | Fold into workflows as non-authoritative notes | Plans should be structured in `workflow.yaml`, while results/decisions live in `results.md` or structured workflow fields. | Safe to keep as comments/notes; final source of truth should be workflow state. |
| Checks / verification | Scattered task verify commands, CI commands, manual checks | Fold into workflows | `.imp/workflows` already defines `checks` as requirement/verification primitive. | Workflow runner must support command/artifact/aggregate/manual checks and closeout enforcement. |
| Context packs | Prepared context for tasks/workers/prototypes | Fold into workflows | Context belongs near the workflow step that needs it. Workflows should define required files, symbols, searches, freshness, and worker-specific bundles. | Blocked on workflow context schema/runtime support. Do not create a separate durable context-pack store for 0.3. |
| Final claim validation / closeout | Model response discipline and DONE/BLOCKED claims | Fold into workflows | Schema reference states final answer should be constrained by workflow state and validated results. | Blocked on closeout step/runtime enforcing required checks, artifacts, and status transitions. |
| Bounded subagents | Runtime-local worker orchestration | Keep and implement as workflow execution primitive | Workflows need real workers for build/review/prototype/check steps. Subagents are runtime execution, not a separate durable work graph. | Required for 0.3 if workflows promise orchestration beyond serial shell steps. |
| Recipes | Reusable runtime behavior/prompt/tool patterns | Keep; may create/execute workflows | Recipe is behavior/template; workflow is durable executable state. Do not collapse them completely. | Clarify README/API distinction. |
| Skills | Durable knowledge/instructions | Keep | Skills inform agents and workflow authoring but are not workflow state. | No removal. Consider skill references in workflow context later. |
| Lua extensions | Current shipped extension runtime | Keep | Lua remains shipped extensibility. Workflow custom steps may eventually call extension tools, but workflow schema should remain Rust-validated YAML. | No removal. Avoid claiming TypeScript extensions are shipped. |
| TUI workflow surfaces | UI for chat/tool activity today; no workflow-native display yet | Keep TUI; fold new workflow state into it | TUI should show active workflow, runnable steps, blockers, checks, subagent activity, results, and closeout status. | Blocked on workflow runtime/read model. |
| Runtime events | Agent/tool/session event stream | Keep | Workflow execution should emit/consume runtime events for observability. | No removal. Add workflow/subagent events. |
| Provider/model abstraction | Core model/provider runtime | Keep | Workflows orchestrate model use; they do not replace providers. | No removal. |
| Tool registry/policy | Native tool surface and safety checks | Keep | Workflow steps must execute through normal tool policy. | No removal. |
| Local read/edit/bash/git/scan/web tools | Core coding tools | Keep | Workflows orchestrate these tools via agent/subagent execution. | No removal. |
| Session handling | Chat/session persistence | Keep | Workflows may link to sessions, but normal chat remains core. | No removal. |
| `ask_user` / human decisions | Interactive decision points | Fold into workflows | Workflow steps/checks should be able to block on human decision and resume. | Blocked on workflow status model for blocked/needs_context/manual decision. |

## Safe near-term removals or tightening

These are safe to strip or keep stripped before full workflow parity:

1. Keep `imp-work` out of the default workspace/dependency tree.
2. Keep `work` tool absent from the default model-facing tool surface.
3. Keep legacy `imp run` / WorkRun command absent from the default CLI.
4. Keep mana command/tool/UI integration optional behind `mana-ui` / `mana-tool`.
5. Remove README claims that imp-work is the active durable work system.
6. Remove README/tool docs that advertise `prototype` as the long-term standalone primitive; describe it as moving into workflows.
7. Avoid implementing new mana-first harness docs under 365 until the epic is reconciled with the workflow direction.
8. Avoid adding a separate runbook abstraction; use workflows and workflow templates/profiles.

## Removals blocked on workflow parity

Do not fully remove these until workflows can replace their useful behavior:

1. Standalone `prototype` tool: blocked on workflow prototype executor and evidence/result recording.
2. Any remaining prototype recording DTOs: blocked on workflow results/artifacts support.
3. Any human closeout/final-claim discipline currently implemented outside workflows: blocked on workflow closeout enforcement.
4. Any context-pack-like behavior still useful for accuracy: blocked on workflow context assembly and freshness checks.
5. Manual verification guidance: blocked on workflow check runner and required-check enforcement.
6. Mana compatibility code that users still depend on: keep optional until migration/import story exists.

## Workflow parity requirements

Minimum parity before workflows can replace normal use of mana/work/prototype:

1. Rust parser and validator for `imp.workflow/v1`.
2. Workflow status/read model: active, blocked, failed, done, cancelled, and per-step/check status.
3. Command/artifact/aggregate/manual check execution.
4. Prototype step executor with sandbox, evidence, learnings, followups, and promote/discard semantics.
5. Bounded subagent execution for worker-backed steps.
6. Context section support: required files/artifacts, generated summaries, and freshness validation.
7. Closeout enforcement: no final DONE unless required checks and acceptance are satisfied or concerns are explicit.
8. Events/results: append-only events plus rendered `results.md`.
9. CLI surface: minimal commands such as create/validate/status/run/next/close.
10. README and help text updated to present workflows as the durable orchestration path.

## Recommended follow-up tasks

1. **Implement Rust workflow schema parser/validator**
   - Source: `.imp/workflows/prototype-rust-workflow-schema-parser/workflow.yaml`.
   - Verify against current dogfood fixtures.

2. **Update README for 0.3 workflow direction**
   - Remove stale imp-work-as-active-system language.
   - Describe workflows, bounded subagents, prototype steps, checks, and optional mana compatibility.

3. **Implement workflow check runner**
   - Start with command, artifact, aggregate, and manual check kinds.
   - Enforce required checks before closeout.

4. **Implement workflow prototype capability**
   - Replace standalone prototype behavior with workflow-backed prototype steps.

5. **Implement real bounded subagent execution**
   - Wire subagent coordinator to actual worker sessions, not just no-op transitions.

6. **Add workflow CLI surface**
   - Keep it small: validate, status, next, run, close.
   - Avoid recreating a large task database command suite.

7. **Add TUI workflow read model**
   - Show active workflow, runnable steps, blockers, checks, worker/subagent activity, and closeout state.

8. **Reconcile mana epic 365**
   - Either supersede old mana-first children or rewrite the epic into workflow-first 0.3 planning.

9. **Create compatibility/migration notes**
   - Explain how old prototype/work/mana users should map behavior to workflows.

## 0.3 release gate

Before bumping to 0.3, imp should have:

- default standalone dependency tree still clean of `imp-work`, `mana-core`, and `mana-cli`;
- workflow parser/validator;
- workflow check runner;
- prototype-as-workflow capability or an explicit compatibility-only bridge;
- real bounded subagent execution or clear experimental marking;
- README aligned with workflows;
- changelog entries for defunct/compatibility-only surfaces;
- `cargo fmt --all --check`, default checks/tests, and workflow fixture validation passing.
