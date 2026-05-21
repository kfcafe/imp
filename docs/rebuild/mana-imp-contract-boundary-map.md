# mana ↔ imp Contract Boundary Map

This map records the current inspected contract boundary and the next migration slice for epic `47.1`.

## Current reality

The shared worker/evidence shapes are **imp-local today** in `crates/imp-core/src/contracts.rs`.

There is no `crates/tower-contracts/src/lib.rs` in this worktree. Earlier planning notes that preferred a neutral Tower-level contracts crate do not match the current repository layout. The practical boundary today is:

- imp defines worker-facing DTOs in `imp_core::contracts`;
- `crates/imp-core/src/mana_worker.rs` re-exports worker assignment/result types for existing call sites;
- mana provides durable unit loading, verify/close operations, scheduling primitives, and future run/lease substrate;
- no cross-crate shared protocol package is currently available in this worktree.

## Existing imp-local contract types

### `WorkerAssignment`

Current role: imp runtime input assembled from a mana unit.

Contains:

- unit identity/title/description/design/acceptance;
- verify command and timeout;
- fail-first flag;
- notes, decisions, dependencies, paths/files;
- previous attempts;
- workspace root;
- optional model override.

Boundary meaning: this is close to the future `TaskSpec` + `WorkerAssignment` split. Durable task facts originate in mana; imp should only receive a normalized assignment snapshot for live execution.

Migration target:

- mana-owned task/run node record stores durable unit/run state;
- imp-owned assignment snapshot carries the bounded runtime inputs needed for one execution attempt.

### `WorkerResult` and `WorkerStatus`

Current role: imp runtime output after a worker attempt.

Contains:

- unit id;
- status (`awaiting_verify`, `completed`, `blocked`, `failed`, `cancelled`);
- summary/error;
- turn/tool/token/cost/model counters.

Boundary meaning: this is the current seed of a future `WorkerOutcome`. It should map into mana-owned run-node resolution, not become a second durable lifecycle universe.

Migration target:

- imp reports `WorkerOutcome` for a lease/run node;
- mana records final node/run resolution and derives unit state changes.

### `VerifierResult`

Current role: shared verifier lineage record.

Contains:

- verifier name;
- status;
- command/exit code/summary;
- artifact references;
- start/finish timestamps;
- run id and unit id.

Boundary meaning: this is already close to the desired verifier result shape. It should remain reference-first and durable-output oriented.

Migration target:

- mana owns verification records and artifact storage/indexing;
- imp may produce or forward verifier evidence but should not be the durable source of truth.

### `ArtifactRef`

Current role: durable artifact reference without embedding heavy payloads.

Contains:

- artifact id;
- artifact kind;
- locator;
- optional run id/unit id/stage.

Boundary meaning: this is the right direction. It avoids moving logs/patches/output through primary DTOs.

Migration target:

- expand only as concrete artifact stores require;
- keep payloads external and reference-first.

### `EvidenceBundleRef`

Current role: minimal reference-first evidence bundle.

Contains:

- bundle id;
- unit id;
- optional run id;
- artifact refs;
- summary.

Boundary meaning: this is a seed for durable evidence promotion across imp and mana. It overlaps with the later durable evidence summary direction but should stay small until multiple producers/consumers require typed exchange.

Migration target:

- mana owns evidence bundle records;
- imp can emit evidence summaries and artifact refs as part of worker/verify outcomes.

## Ownership rules

### mana owns durable substrate

- unit graph and metadata;
- run and run-node records;
- scheduling legality;
- leases and heartbeats;
- verification records;
- checkpoints and artifact indexes;
- durable evidence summaries;
- final resolution/close/fail state.

### imp owns live runtime

- model session and transcript behavior;
- context assembly for a worker attempt;
- tool loop execution;
- runtime policy enforcement;
- operator interaction;
- local transcript/session artifacts;
- conversion from live attempt output into bounded worker/evidence DTOs.

## Placement decision for now

Do not move these contracts yet.

Because the current repository has `imp_core::contracts` but no neutral contracts crate, the least risky next step is to harden the imp-local contract module and explicitly map it to mana-owned future run/lease APIs. A crate move should wait until mana and imp both consume the same typed package in this worktree.

## first implementation slice

The **first implementation slice** should be small and non-disruptive:

1. Rename or document `WorkerAssignment` as the live assignment snapshot generated from mana-owned durable state.
2. Add a typed conversion/mapping note or helper from `WorkerResult` + `VerifierResult` into a future mana run-node outcome shape.
3. Keep all public call sites stable through existing `mana_worker.rs` re-exports.
4. Add tests proving the DTOs serialize in stable snake_case/reference-first form.
5. Do not introduce leases, new crates, or cross-repo moves in this slice.

## Compatibility adapter direction

During migration:

- `mana_worker::load_assignment` remains the adapter from mana unit state to imp runtime assignment;
- `mana_worker` continues re-exporting `WorkerAssignment`, `WorkerAttempt`, `WorkerResult`, and `WorkerStatus`;
- new mana run/lease APIs should accept a bounded outcome/evidence projection rather than imp internals;
- old call sites should compile unchanged until the run-node protocol exists.

## Boundary drift risks

- Duplicating lifecycle status between `WorkerStatus` and future mana run-node state.
- Persisting imp transcript details as mana canonical recovery state.
- Expanding `EvidenceBundleRef` into a payload-heavy log container.
- Moving crates before both sides have a concrete consumer.
- Letting `mana_worker.rs` continue accumulating durable substrate logic instead of delegating that to mana APIs.
