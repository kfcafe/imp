# imp Attach Path Cutover Plan

This is an audit-plus-cutover sequence from today’s `imp run <unit-id>` and compatibility `mana run` coexistence toward a lease-based attach model where `imp run {mana_id}` attaches a live imp runtime/session to a mana-owned run.

## Current Repo Reality

Inspected state in this worktree:

- `crates/imp-core/src/mana_worker.rs` declares itself the canonical single-unit mana worker runtime. It loads mana units through `mana_core::api`, assembles task context, and reports structured worker outcomes.
- `../mana/crates/mana-core/src/ops/run.rs` owns scheduling legality primitives today: `ReadyQueue`, `ReadyUnit`, `RunPlan`, `RunWave`, blocked units, warnings, retry context, dependency satisfaction, and target matching.
- `../mana/crates/mana-cli/src/commands/run/mod.rs` still presents `mana run` as dispatch/spawn behavior. It supports template-mode compatibility and direct mode that spawns `imp run <id>`.
- `../mana/crates/mana-cli/src/commands/run/plan.rs` adapts `mana_core::ops::run` plans into CLI dispatch units and wave planning.
- The originally referenced rebuild docs were not present in this worktree, so this plan is grounded in the inspected code and the mana unit contract.

## Target State

- `mana` owns durable/shared/coordinated execution truth: run records, node legality, leases, heartbeats, checkpoints, artifacts, verification records, and final resolution.
- `imp` owns live execution: model session, transcript, tool loop, runtime policy enforcement, context assembly, and operator interaction.
- `imp run {mana_id}` is the preferred live boundary. It attaches to a mana-owned run lease rather than independently deciding run legality.
- `mana run` becomes compatibility orchestration that creates/selects mana-owned runs and delegates live work to imp, then narrows or disappears once callers migrate.
- Transcript replay is not canonical recovery. Mana stores structured recovery substrate; imp stores/owns transcript/session behavior.

## Canonical Vocabulary

Before implementation, these concepts need one canonical owner:

- **Run**: mana-owned durable execution container for one target set, policy, scheduling snapshot, and aggregate outcome.
- **Run node**: mana-owned durable execution item corresponding to a unit attempt within a run.
- **Lease**: mana-owned exclusive right for a worker/runtime to execute or verify a run node for a bounded heartbeat interval.
- **Heartbeat**: mana-owned liveness update from imp for a lease.
- **Event**: durable structured state transition or progress summary. Not a transcript token stream.
- **Artifact**: durable reference to produced files, logs, verify output, patches, checkpoints, or evidence summaries.
- **Checkpoint**: mana-indexed restore/recovery anchor created or referenced by imp at important execution boundaries.
- **Resolution**: mana-owned final state transition for a node/run: closed, failed, abandoned, cancelled, or awaiting verify.

## Phase 0 — Preserve Current Primary Path

Owner: imp with mana compatibility.

Current canonical live path remains `imp run <unit-id>` through `imp-core/src/mana_worker.rs`.

Compatibility glue that may remain temporarily:

- `mana run` direct mode spawning `imp run <id>`.
- Template-mode spawning where local configs still depend on it.
- Current `ReadyQueue`/`RunPlan` scheduling helpers in mana-core.

Must not remain split long term:

- scheduler legality decisions duplicated between imp and mana;
- independent worker ownership without a durable lease;
- final outcome recording outside mana-owned run/node semantics.

Narrow proving slice:

- Keep one-shot `imp run <unit-id>` smoke working while adding no new lease semantics.
- Use this as the baseline regression gate for later phases.

## Phase 1 — Canonical Run/Node Schema in mana

Owner: mana.

Add durable schema/API for run and run-node records without changing execution yet.

Likely touched areas:

- `../mana/crates/mana-core/src/ops/run.rs`
- new mana-core run-record module or storage helpers
- mana CLI display/status surfaces

Minimum fields:

- run id, target, created_at, scheduler snapshot, policy/config snapshot;
- node id, unit id, attempt number, status, assigned worker identity, lease id optional;
- artifact/checkpoint refs;
- verify command and outcome;
- final resolution.

Narrow proving slice:

- `mana run --dry-run` or a new internal API can create a run plan record in dry-run/shadow mode and print it without dispatching.

Compatibility glue:

- Existing `RunPlan` remains the scheduler computation source.
- No live worker behavior changes yet.

Canonical before next phases:

- run/node ids and lifecycle states must be stable enough for imp to reference.

## Phase 2 — dual-write Compatibility Recording

Owner: shared mana + imp contract work.

Keep current `imp run <unit-id>` execution, but have it record into mana-owned run/node semantics in parallel with existing unit attempt/close behavior.

Likely touched areas:

- `crates/imp-core/src/mana_worker.rs`
- mana-core run APIs from Phase 1
- mana close/verify outcome adapters

Behavior:

- imp loads the assignment as today;
- mana creates or resolves a compatibility run/node for the unit;
- imp writes start/progress/checkpoint/result summaries to the node;
- existing unit close/fail behavior remains authoritative until parity is proven.

Narrow proving slice:

- `imp run <unit-id>` creates a compatibility run node and records final worker result while still closing/failing the unit through the existing path.

Compatibility glue:

- Existing attempt logs and unit notes remain.
- dual-write summaries may be redundant during this phase.

