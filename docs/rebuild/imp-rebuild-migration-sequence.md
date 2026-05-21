# imp Rebuild Migration Sequence and Shim Policy

This plan sequences the imp rebuild as an incremental migration with compatibility shims. It intentionally avoids a flag-day rewrite: existing `imp chat`, `imp print`, `imp run`, `imp view`, TUI, and RPC paths must remain usable while ownership seams are clarified.

## Migration principles

- Land one narrow slice at a time behind existing public entrypoints.
- Prefer compatibility shim modules and re-exports over broad call-site churn.
- Keep runtime behavior stable before moving ownership boundaries.
- Add verify gates to every slice before removing old paths.
- Roll back by disabling the new adapter/shim, not by reverting unrelated refactors.

## Phase 1 — Contract boundary hardening

Goal: stabilize the current imp-local boundary before creating or moving shared packages.

Order:
1. Keep `imp_core::contracts` as the current contract home.
2. Document `WorkerAssignment` as the live assignment snapshot from mana-owned durable state.
3. Keep `mana_worker.rs` re-exports for compatibility shim stability.
4. Add serialization/shape tests for reference-first DTOs.

Verify gate:

- `cargo test -p imp-core contracts -- --nocapture`
- `cargo check -p imp-core`

Rollback:

- Revert only the new helper/tests; keep old re-exports and call sites intact.

## Phase 2 — imp-core decomposition

Goal: reduce file/module size without changing behavior.

Order:
1. Extract pure DTOs/helpers first.
2. Extract context assembly and worker result mapping next.
3. Extract tool/runtime policy seams only after DTOs are stable.
4. Avoid changing public command behavior in the same slice as mechanical moves.

Compatibility shim:

- Existing modules keep re-exporting moved items until downstream call sites are migrated.

Verify gate:

- `cargo check -p imp-core`
- focused tests for the extracted module
- existing worker/tool tests for touched behavior

Rollback:

- Restore re-export target or revert the single extraction commit.

## Phase 3 — runner and lease boundary

Goal: move toward mana-owned run/node/lease state while preserving `imp run <unit-id>`.

Order:
1. Add mana run/node schema in shadow mode.
2. Dual-write imp worker outcomes into compatibility run records.
3. Add shadow scheduling validation.
4. Introduce single-unit lease attach behind current `imp run`.
5. Only then narrow `mana run` into attach orchestration.

Compatibility shim:

- `imp run <unit-id>` remains the user-visible path.
- `mana run` may continue spawning imp during transition.

Verify gate:

- one-shot `imp run <unit-id>` smoke
- mana dry-run/shadow validation tests
- lease acquire/heartbeat/resolve unit tests once APIs exist

Rollback:

- Disable dual-write or lease attach flag and fall back to current `mana_worker` execution.

## Phase 4 — evidence and verification model

Goal: make durable evidence summaries explicit without persisting live runtime state.

Order:
1. Keep `ArtifactRef`, `VerifierResult`, and `EvidenceBundleRef` reference-first.
2. Add mappings from worker/verify outcomes into durable evidence summaries.
3. Promote summaries to mana only at checkpoint/verify/result boundaries.

Compatibility shim:

- Existing mana notes/logs remain accepted storage until a typed boundary has multiple consumers.

Verify gate:

- DTO serialization tests
- verify-result mapping tests
- one failed-check regression showing blocker and artifact references are preserved

Rollback:

- Stop emitting typed summaries and keep current notes/log output.

## Phase 5 — TUI, CLI, view, and RPC surfaces

Goal: share semantic UI/runtime events while keeping presentation local.

Order:
1. Treat `UserInterface` categories as the canonical request seam.
2. Adapt TUI-local `UiRequest` to the shared seam.
3. Normalize CLI/RPC stream serialization around shared runtime events.
4. Keep TUI focus/layout/render caches local.
5. Keep `imp view` navigation/filtering local.

Compatibility shim:

- Surface adapters translate old event/request shapes to the shared model until all callers migrate.

Verify gate:

- `cargo test -p imp-tui --lib`
- CLI output smoke for human mode
- JSON/RPC serialization tests for machine mode when touched

Rollback:

- Keep the old surface-local adapter and switch only the changed surface back.

## Phase 6 — cleanup and shim removal

Goal: remove compatibility shim layers only after replacement paths have passed repeated checks.

Removal rules:

- A shim must name its owner, replacement, and removal condition.
- Do not remove a shim in the same patch that introduces the replacement.
- Remove shims only after all downstream surfaces compile and focused behavior tests pass.

Verify gate:

- `cargo fmt --check`
- `cargo check --workspace`
- focused crate tests for touched surfaces

Rollback:

- Restore the shim re-export/adapter without reverting the replacement implementation.

## User-visible stability requirements

Throughout the migration:

- `imp chat` remains available.
- `imp print` remains available for one-shot model calls.
- `imp run <unit-id>` remains the preferred worker entrypoint.
- `mana run` compatibility does not disappear until attach orchestration is proven.
- Human output remains readable; machine output remains parseable and versioned when changed.

## Safe independently landable slices

- Docs and boundary maps.
- Pure DTO/helper additions with tests.
- Re-export-preserving module extractions.
- Shadow-mode run/lease records.
- Surface adapters that preserve old output.

## Slices that require preparatory adapters

- Lease-based execution.
- Changing `mana run` dispatch ownership.
- Shared event schema migration across CLI/TUI/RPC.
- Removing any public re-export or compatibility shim.

## Summary

The rebuild should proceed as a sequence of adapter-backed migrations: contract hardening, module decomposition, runner/lease shadowing, evidence promotion, surface normalization, then shim removal. The plan preserves existing paths and makes rollback local to each slice instead of requiring a flag-day rewrite.
