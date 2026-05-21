# mana-core Embedding Surface Audit

This audit compares the current `mana-core` embedding surface against the target semantic, lease-based, library-first contract recorded in root mana unit `51.6`.

## Inspected files

- `../mana/crates/mana-core/src/api/mod.rs`
- `../mana/crates/mana-core/src/ops/mod.rs`
- `../mana/crates/mana-core/src/ops/run.rs`
- `../mana/crates/mana-cli/src/commands/run/mod.rs`
- `../mana/.mana/51.6-define-the-next-mana-core-embedding-slice-add-stab.md`

Referenced files named by the task but missing in this imp worktree:

- `docs/rebuild/mana-lease-model.md`
- `docs/rebuild/mana-imp-ownership-boundary.md`

The target contract details were available from `../mana/.mana/51.6...` and are used below instead of inventing a new contract.

## Target contract from 51.6

The embedding surface should be semantic, lease-based, and library-first.

Minimum durable concepts:

- `Run`
- `Lease`
- `Event`
- `ArtifactRef`
- `CheckpointRef`
- executor `Capabilities`

Minimum semantic operations to evaluate:

- `create_run(spec, input, policy)`
- `get_snapshot(run_id)`
- `stream_events(run_id, after_cursor?)`
- `attach_executor(run_id, capabilities, mode?)`
- `heartbeat(lease_id, progress?, checkpoint_refs?)`
- `append_evidence(lease_id, facts?, artifacts?, checkpoints?, proposed_graph_delta?)`
- `resolve_lease(lease_id, outcome, payload?)`
- `control_run(run_id, pause|resume|cancel|retry|inject_input)`

Rules:

- clients never set canonical state directly;
- clients should not reimplement graph/scheduler semantics to find work;
- dynamic graph changes are proposals validated by mana;
- identical semantics must exist in-process and over RPC;
- transcript is never the canonical recovery substrate.

## Current aligned surfaces

### Broad library access exists

`mana-core/src/api/mod.rs` already presents itself as a programmatic API for embedding mana in another application. It emphasizes:

- no stdout/stderr side effects;
- structured params/results;
- `&Path` entrypoint;
- serializable types;
- no singleton/global state.

This aligns with the library-first direction.

### Unit graph/query/mutation APIs exist

`api/mod.rs` exposes discovery, query, mutation, status, context, fact, verify, and run-planning surfaces. These are useful substrate pieces for an embedder, especially for snapshots and context assembly.

### Scheduler computation is already in mana-core

`ops/run.rs` owns dependency-aware scheduling primitives:

- `RunTarget`
- `ReadyUnit`
- `ReadyQueue`
- `RunPlan`
- `RunWave`
- `BlockedUnit`
- `RunScopeWarning`

This is directionally correct: clients should ask mana for readiness rather than duplicating graph semantics.

### CLI direct mode already delegates live work to imp

`mana-cli/src/commands/run/mod.rs` documents direct mode spawning `imp run <id>` when no template config is present. That matches the ownership direction where imp owns live execution and mana owns durable work state, but it is still a CLI spawn behavior rather than a stable library attach contract.

## Missing target concepts

### `Run`

Current `ops/run.rs` has `RunPlan` and scheduling waves, but not a durable run record API matching `create_run(spec, input, policy)`.

`api/mod.rs` re-exports `RunRecord` / `RunResult` from `unit`, but the inspected run surface is still plan/dispatch oriented, not a first-class durable run container with events, leases, and snapshots.

### `Lease`

No inspected API exposes lease acquisition, holder identity, heartbeat deadline, capabilities, or resolution.

Missing operations:

- `attach_executor`
- `heartbeat`
- `resolve_lease`

This is the biggest gap for replacing spawn-oriented coordination with semantic attach.

### `Event`

No inspected `mana-core` API exposes a run event stream with cursors.

Missing operations:

- `stream_events(run_id, after_cursor?)`
- append structured run/lease progress events

Current CLI JSON stream behavior is not the same as a durable mana-owned event stream.

### `ArtifactRef` / `CheckpointRef`

The inspected APIs expose verify results and unit metadata, but there is no obvious embedding-critical run-level operation for appending artifact/checkpoint refs during an executor lease.

Missing operation:

- `append_evidence(lease_id, facts?, artifacts?, checkpoints?, proposed_graph_delta?)`

### executor `Capabilities`

`ReadyUnit` includes scheduling metadata and model override, but there is no executor capabilities handshake that lets mana decide whether an executor may attach to a run/node.

Missing operation:

- `attach_executor(run_id, capabilities, mode?)`

### `control_run`

No inspected library surface exposes semantic controls like pause/resume/cancel/retry/inject input for a durable run.

