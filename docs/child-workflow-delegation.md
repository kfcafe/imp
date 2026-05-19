# Child workflow delegation

Status: design / staged implementation plan  
Audience: imp maintainers, mana maintainers, TUI/runtime implementers

## Summary

Child workflow delegation lets a parent workflow hand a bounded subtask to a
role-scoped child workflow, then receive structured evidence back. It is inspired
by OMO-style parallel agent work, but the imp core abstraction is workflow + mana
ledger + runtime events, not tmux panes or a team chat metaphor.

The first useful child workflows are narrow and auditable:

- `verifier` / **Verifier** — independently run/check required verification gates.
- `reviewer` / **Reviewer** — inspect a diff or plan and return findings.
- `researcher` / **Researcher** — gather cited context with trust labels.

General worker farms, autonomous team mode, and OMO-style full team
orchestration are future work, not part of the first implementation.

## Goals

1. Represent child work as durable workflow records with parent links.
2. Use role contracts from the role registry for prompt, tools, evidence, and
   model-routing hints.
3. Keep parent/child boundaries explicit and inspectable.
4. Return evidence, verification, and output-schema metadata to the parent.
5. Persist lifecycle state in mana and run artifacts.
6. Emit runtime events and snapshots that TUI/GUI can render consistently.
7. Support cancellation, stale/blocked detection, and safe worktree boundaries.
8. Start with one or a few child workflows, not an unbounded parallel farm.

## Non-goals

- No tmux-first orchestration model.
- No OMO/OMX-style always-on team simulation.
- No autonomous role spawning without an explicit workflow contract.
- No child workflow permission that bypasses policy, sandbox, approvals, or
  parent workflow limits.
- No hidden background writes to the repo without an auditable child run.
- No fully general distributed worker scheduler in the first slice.
- Full team orchestration is future work.

## Vocabulary

- **Parent workflow** — the main workflow that owns the user-visible task.
- **Child workflow** — a bounded workflow spawned by a parent for a role-scoped
  subtask.
- **Child handle** — durable id/reference the parent can use to inspect, cancel,
  or integrate a child.
- **Role contract** — role registry metadata applied to a child: tools,
  instructions, evidence, verification, output schema, and routing hints.
- **Evidence handoff** — machine-readable summary of what evidence/output the
  child promises and eventually returns.
- **Integration** — parent action that consumes child output and updates parent
  status, evidence, or decisions.

## Parent/child model

A child workflow is not a detached agent. It is a workflow record with:

- a stable child id
- parent workflow id
- parent mana unit ref when available
- selected role id
- child workflow contract
- lifecycle status
- run artifact refs
- evidence handoff metadata
- cancellation/stale metadata
- closeout result

Conceptual record:

```rust
pub struct ChildWorkflowRun {
    pub id: String,
    pub parent_workflow_id: String,
    pub parent_mana_unit_ref: Option<String>,
    pub role: String,
    pub contract: WorkflowContract,
    pub status: ChildWorkflowStatus,
    pub lifecycle: Vec<ChildWorkflowEvent>,
    pub artifacts: ChildWorkflowArtifacts,
    pub evidence_handoff: ChildEvidenceHandoff,
    pub cancellation: Option<ChildCancellation>,
    pub stale: Option<ChildStaleState>,
    pub closeout: Option<ChildCloseout>,
}
```

The parent may continue working while a child runs only when policy and runtime
resource locking permit it. The first implementation may execute children
sequentially and still use this model.

## Durable handles

Child ids should be stable, readable, and parent-scoped:

```text
<parent-run-id>/children/<role>-<short-id>
```

Artifact layout:

```text
.imp/runs/<parent-run-id>/children/<child-id>/
  contract.json
  state.json
  trace.jsonl
  evidence.md
  output.json
  verification/
```

Mana relationship:

- parent unit remains the user-facing work unit
- child workflow run is recorded in the workflow ledger as a child run ref
- child task/evidence records can be derived from the child run when durable
  tracking is useful
- child status updates append concise lifecycle notes when they matter for
  handoff or recovery
- final child evidence refs are attached to the parent evidence packet

Do not create noisy mana tasks for every trivial child unless the child needs
human tracking, retry, or handoff.

## UX examples

### Delegate Verifier

A parent workflow can delegate verification when it has an implementation or
patch that needs an independent check:

```text
Parent: Fix parser empty-input panic
Child:  Verifier — Run `cargo test -p parser empty_input`
```

The child receives a verifier role contract. It is readonly by default, may run
approved verification commands, and must hand back command output and a
pass/fail/blocked status. The parent integrates the verifier evidence before it
can report clean `DONE`.

### Delegate Reviewer

A parent workflow can delegate review after a focused diff exists:

