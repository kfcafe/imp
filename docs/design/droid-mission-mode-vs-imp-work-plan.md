# Droid Mission Mode vs imp-work: audit and implementation plan

Status: review-only audit  
Date: 2026-05-22  
Scope: `~/imp` native `imp-work` and CLI run surfaces, compared against Droid Mission Mode concepts extracted from `~/.local/bin/droid`.

## Key finding

`imp-work` already has many of the right primitives for a Droid Mission-style system, but the existing `run` surface is still **workflow compatibility**, not native imp-work orchestration.

Evidence:

- `crates/imp-cli/src/lib.rs:409` defines hidden `Run(HeadlessWorkflowArgs)` as a compatibility alias.
- `crates/imp-cli/src/lib.rs:1200` routes `Commands::Run` through `run_headless_mode(...)`.
- `HeadlessWorkflowArgs` requires `unit_id` and `workflow_dir`, confirming `imp run` currently targets workflow units, not native work tasks/epics.
- `crates/imp-work/src/scheduler.rs` already has native scheduler concepts:
  - `WorkerProfile`
  - `RunPolicy`
  - `DispatchPlan`
  - `MultiAgentRunPlan`
  - `LeaseRecord`
  - path conflict blocking
  - dependency readiness
  - worker completion aggregation
- `crates/imp-core/src/tools/work.rs` exposes native work actions including `next`, `claim`, `outcome`, `runs`, `tree`, `verify`, `close`, and `fail`, but **not `run`**.

The correct direction is not to invent `run` from scratch. It is to **promote the existing imp-work scheduler/run primitives into a real native `work run` / Work Control product surface, then migrate `imp run` from workflow compatibility to native work orchestration once parity is proven.**

---

## Audit: current imp-work capabilities

### 1. Durable task graph

`crates/imp-work/src/model.rs` includes:

- `Task`
- `Epic`
- `Decision`
- `MemoryItem`
- `Prototype`
- `Check`
- `ContextPack`
- `Run`
- `Lease`

This is already stronger than Droid's mission-local `features.json` in terms of generality.

### 2. Dependency/readiness logic

`crates/imp-work/src/workflow.rs` has:

- `build_work_tree`
- `readiness_for`
- missing dependency warnings
- dependency cycle detection
- blockers

`crates/imp-work/src/scheduler.rs` uses dependencies to filter `ready_queue`.

### 3. Scheduler with bounded concurrency

`crates/imp-work/src/scheduler.rs` has:

- `RunPolicy { max_jobs, path_conflicts }`
- `plan_dispatch`
- `plan_multi_agent_run`
- `complete_multi_agent_run`
- tests for bounded jobs and dependency waves

This is the kernel of a Mission runner.

### 4. Leases/path locks

`LeaseRequest`, `LeaseRecord`, `path_locks`, and conflict checks already exist.

Important tests:

- `scheduler_rejects_conflicting_path_locks`
- `scheduler_policy_blocks_conflicting_ready_paths`

### 5. Outcome persistence

`crates/imp-work/src/store.rs` has:

- `append_run`
- `append_outcome`
- `write_coordinator_summary`
- `persist_worker_result`
- memory updates
- followup task creation
- context staleness after outcome

This maps well to Droid's worker handoff/progress log ideas.

### 6. Verification/close/fail conventions

`crates/imp-work/src/workflow.rs` has:

- `close_task_with_conventions`
- `fail_task_with_conventions`
- `summarize_checks`

`close_task_with_conventions` requires verify for checked tasks unless forced.

This is stronger than many agent runners because it makes closeout evidence explicit.

---

## Gaps versus Droid Mission Mode

### Gap 1: no native imp-work `run` tool action

The `work` tool schema has:

```text
create, list, context, refresh_context, next, show, update, claim,
dep_add, dep_remove, validate, scope, guide, search, outcome,
prototype_outcome, runs, tree, verify, close, fail, remember
```

No `run`.

Evidence:

- `crates/imp-core/src/tools/work.rs` tool schema action enum excludes `run`.
- The dispatch `match action` excludes `run`.

This means the scheduler exists in library code but is not exposed as the durable orchestration command/tool.

### Gap 2: scheduler is in-memory, not fully store-backed

`Scheduler` stores tasks, leases, runs, outcomes in memory:

```rust
pub struct Scheduler {
    tasks: BTreeMap<String, Task>,
    leases: BTreeMap<String, LeaseRecord>,
    runs: Vec<Run>,
    outcomes: Vec<WorkOutcome>,
    path_locks: BTreeMap<PathBuf, String>,
    tick: u64,
}
```

`WorkStore::load_scheduler()` appears to hydrate enough for `next`/`claim`, but native multi-agent run planning/completion is not exposed as a persisted coordinator transaction.