## Wrong-layer or compatibility-shaped surfaces

### `mana-cli run` is CLI-shaped

`mana-cli/src/commands/run/mod.rs` owns terminal/CLI behavior:

- argument struct;
- dry-run/loop/json-stream flags;
- template mode;
- direct spawning of `imp run <id>`;
- review integration;
- terminal detection.

This should remain compatibility/orchestration UX, not the embedding boundary.

### Template mode is compatibility-only

Template mode is explicitly backward compatibility. It should not shape the semantic embedding contract.

### `RunPlan` is not a durable `Run`

`RunPlan` is valuable scheduling output, but it is not a run lifecycle object. It should feed `create_run` or `get_snapshot`, not substitute for run/lease APIs.

### Raw unit status mutation is insufficient

Existing close/update/status operations are useful maintenance primitives, but the target explicitly rejects raw canonical state setters as the executor contract. Executors should resolve leases and propose evidence/graph deltas; mana should validate and own the durable state transition.

## Embedding-critical next pieces

Critical now:

1. A top-level run service/module in `mana-core` that can create a run from existing `RunTarget`/`RunPlan` output.
2. A minimal lease object and attach/heartbeat/resolve API for one executor and one run node.
3. A run snapshot type that exposes run/node/lease status without forcing clients to parse CLI output.
4. A reference-first event/evidence append shape for artifacts/checkpoints/verifier summaries.

Can remain lower-level/CLI compatibility for now:

- template dispatch;
- wave spawning details;
- review-after-close behavior;
- terminal/json-stream rendering;
- maintenance ops such as init/sync/config/adopt/unarchive unless a concrete embedder needs them.

## Recommendation

Do not start by adding wrappers for every lower-level `ops::*` module. That would expand the embedding API without solving the run/lease boundary.

Instead, define one stable library-facing spawn/attach boundary in `mana-core`:

- `mana_core::run_service` or `mana_core::api::runs`
- backed by current `ops::run` planning logic;
- initially local/in-process only;
- no cloud/service architecture;
- no transcript/chat recovery model.

The CLI and RPC layers can later adapt to the same service, but the semantic contract should be implemented in `mana-core`, not in `mana-cli` or `mana-pool`.

## First proving slice

Smallest coherent implementation slice:

1. Add mana-core types only:
   - `RunId`
   - `RunNodeId`
   - `LeaseId`
   - `RunSnapshot`
   - `ExecutorCapabilities`
   - `LeaseHeartbeat`
   - `LeaseOutcome`
   - `RunEvent`
   - `ArtifactRef`
   - `CheckpointRef`
2. Add in-process service functions in a new module or `api::runs`:
   - `create_run(mana_dir, spec)` using current `compute_run_plan` semantics;
   - `get_snapshot(mana_dir, run_id)`;
   - `attach_executor(mana_dir, run_id, capabilities)` for a single ready node;
   - `heartbeat(mana_dir, lease_id, progress)`;
   - `resolve_lease(mana_dir, lease_id, outcome)`.
3. Persist enough run/lease state to prove lifecycle, but do not replace `mana run` dispatch yet.
4. Add unit tests using a temporary `.mana` fixture.

Future verify direction:

```sh
cargo test -p mana-core run_service -- --nocapture
cargo check -p mana-cli
```

The first test should prove:

- a run can be created from current readiness planning;
- an executor can attach to exactly one eligible node;
- heartbeat updates lease liveness;
- resolving the lease records final outcome;
- no client directly sets unit status through the lease API.

## First stable spawn/attach boundary location

The first stable boundary belongs in `mana-core`.

Rationale:

- The semantics must be identical in-process and over RPC.
- `mana-cli` is presentation/orchestration compatibility, not canonical state semantics.
- `mana-pool` may execute pools/workers but should not own durable graph/run legality.
- `imp` owns live agent execution, not mana run/lease truth.

After the mana-core service exists:

- `mana-cli run` can become a compatibility adapter over it;
- `imp run {mana_id}` can attach through it;
- RPC can expose the same semantic operations.

## Explicit non-goals

- Do not implement broad wrappers for every ops module before run/lease exists.
- Do not treat transcript/chats as recovery substrate.
- Do not make `mana-cli run` the stable embedding contract.
- Do not add raw status setters for executors.
- Do not invent remote/cloud orchestration.

## Conclusion

The current embedding surface is solid for unit/query/mutation operations and readiness planning, but it lacks the semantic run/lease/event/evidence layer required by the target contract. The next coherent slice is a small `mana-core` run/lease service that wraps current planning logic and proves create/snapshot/attach/heartbeat/resolve locally before changing CLI or imp execution behavior.