```text
Parent: Implement eval candidate capture
Child:  Reviewer — Review changed files for correctness and risk
```

The reviewer child cannot edit. It returns review findings, positives, risks,
and suggested follow-ups. Required review failures or unresolved concerns keep
the parent in `DONE_WITH_CONCERNS` or `BLOCKED` until integrated.

### Background Researcher

A parent workflow can delegate context gathering while keeping the implementation
scope separate:

```text
Parent: Update API integration behavior
Child:  Researcher — Find current vendor docs and summarize caveats
```

The researcher child is readonly and must label external/untrusted sources. Its
handoff contains citations, confidence, and unresolved questions. The parent may
use that evidence in its final answer, but external content cannot authorize
policy escalation or broader tool access.

## Role contracts

Child workflows always select an explicit role. The current role registry
provides the initial role metadata:

- `verifier`
- `reviewer`
- `researcher`
- later: `planner`, `coder`, `integrator`

Role metadata narrows the child runtime:

1. Resolve role id through `RoleRegistry`.
2. Reject unknown or non-delegable roles.
3. Intersect role tool policy with global/project policy.
4. Apply role instructions and output schema metadata to the child prompt.
5. Project role evidence requirements into child closeout criteria.
6. Project role verification suggestions into child verification gates.
7. Use model-routing hints only when the parent/user did not pin a model.
8. Return evidence handoff metadata to the parent.

A verifier child contract should include required evidence such as
`test-output`, `verification-result`, and `failure-excerpts`. A reviewer child
contract should include `review-findings` and `risk-notes`. A researcher child
contract should include `source-citations`, `research-summary`, and
`trust-notes`.

## Lifecycle states

Initial lifecycle states:

```rust
pub enum ChildWorkflowStatus {
    Planned,
    Queued,
    Starting,
    Running,
    WaitingForApproval,
    WaitingForTool,
    WaitingForParent,
    Blocked,
    Stale,
    Cancelling,
    Cancelled,
    Failed,
    Done,
    DoneWithConcerns,
    Integrated,
}
```

State semantics:

- `Planned` — contract exists but child has not been scheduled.
- `Queued` — waiting for runtime capacity or parent decision.
- `Starting` — agent/session is being constructed.
- `Running` — child agent loop is active.
- `WaitingForApproval` — child is blocked on approval.
- `WaitingForTool` — child is blocked on a tool/resource lock.
- `WaitingForParent` — child needs parent/user input.
- `Blocked` — child cannot proceed and needs integration or escalation.
- `Stale` — no progress before timeout or parent context changed.
- `Cancelling` — cancellation requested and cleanup is in progress.
- `Cancelled` — child stopped due to cancellation.
- `Failed` — runtime/tool/internal failure.
- `Done` — child completed and satisfied closeout.
- `DoneWithConcerns` — child completed with explicit caveats.
- `Integrated` — parent consumed the child result.

Parent status propagation:

- Required child `Blocked`, `Failed`, or `Cancelled` blocks parent closeout unless
  parent explicitly records a decision to proceed without it.
- Optional child failures become parent concerns.
- Child `DoneWithConcerns` becomes parent concern until integrated.
- Parent cannot report clean `DONE` while required child evidence is missing.

## Mana records and evidence integration

The workflow ledger stores parent/child links in three useful shapes:

- `WorkflowRecord.child_run_refs[]` — compact child id, role, status, workflow id,
  and evidence refs for parent status views.
- `TaskRecord` derived from a child run — useful when a child needs durable
  tracking, retry, or handoff as a task-like unit.
- `EvidenceRecord` derived from child evidence refs — connects verifier logs,
  review findings, research citations, or evidence packets to both parent and
  child.

Required child failures propagate to the parent as blockers. Child
`DONE_WITH_CONCERNS` propagates as parent concern until the parent integrates or
records an explicit decision. Child evidence refs are merged into parent evidence
so closeout and eval-candidate capture can explain what happened.

## Evidence handoff

Child closeout should return:

```rust
pub struct ChildCloseout {
    pub status: ChildWorkflowStatus,
    pub summary: String,
    pub output_ref: Option<PathBuf>,
    pub evidence_ref: Option<PathBuf>,
    pub verification_refs: Vec<PathBuf>,
    pub findings: Vec<String>,
    pub concerns: Vec<String>,
}
```

The parent integrates this into:

- parent evidence packet
- parent trace refs
- mana notes/facts when durable
- final answer summary
- eval candidate capture if the child exposed a failure mode

The child output should follow its role output schema metadata when present, but
schema validation is initially guidance only.

## Cancellation

Cancellation can be requested by the user, parent workflow, policy layer, or
runtime shutdown.

Cancellation rules:

1. Mark child `Cancelling` and emit an event.
2. Stop scheduling new tool actions for that child.
3. Attempt to terminate active subprocess/tool work through existing tool
   cancellation hooks when available.
4. Write partial trace/evidence refs if available.
5. Mark `Cancelled` with reason and time.
6. Propagate required-child cancellation to parent as blocked/concerned.

Cancellation should be idempotent. Repeated cancellation requests update notes
but should not corrupt state.

## Stale and blocked detection

A child is stale when it has not emitted progress after its idle timeout, or when
its parent context changes enough that the child contract is no longer valid.

Suggested metadata:

```rust
pub struct ChildStaleState {
    pub idle_timeout_secs: u64,
    pub last_progress_at: DateTime<Utc>,
    pub reason: String,
}
```

Blocked detection should reuse existing closeout taxonomy:

- `BLOCKED` when required input/resource is missing
- `NEEDS_CONTEXT` when child needs parent/user context
- `DONE_WITH_CONCERNS` when child can produce useful partial evidence
- `FAILED` for runtime/tool errors

Do not silently retry stale children forever. Parent integration decides whether
to retry, cancel, or proceed with concerns.

## Worktree integration

Child workflows that may edit files need an explicit worktree policy:

- verifier/reviewer/researcher are readonly by default
- coder children should usually run in an isolated worktree
- parent decides whether to apply child diffs
- child diff/artifact refs must be attached before applying
- worktree apply conflicts become child `Blocked` or
  `worktree-apply-conflict` eval candidates

First implementation can support readonly child roles before implementing
write-capable child worktrees.

## Runtime events and snapshots

Events should be stable enough for TUI/GUI consumption:

```rust
pub enum RuntimeEvent {
    ChildWorkflowPlanned { parent_id, child_id, role },
    ChildWorkflowStarted { child_id },
    ChildWorkflowProgress { child_id, summary },
    ChildWorkflowWaiting { child_id, reason },
    ChildWorkflowBlocked { child_id, reason },
    ChildWorkflowCancelled { child_id, reason },
    ChildWorkflowCompleted { child_id, status, evidence_ref },
    ChildWorkflowIntegrated { parent_id, child_id },
}
```

Snapshot fields:

- child id
- parent id
- role
- title
- status
- current phase
- last progress time
- evidence ref
- concerns
- cancellation state

The TUI should display child workflows as nested rows under the parent workflow,
not as separate top-level sessions by default.

## TUI visualization

Initial TUI surface (`/status`):

```text
Workflow: Fix parser bug                RUNNING
children:
  verifier  child-verify-1  Running            Verify parser fix
    summary: cargo test is still running
  reviewer  child-review-1  DoneWithConcerns   Review diff
    concerns: one medium-severity finding
    evidence: .imp/runs/parent/children/child-review-1/evidence.md
```

Current implementation renders child rows from the runtime snapshot under the
parent status output. It shows role, child id, status, title, summary, concerns,
and first evidence ref when present.

Planned interactions:

- expand child details
- open evidence output
- cancel child
- integrate/retry from parent context
- show blocked/stale reason

The TUI does not need tmux panes for child workflows. If terminal multiplexing is
later useful for debugging, it should be optional display plumbing, not the
runtime model.

## Current implementation status

Implemented foundation:

- role-scoped child workflow model and lifecycle state
- parent-to-child contract creation from workflow + role registry
- sequential child runner API using `Agent` + `AgentHandle`
- cancellation command path and stale/cancel policy decisions
- mana ledger adapters for child refs, child task records, and evidence records
- runtime event/snapshot summaries for child workflow status
- TUI `/status` rendering of nested child workflow rows
- documentation of UX and limitations

Still future work:

- interactive commands to spawn child workflows from the CLI/TUI
- full parallel child scheduling/resource management
- rich child detail panes and evidence navigation in the TUI
- write-capable coder children with isolated worktree apply/reject flows
- OMO-style full team orchestration

## Initial implementation slices

1. Design doc and terminology.
2. Child workflow run types and lifecycle states.
3. Contract creation from parent + role registry.
4. Sequential child workflow manager for verifier/reviewer/researcher.
5. Mana ledger persistence and evidence refs.
6. Runtime events and snapshots.
7. Cancellation and stale detection.
8. TUI nested child workflow display.
9. Documentation for UX and limitations.

## Non-goal comparison: OMO/tmux

OMO-style systems often make terminal sessions or tmux panes the visible unit of
work. That is useful as a debugging presentation, but imp should keep the durable
unit as workflow state:

- tmux pane: optional view
- child agent process: implementation detail
- workflow contract: authority boundary
- mana ledger: durable truth
- evidence packet: review/integration artifact

This keeps child delegation usable from CLI, TUI, GUI, CI, and mana without
binding the product to a particular terminal multiplexer.