Impact:

- Good for planning/tests.
- Not yet a robust mission runner.
- Needs durable run/session IDs, event log, resumability, and partial failure recovery.

### Gap 3: no mission/project-run object

Droid has a mission-level object/state machine. imp-work has `Epic` and `Run`, but no explicit “orchestration run” object with:

- goal
- root epic/task
- current wave
- worker leases
- validators
- status
- policy
- event log
- pause/resume state
- handoff queue

Current `Run` is a single work outcome:

```rust
pub struct Run {
    pub id: WorkId,
    pub work_id: Option<WorkId>,
    pub context_pack_id: Option<WorkId>,
    pub outcome: RunOutcome,
    pub summary: String,
    pub changed_paths: Vec<PathBuf>,
    pub checks: Vec<CheckResult>,
}
```

That is not enough to represent a Droid-style mission coordinator.

### Gap 4: worker handoff schema is too thin

Current `WorkOutcome`:

```rust
pub struct WorkOutcome {
    pub work_id: WorkId,
    pub outcome: RunOutcome,
    pub summary: String,
    pub changed_paths: Vec<PathBuf>,
    pub checks_passed: usize,
    pub checks_failed: usize,
    pub memory_updates: Vec<String>,
    pub followups: Vec<String>,
}
```

Good start, but Droid-like missions need more structured handoff:

- success state
- discovered issues
- left undone
- suggested fix
- validator results
- commit/worktree refs
- evidence refs
- replan recommendations
- whether orchestrator should continue, pause, or ask user

### Gap 5: no validator/reviewer lane

imp-work supports checks and review-ish status, but no native concept of:

- validator profile
- validation task generated after implementation
- milestone-level validation
- user-testing gate
- scrutiny pass
- review quorum

Droid's validator workers are a powerful piece worth stealing, but imp should implement them as named worker profiles/check tasks, not hardcoded magic.

### Gap 6: no first-class pause/resume/cancel for orchestration

`claim` and `outcome` can recover individual tasks, but there is no coordinator-level:

- pause run
- resume run
- stop worker
- retry failed task
- continue next wave
- show active workers
- show blocked reason tree

Droid Mission Mode's biggest product advantage is the “mission control” loop.

### Gap 7: `imp run` namespace conflict

There is already an `imp run`, but it is hidden workflow compatibility.

This matters because adding native `imp run` directly would be a migration/UX decision, not just implementation. Safer path:

1. Add `work(action="run")` tool.
2. Add `imp work run ...` CLI.
3. Keep hidden `imp run` alias for workflow during transition.
4. Later repoint `imp run` to native work when parity is proven.

---

## Recommended plan

### Phase 0 — Decide naming and compatibility boundary

Goal: avoid breaking existing workflow workflows.

Recommendation:

- Keep current hidden `imp run <workflow-id>` behavior for now.
- Add native:
  - tool: `work(action="run")`
  - CLI: `imp work run <task-or-epic-id>`
  - later alias: `imp run --work <id>` or repoint `imp run`.

Acceptance:

- Existing `imp run 5.1 --defer-verify` tests still pass.
- Native work run has explicit name and docs.

### Phase 1 — Expose native scheduler planning as read-only/dry-run

Add `work(action="run", dry_run=true)` or separate `work(action="plan_run")`.

Inputs:

- `id`: task or epic root
- `jobs`
- `keep_going`
- `require_context`
- `path_conflicts`
- `worker_profile`
- optional `validator_profile`

Output:

- dispatchable tasks
- blocked tasks
- dependency blockers
- path conflicts
- proposed leases
- proposed waves

Implementation should use existing:

- `WorkStore::load_scheduler`
- `Scheduler::plan_dispatch`
- `Scheduler::plan_multi_agent_run`

But do not yet spawn workers.

Acceptance:

- Returns machine-readable plan.
- Does not mutate store in dry-run.
- Tests cover dependencies, max jobs, path conflicts, missing context.

Why first:

- Low risk.
- Makes existing scheduler useful.
- Lets UI/agents inspect run feasibility before real execution.

### Phase 2 — Add durable orchestration event log

Before spawning anything, add append-only event records.

Suggested new model:

```rust
pub struct WorkRun {
    pub id: WorkId,
    pub root_work_id: WorkId,
    pub status: WorkRunStatus,
    pub policy: WorkRunPolicy,
    pub started_at: String,
    pub updated_at: String,
    pub current_wave: usize,
}

pub enum WorkRunStatus {
    Planning,
    Running,
    Paused,
    WaitingForWorkers,
    WaitingForVerify,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

pub struct WorkRunEvent {
    pub sequence: u64,
    pub run_id: WorkId,
    pub kind: WorkRunEventKind,
    pub timestamp: String,
}
```