Canonical before next phases:

- result mapping from imp `WorkerResult` to mana run-node resolution.

## Phase 3 — Shadow Scheduling and Shadow Validation

Owner: mana.

Make mana compute authoritative legality/readiness and compare it against existing dispatch decisions without enforcing leases yet.

Likely touched areas:

- `../mana/crates/mana-core/src/ops/run.rs`
- `../mana/crates/mana-cli/src/commands/run/plan.rs`
- imp native mana tool/run orchestration surfaces if they show run state

Behavior:

- current `mana run` and `imp run` flows ask mana for legality/readiness snapshots;
- discrepancies are recorded as warnings/artifacts, not hard failures initially;
- unresolved decisions, dependency closure, scope warnings, artifact requirements, and retry policy are all evaluated by mana.

Narrow proving slice:

- Add a shadow-validation result to dry-run output that says whether a target is legal to attach and why not if blocked.

Compatibility glue:

- existing direct/template dispatch can continue.

Canonical before next phases:

- scheduler legality cannot remain split across imp and mana.

## Phase 4 — Lease-Based Attach for Single Unit

Owner: shared mana + imp; imp owns live worker, mana owns lease state.

Change the preferred `imp run {mana_id}` path so imp attaches to a mana-owned node lease.

Likely touched areas:

- `crates/imp-core/src/mana_worker.rs`
- `crates/imp-cli/src/lib.rs` / run command wiring
- mana-core lease APIs
- mana CLI status/run-state display

Behavior:

1. imp asks mana to create/select a run node for `{mana_id}`.
2. imp requests a lease for that node.
3. mana grants or rejects based on legality/readiness/current holder.
4. imp heartbeats while executing.
5. imp records checkpoints/artifacts through mana APIs.
6. imp resolves the node through mana with structured outcome.
7. mana updates unit state/attempt logs as a derived effect or compatibility projection.

Narrow proving slice:

- One single-unit `imp run <unit-id>` attach path for a local `.mana` unit: acquire lease, heartbeat once, execute current worker path, resolve lease with final worker status, and preserve existing verify/close behavior.

Compatibility glue:

- If no lease API is available, current path can still run behind an experimental flag.
- Existing transcript/session remains imp-local.

Canonical before next phases:

- Lease ownership and resolution must be the only accepted live-worker claim for the preferred path.

## Phase 5 — mana run Becomes Attach-Orchestrator Compatibility

Owner: mana CLI with imp worker contract.

Turn `mana run` into an orchestrator over mana-owned runs and imp attach workers rather than an independent spawn/dispatch owner.

Likely touched areas:

- `../mana/crates/mana-cli/src/commands/run/mod.rs`
- `../mana/crates/mana-cli/src/commands/run/ready_queue.rs`
- `../mana/crates/mana-cli/src/commands/run/wave.rs`
- `../mana/crates/mana-cli/src/commands/run/plan.rs`

Behavior:

- `mana run` computes/creates run nodes;
- dispatch workers by asking `imp run <node-or-unit>` to attach;
- monitor lease heartbeats/results;
- render run status from mana-owned state;
- continue supporting dry-run and json-stream output.

Narrow proving slice:

- Direct mode dispatches one wave through lease-based imp attach and reports outcomes entirely from mana run-node state.

Compatibility glue:

- Template mode may stay temporarily but should be documented as legacy.

Canonical before next phases:

- `mana run` must no longer invent lifecycle states not representable in mana run/node records.

## Phase 6 — Transcript Ownership Finalization

Owner: imp for transcript/session, mana for recovery artifacts.

Clarify and enforce that imp transcripts are runtime/session artifacts, not mana’s canonical recovery substrate.

Behavior:

- imp may publish bounded transcript summaries/artifact refs;
- mana stores structured checkpoints, verify outputs, artifacts, and final evidence summaries;
- recovery starts from mana run/node/checkpoint state and may ask imp to resume/reconstruct context, but not by replaying transcript as truth.

Narrow proving slice:

- A failed lease-based run records checkpoint/ref + verify failure artifact in mana while imp keeps transcript/session local.

## Phase 7 — Narrow or Deprecate mana run

Owner: mana CLI and docs.

Once attach orchestration is stable, decide the remaining role for `mana run`.

Options:

- keep as a planner/orchestrator that always delegates live execution to imp attach;
- keep only dry-run/status/retry orchestration and move live execution entrypoint to `imp run`;
- deprecate template spawning entirely.

Narrow proving slice:

- Emit deprecation/help text for template mode once direct lease attach is stable.

## First Lease-Based Attach Proving Slice

The first real implementation target should be deliberately small:

1. Add mana-core APIs for a single run node lease: create/select node, acquire, heartbeat, resolve.
2. Add an experimental imp path that calls those APIs around the existing `mana_worker` execution.
3. Run against one local task with no parallelism.
4. Preserve existing verify/close behavior.
5. Assert that mana records the lease lifecycle and final resolution.

This proves the ownership boundary without changing scheduling, parallel dispatch, transcript storage, or `mana run` user behavior.

## Non-Goals

- No flag-day replacement of current execution paths.
- No transcript replay as canonical recovery.
- No scheduler legality split between imp and mana after shadow validation hardens.
- No direct backend/runtime ownership in mana.
- No `mana run` semantic expansion beyond compatibility orchestration.