Event kinds:

- `run_created`
- `wave_planned`
- `worker_leased`
- `worker_started`
- `worker_heartbeat`
- `worker_completed`
- `worker_failed`
- `verify_started`
- `verify_completed`
- `handoff_recorded`
- `run_paused`
- `run_resumed`
- `run_completed`

Storage:

```text
runs/<run-id>/run.json
runs/<run-id>/events.jsonl
runs/<run-id>/summary.json
runs/<run-id>/workers/<lease-id>.json
runs/<run-id>/evidence.md
```

Acceptance:

- Event append is deterministic and durable.
- `work(action="runs")` can show orchestration runs and legacy single-task runs.
- Existing `Run` outcome files remain compatible.

This is one of the most important Droid lessons: state machine + progress log.

### Phase 3 — Strengthen worker handoff

Expand `WorkOutcome` or add a separate `WorkerHandoff`.

Recommended additive type:

```rust
pub struct WorkerHandoff {
    pub work_id: WorkId,
    pub lease_id: Option<WorkId>,
    pub success_state: HandoffSuccessState,
    pub summary: String,
    pub changed_paths: Vec<PathBuf>,
    pub evidence: Vec<EvidenceRef>,
    pub checks: Vec<CheckResult>,
    pub discovered_issues: Vec<String>,
    pub left_undone: Vec<String>,
    pub suggested_fixes: Vec<String>,
    pub followups: Vec<String>,
    pub memory_updates: Vec<String>,
    pub next_action: Option<String>,
    pub should_return_to_coordinator: bool,
}
```

Keep current `WorkOutcome` as the simple path and derive `WorkerHandoff` from it when absent.

Acceptance:

- `work(action="outcome")` can accept structured evidence/handoff fields.
- Followups can become real tasks.
- Discovered issues can become task/memory/decision records.
- Handoff is shown in `runs`.

This borrows Droid's best handoff ideas while keeping imp's evidence-first model.

### Phase 4 — Native `work run` wave coordinator, no autonomous spawn yet

Implement a coordinator that:

1. loads root task/epic tasks
2. plans ready wave
3. creates durable `WorkRun`
4. claims tasks with leases/path locks
5. returns instructions/context for workers

This is a “manual worker” runner. It should not yet launch child agents.

Tool behavior:

```json
{
  "action": "run",
  "id": "E-...",
  "jobs": 3,
  "dry_run": false
}
```

Returns:

- run id
- leased tasks
- worker prompts/context pack IDs
- blocked tasks
- next command/action

Acceptance:

- Store persists leases.
- Store persists run event log.
- Interrupted run can be inspected.
- No duplicate claims.
- Ready tasks with stale/missing context can be blocked if `require_context=true`.

This maps to Droid Mission Control without needing process orchestration yet.

### Phase 5 — Add real worker spawning behind a runtime provider boundary

Do not couple imp-work directly to the agent loop.

Introduce abstraction:

```rust
trait WorkRunner {
    async fn start_worker(&self, assignment: WorkerAssignment) -> Result<WorkerHandle>;
    async fn poll_worker(&self, handle: WorkerHandle) -> Result<WorkerState>;
    async fn stop_worker(&self, handle: WorkerHandle) -> Result<()>;
}
```

Provider implementations later:

- current process/manual
- headless imp agent
- git worktree worker
- remote/daemon worker

This avoids burying runtime behavior in the store.

Acceptance:

- Scheduler/store tests do not require LLM runtime.
- Worker provider has conformance tests.
- Event log records provider/session IDs.

### Phase 6 — Validator lanes

Add validators as regular work/control constructs, not magic.

Model:

- implementation task closes with evidence
- validation task/check is generated or required
- validator profile runs read-only/reviewer mode
- final close requires implementation + validation gates

Inputs:

- `validator_profile`
- `review_after_run`
- `verify_mode`: inline/deferred/batch/manual

Acceptance:

- A failed validator reopens/blocks the task with structured reason.
- A passing validator attaches evidence.
- Validation can run at task or epic/milestone level.

This is Droid's validator-worker idea implemented in an imp-native evidence model.

### Phase 7 — TUI/CLI Work Control

Once durable state exists, build UI.

CLI:

```sh
imp work run E-123 --jobs 3 --dry-run
imp work runs
imp work run-state R-456
imp work pause R-456
imp work resume R-456
imp work stop-worker L-789
imp work retry T-123
```

TUI:

- active run
- current wave
- leased workers
- blockers
- handoffs
- changed paths
- verify status
- evidence refs
- next action

This should be simple, inspectable, local-first — not project-workflowgement bloat.

---

## Highest-risk implementation details

### 1. Double-claim and stale leases

Current `claim_task` directly updates task status and appends a lease, but does not appear to use a single atomic scheduler transaction.

Risk:

- two agents claim the same task under concurrency
- stale `Active` tasks remain stuck
- path locks are advisory unless scheduler state is consistently loaded

Plan:

- add store-level `claim_ready_task_transaction`
- detect existing active leases
- add lease expiry/heartbeat
- add stale lease recovery

### 2. Path lock source quality

`scheduler.rs` derives `task_paths` only from `SourceKind::FileRange`.

Tasks may have relevant `paths` in other forms or no paths. Droid-style parallel workers need conservative path locking.

Plan:

- normalize source refs and explicit paths into task locks
- allow declared path locks
- default unknown-write tasks to project-wide lock unless worker is read-only
- support worktree isolation later

### 3. Verification is currently simulated in tool action

`verify_task` uses `checks_passed` / `checks_failed` params; it does not run commands.

That is okay for recording results, but not enough for autonomous run.

Plan:

- separate:
  - `verify_record`
  - `verify_run`
- execute check commands through policy/reference monitor
- save output refs

### 4. `Run` naming ambiguity

Current `Run` means “task outcome run.” Droid-like orchestration needs “coordinator run.”

Avoid overloading too much:

- keep `Run` for task attempt/outcome if needed
- add `WorkRun` / `CoordinatorRun` / `ProjectRun`

### 5. Do not hardcode roles

Droid has orchestrator/worker/validator. imp should avoid hardcoding too much.

Use:

- worker profiles
- `.imp/agents` later
- policy/tool sets
- validator profile config

---

## Concrete initial task breakdown

### Task 1: Add native imp-work run planning action

Acceptance:

- `work(action="run", dry_run=true, jobs=N)` returns dispatch plan.
- Supports `id`, `jobs`, `require_context`, `path_conflicts`.
- Does not mutate store in dry-run.
- Tests cover no-ready, dependency-blocked, path-conflict, max-jobs.

Files:

- `crates/imp-core/src/tools/work.rs`
- `crates/imp-work/src/scheduler.rs`
- tests in both crates

### Task 2: Add WorkRun model and event log

Acceptance:

- New model types serialize/deserialize.
- Store can create/load/list work runs.
- Events append as JSONL with sequence numbers.
- `work(action="runs")` includes coordinator runs.

Files:

- `crates/imp-work/src/model.rs`
- `crates/imp-work/src/store.rs`
- maybe `crates/imp-work/src/run.rs`

### Task 3: Implement non-spawning coordinator run start

Acceptance:

- `work(action="run", dry_run=false)` creates a `WorkRun`, plans first wave, persists leases/events, returns worker assignments.
- Re-running against active run reports existing state rather than double-claiming.
- Leases release on `outcome`.

Files:

- `crates/imp-core/src/tools/work.rs`
- `crates/imp-work/src/scheduler.rs`
- `crates/imp-work/src/store.rs`

### Task 4: Structured handoff schema

Acceptance:

- `work(action="outcome")` accepts evidence, discovered issues, left undone, suggested fixes.
- Followups can materialize as tasks.
- Handoff appears in run-state/runs output.

Files:

- `crates/imp-work/src/model.rs`
- `crates/imp-work/src/store.rs`
- `crates/imp-core/src/tools/work.rs`

### Task 5: CLI `imp work run`

Acceptance:

- Does not disturb hidden workflow `imp run`.
- `imp work run <id> --dry-run --jobs N` calls native work planning.
- `imp work runs` lists native coordinator runs.

Files:

- `crates/imp-cli/src/lib.rs`

### Task 6: Verification execution

Acceptance:

- Check commands can be run with timeout/policy.
- Output refs are persisted.
- Close requires passing executed checks or force reason.
- Batch verify can run after a wave.

Files:

- `crates/imp-core/src/tools/work.rs`
- runtime/policy integration
- store artifacts

---

## Recommended MVP

The best small-but-powerful MVP is:

```text
Native work run, manual workers, durable event log.
```

Specifically:

1. `work(action="run", dry_run=true)` — plan wave.
2. `work(action="run", dry_run=false)` — create coordinator run + leases.
3. `work(action="runs")` — show active/completed coordinator runs.
4. `work(action="outcome")` — release lease and append handoff/event.
5. `work(action="run_state")` or `runs id=...` — show current wave, blockers, handoffs, next action.

Do **not** spawn child LLM workers in the first slice. Get the state model right first.

That would give imp the most powerful structural parts of Droid Mission Mode:

- durable mission/control object
- dependency waves
- leases/workers
- progress log
- handoff
- resumability
- verification hooks

while preserving imp's strengths:

- evidence
- local-first
- transparent store
- explicit policy
- no hidden product magic
