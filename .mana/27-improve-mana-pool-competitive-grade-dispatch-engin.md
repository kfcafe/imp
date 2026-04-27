---
id: '27'
title: 'Improve mana-pool: competitive-grade dispatch engine'
slug: improve-mana-pool-competitive-grade-dispatch-engin
status: open
priority: 2
created_at: '2026-03-24T01:53:17.534023Z'
updated_at: '2026-04-09T18:04:06.464376Z'
notes: |-
  ---
  2026-04-09T05:24:13.642161+00:00
  2026-04-09 durable handoff from unit 27.1: the verified slice that closed was narrow `mana-pool` dispatch coverage, not a new pool-side policy surface. In this checkout `mana-pool` is still effectively standalone (no observed mana-cli wiring into `mana-pool` APIs), and archived unit 27.1 had no title/description beyond the slug/verify. The successful implementation path was to add/verify explicit `budget`-named tests in `mana/crates/mana-pool/src/dispatch.rs` so the unit gate exercised concrete behavior directly: `budget_enforces_max_concurrent_limit` proves dispatch never exceeds `max_concurrent`, and `budget_circuit_breaker_stops_new_spawns_after_failure` proves no new units spawn after first failure when `keep_going=false`. Exact verify gate now passes: `cd /Users/asher/tower && cargo test -p mana-pool budget -- 2>&1 | grep 'test result' | grep -v '0 passed'`. For future 27.x work: inspect staged/worktree state before editing, because this checkout already contained the matching `dispatch.rs` test additions when triaged.

  ---
  2026-04-09T06:24:19.997546+00:00
  2026-04-09 keep-going policy boundary note:
  - Result of 27.7 policy analysis: do not move raw confidence scoring into mana-pool.
  - Current boundary should remain: imp owns confidence computation, evidence interpretation, visibility of the mana delta, and the local decision to continue into the next small bounded step.
  - mana-pool owns only mechanical dispatch/scheduling once work is eligible: dependency readiness, ordering, path conflicts, memory/capacity limits, retry context, batch-verify coordination, and binary keep_going failure policy.
  - If dispatch-layer participation is needed later, the extension should be a typed durable eligibility/risk contract (for example continue_eligible/autonomy_eligible + reason codes + decision/risk/review gating), not an opaque model confidence score.
  - Unresolved consequential decisions, weak verify, and review-required/risky work should be handled as explicit gating inputs before dispatch, not as pool-internal confidence math.

  ---
  2026-04-09T06:32:30.418952+00:00
  2026-04-09 layered-verify implementation record for closed child 27.6:
  - Scope/ownership: root-scope cross-project slice because the change spans shared mana unit frontmatter in `mana-core` and deferred verification behavior in `mana-pool`.
  - Decomposition executed:
    1. Add shared frontmatter field `verify_fast: Option<String>` to `mana-core/src/unit/mod.rs` (`Unit` + `UnitWire`) so unit YAML can read/write an optional fast preflight verify command.
    2. Add `verify_fast: Option<String>` to `mana-pool/src/types.rs` `DispatchUnit` so dispatch has the preflight command available alongside `verify_command`.
    3. Change `mana-pool/src/dispatch.rs` layered batch-verify behavior to run grouped `verify_fast` commands first, skip grouped full `verify_command` for units whose fast verify failed, and mark those units failed with message `fast verify failed`.
    4. Preserve direct full-verify behavior for units with no `verify_fast`.
    5. Add focused tests covering all three behaviors.
  - Tests added in `mana/crates/mana-pool/src/dispatch.rs`:
    - `layered_verify_fast_fails_skips_full`
    - `layered_verify_fast_passes_runs_full`
    - `layered_verify_no_fast_runs_full_directly`
  - Shared serde tests updated in `mana-core/src/unit/mod.rs` so `verify_fast` round-trips and is omitted when unset.
  - Verify evidence:
    - exact unit gate passed: `cd /Users/asher/tower && cargo test -p mana-pool layered_verify -- 2>&1 | grep 'test result' | grep -v '0 passed'`
    - observed: `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 24 filtered out`
    - implementation-time safety check also passed: `cargo check -p mana-core`

  ---
  2026-04-09T07:20:33.553330+00:00
  2026-04-09 recovery decision for former 27.1: metadata repair path selected. Confirmed in this checkout that the old verify gate maps to existing narrow dispatch tests in `mana/crates/mana-pool/src/dispatch.rs`: `budget_enforces_max_concurrent_limit` and `budget_circuit_breaker_stops_new_spawns_after_failure`. The exact historical gate passes (`cargo test -p mana-pool budget ...` → 2 passed). Do not interpret former 27.1 as an unfinished new pool-side policy surface; explicit circuit-breaker logic still belongs to close/failure handling in `mana-core` / `mana-cli`. If future product intent wants a real pool-side budget/circuit-breaker policy, create a fresh child under 27 with explicit behavior, files, and tests instead of reusing 27.1.

  ---
  2026-04-09T07:20:52.854722+00:00
  2026-04-09 27.1 scope recovery decision: choose the metadata-repair path, not a new pool-side feature path.
  - Current checkout already contains the narrow `mana-pool` dispatch coverage that matched former 27.1's visible verify gate.
  - Verified directly: `cd /Users/asher/tower && cargo test -p mana-pool budget -- 2>&1 | grep 'test result' | grep -v '0 passed'` -> `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out`.
  - Exact tests in `mana/crates/mana-pool/src/dispatch.rs`:
    - `budget_enforces_max_concurrent_limit`
    - `budget_circuit_breaker_stops_new_spawns_after_failure`
  - Interpretation: former 27.1 should be treated as already-completed narrow dispatch-test coverage proving concurrency-budget enforcement and stop-on-failure/no-new-spawns behavior when `keep_going=false`.
  - Boundary confirmation from current code context: circuit-breaker attempt/close policy lives in `mana/crates/mana-core/src/ops/close.rs` and `mana/crates/mana-cli/src/commands/close/failure.rs`, not as a separate `mana-pool` dispatch policy surface.
  - Therefore do not reopen or retry the inaccessible old 27.1 body. If future product intent is a real new pool-side budget/circuit-breaker policy, create a fresh child under 27 with explicit files, behavior, and a concrete verify gate before implementation.

  ---
  2026-04-09T07:21:09.382876+00:00
  2026-04-09 visible delta from closed child 27.8:
  - Recovery decomposition is now explicit in mana rather than only chat.
  - Former `27.1` should be read as a metadata-repair / narrow test-coverage slice, not an unimplemented pool-side runtime policy.
  - Verification anchor remains the exact historical gate: `cd /Users/asher/tower && cargo test -p mana-pool budget -- 2>&1 | grep 'test result' | grep -v '0 passed'`.
  - Concrete tests in `mana/crates/mana-pool/src/dispatch.rs`: `budget_enforces_max_concurrent_limit`, `budget_circuit_breaker_stops_new_spawns_after_failure`.
  - If future intent changes, create a fresh child under `27` with explicit behavior, file targets, and test names instead of trying to resurrect inaccessible archived `27.1` metadata.

  ---
  2026-04-09T07:21:32.809082+00:00
  2026-04-09 durable decomposition for former 27.1 recovery slice:
  1. Inspect `mana/crates/mana-pool/src/dispatch.rs` for `budget`-named tests instead of inferring scope from the missing archived body.
  2. Re-run the visible historical gate `cargo test -p mana-pool budget -- 2>&1 | grep 'test result' | grep -v '0 passed'` to confirm whether the narrow coverage already exists in this checkout.
  3. Use `mana-core` / `mana-cli` close-failure code only as ownership context: confirm circuit-breaker attempt policy lives there, not in a new `mana-pool` dispatch feature.
  4. If the gate passes and the tests match the recovered behavior, repair metadata on parent `27` and stop; do not recreate ambiguous 27.1 instructions.
  5. Only if future product intent wants new pool-side runtime behavior, create a fresh child under `27` with explicit files, behavior, and targeted tests before any implementation.

  ---
  2026-04-09T08:54:46.448121+00:00
  2026-04-09 durable follow-up from archived 27.11 (typed autonomy-eligibility contract):
  - Current inspected code does not prove a real mana-pool-side need for autonomy eligibility inputs beyond imp-local continuation.
  - `mana-pool` remains mechanical dispatch only: dependency/path/resource scheduling, retry context, verify grouping, and coarse `keep_going` stop-on-failure behavior.
  - `mana run` unresolved decisions are still surfaced as caller/operator warnings, not pool-native typed gates.
  - imp remains the owner of confidence/evidence interpretation and visible between-turn continuation policy.
  - If a future cross-run/daemon/handoff need emerges, the pool-facing contract should be a narrow typed eligibility/disposition record only, centered on: `continue_eligible`, `human_pause_required`, `review_required`, decision-block severity, verify/evidence quality bands, risk band, explicit reason codes, and provenance/freshness.
  - Anti-goals preserved: no opaque confidence scalar in pool types, no duplicate imp-local confidence engine in `mana-pool`, and no invisible autonomous continuation.

  ---
  2026-04-09T08:58:04.290505+00:00
  2026-04-09 durable decomposition added after closing 27.10 autonomy-gating contract spec:
  - 27.13 `Add canonical autonomy disposition and blocker vocabulary to mana-core`
  - 27.14 `Define attempt-scoped autonomy observation record for imp-visible evidence`
  - 27.15 `Make unresolved decisions a canonical scheduler-visible blocker in run planning`
  - 27.17 `Model review/approval state and typed verify posture for scheduler gating`
  - 27.18 `Normalize retry history into typed attempt-pressure for autonomy gating`
  - 27.19 `Project autonomy disposition onto DispatchUnit without adding pool policy`

  Dependency intent:
  - Upstream canonical contract and evidence normalization live in mana-core / imp first.
  - `mana-pool` projection work (27.19) depends on those upstream fields existing so the pool consumes only an explicit derived disposition.
  - No pool-side keep-going implementation should open until these units land; unresolved decisions, review/approval state, verify posture, visibility/provenance, and attempt pressure must all be durable and typed first.

  ---
  2026-04-09T12:03:29.415869+00:00
  2026-04-09 stale-fact validation decomposition for former fact 47 / historical 27.1 gate drift:
  1. Inspect `mana/crates/mana-pool/src/dispatch.rs` for current `budget`-named tests before trusting the old negative fact.
  2. Re-run the exact historical filter `cd /Users/asher/tower && cargo test -p mana-pool budget ...` and treat its output as primary evidence.
  3. If matching `budget` tests exist and pass, do not mutate code to recreate the old `0 passed` condition; treat the negative fact as stale because the repo advanced.
  4. Use archived `27.1` plus existing parent-27 notes as the durable explanation that `budget` now maps to narrow dispatch coverage (`budget_enforces_max_concurrent_limit`, `budget_circuit_breaker_stops_new_spawns_after_failure`).
  5. Preserve the delta by replacing the stale negative assertion with a positive fact unit grounded in the current verify output, rather than leaving the conclusion only in chat.
  Evidence in this checkout: `cargo test -p mana-pool budget` now reports `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out`.

  ---
  2026-04-09T12:03:41.722037+00:00
  2026-04-09 visible delta from closed fact 47 (former negative no-match verify claim for 27.1): resolved as root-mana metadata repair, not new `mana-pool` implementation work.

  Recovery decomposition executed:
  1. Run the historical filtered gate first to test the claim, not the slug: `cd /Users/asher/tower && cargo test -p mana-pool budget -- --nocapture` showed 2 matching tests already exist.
  2. Inspect `mana/crates/mana-pool/src/dispatch.rs` and confirm the exact coverage names: `budget_enforces_max_concurrent_limit` and `budget_circuit_breaker_stops_new_spawns_after_failure`.
  3. Compare current root fact metadata with archived 27.1 metadata and recover the intended positive verify shape.
  4. Repair the root fact verify line from the stale/inverted form `cargo test -p mana-pool budget -- 2>&1 | rg -q '0 passed'` to the narrow positive gate `cd /Users/asher/tower && cargo test -p mana-pool budget -- 2>&1 | grep 'test result' | grep -v '0 passed'`.
  5. Re-run that exact gate before closure; observed output: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out`.

  Boundary/ownership reminder for future 27.x work:
  - This slice does **not** justify reopening a new pool-side budget/circuit-breaker feature.
  - In the current repo, the relevant `mana-pool` behavior is already covered by narrow dispatch tests; the only stale part was root `.mana` metadata.
  - If future product intent wants a real new pool-side policy surface, create a fresh child under `27` with explicit behavior, file targets, and test names instead of reusing former 27.1 / fact 47.

  ---
  2026-04-09T13:20:38.301922+00:00
  2026-04-09 durable decomposition from closed child 27.12 (`mana show` archived lookup repair):
  - Scope stayed intentionally narrow to `mana show <id>` resolution only; no list/tree/archive UX broadening and no close/archive semantic changes.
  - Implementation plan executed:
    1. In `mana/crates/mana-core/src/ops/show.rs`, preserve active-unit precedence by calling `find_unit_file(...)` first.
    2. If active lookup misses, fall back immediately to `find_archived_unit(...)` so archived IDs like `27.8` resolve after close.
    3. Keep the return shape unchanged (`GetResult { unit, path }`) so `mana-cli/src/commands/show.rs` rendering continues to work without CLI logic changes.
    4. Add a focused core regression test with an archived-only fixture under `.mana/archive/YYYY/MM/` asserting `ops::show::get("1")` succeeds, returns the archived path, and preserves archived unit data.
    5. Add a focused CLI regression test asserting `cmd_show("1", false, false, false, ...)` succeeds when unit `1` exists only in the archive tree.
    6. Preserve not-found behavior for IDs absent from both active and archived trees.
  - Verify gate used for closure and passed in this checkout: `cd /Users/asher/tower && cargo test -p mana-core ops::show -- --nocapture && cargo test -p mana-cli show -- --nocapture`.
  - Repo-state note: after verification, `git status` was clean while file contents already matched the intended fallback + tests, so the closure evidence is the current inspected file state plus the passing verify gate, not a pending diff.

  ---
  2026-04-09T13:48:26.409783+00:00
  2026-04-09 durable follow-up after closing 27.13 (canonical autonomy schema/vocabulary):
  - 27.13 is now complete: mana-core stores the canonical typed scheduler-facing autonomy disposition on `Unit.autonomy_disposition` and preserves YAML omission when unset.
  - Immediate downstream sequence stays explicit and unchanged:
    1. 27.14 defines/normalizes attempt-scoped autonomy observation evidence.
    2. 27.20 implements the single canonical evaluator in mana-core that derives `Unit.autonomy_disposition` from durable facts/evidence rather than leaving schedulers to recompute policy.
    3. 27.21 projects only the explicit derived disposition onto run/planning/pool-facing structures without adding new dispatch behavior.
  - Boundary reminder: no raw confidence field, no pool-side blocker inference from prose or verify text, and no dispatch/keep-going changes before 27.20 + 27.21 land.
  - Root-graph visibility: this decomposition now lives in mana, not only in the execution transcript.

  ---
  2026-04-09T13:51:57.848202+00:00
  27.16 follow-on type-placement plan has been externalized across child autonomy units. Global decision from the doc: keep the canonical autonomy vocabulary in mana-core rather than tower-contracts for the first migration, because this is a durable unit/evaluator contract owned by mana-core and mana-review currently depends on mana-core. Pool and imp should import the mana-core-owned types rather than creating duplicate shared or pool-local autonomy enums.

  ---
  2026-04-09T18:03:57.297747+00:00
  2026-04-09 post-close validation for archived 27.11:
  - Re-ran the archive-level verify gate against `.mana/archive/2026/04/27.11-specify-typed-autonomy-eligibility-contract-only-i.md`.
  - Exact command passed: `cd /Users/asher/tower && test -f .mana/archive/2026/04/27.11-specify-typed-autonomy-eligibility-contract-only-i.md && rg -q 'No pool-side autonomy contract is needed today' .mana/archive/2026/04/27.11-specify-typed-autonomy-eligibility-contract-only-i.md && rg -q 'continue_eligible: bool' .mana/archive/2026/04/27.11-specify-typed-autonomy-eligibility-contract-only-i.md && rg -q 'provenance' .mana/archive/2026/04/27.11-specify-typed-autonomy-eligibility-contract-only-i.md`.
  - Durable conclusion remains unchanged: current repo evidence supports no present mana-pool-side need for typed autonomy eligibility inputs; if a future cross-run scheduler need appears, the pool-facing shape should stay a narrow typed eligibility/disposition record rather than raw confidence.

  ---
  2026-04-09T18:04:06.464369+00:00
  2026-04-09 current boundary check for cross-project confidence/continuation ownership:
  - Verified in `mana/crates/mana-pool/src/types.rs` that pool-facing dispatch config still exposes only coarse mechanical scheduler policy such as `keep_going`; no pool-side `confidence`, `autonomy_eligible`, or `continue_eligible` fields are present in inspected `mana-pool` types/dispatch code.
  - Verified in `mana/crates/mana-cli/src/commands/run/mod.rs` that unresolved decisions remain a CLI/operator warning + confirmation path via `collect_decision_warnings` / `confirm_dispatch_with_decisions`, not a `mana-pool` scheduling contract.
  - Verified in `imp/crates/imp-core/src/agent.rs` that the implemented confidence-based continuation hook remains imp-local behind `ContinuePolicy`, `should_queue_confidence_continue_follow_up`, and `confidence_continue_follow_up_text`.
  - This keeps the current architectural decomposition explicit in root mana: `mana-pool` currently consumes only mechanical dispatch metadata; imp owns the implemented confidence-based continuation behavior.
  - Verification anchor used for this check: `cd /Users/asher/tower && rg -q 'pub keep_going: bool' mana/crates/mana-pool/src/types.rs && ! rg -q 'confidence|autonomy_eligible|continue_eligible' mana/crates/mana-pool/src/types.rs mana/crates/mana-pool/src/dispatch.rs && rg -q 'collect_decision_warnings|confirm_dispatch_with_decisions' mana/crates/mana-cli/src/commands/run/mod.rs && rg -q 'ContinuePolicy|confidence_continue_follow_up_text|should_queue_confidence_continue_follow_up' imp/crates/imp-core/src/agent.rs`
verify: cargo check -p mana-pool
kind: job
feature: true
decisions:
- 'Former unit 27.1 (''budget-and-circuit-breaker-for-dispatch'') is currently inconsistent: list/tree show it as closed, but its verify gate targets no matching mana-pool tests and its durable description is missing. Before any implementation retry, treat scope recovery/clarification as required work.'
- Interpret archived unit 27.1 narrowly as explicit `mana-pool` dispatch test coverage aligned to its verify gate, not as evidence of an unwired new pool-side budget/circuit-breaker runtime policy surface. If future work wants real runtime behavior here, create a new child under 27 with explicit CLI/pool integration scope and verify gate instead of reopening 27.1 on the slug alone.
- '2026-04-09 formal recovery decision for missing archived child 27.1: treat the former work as already-completed narrow `mana-pool` dispatch test coverage, not as a latent request for new pool-side runtime budget/circuit-breaker policy. Basis: `mana/crates/mana-pool/src/dispatch.rs` currently contains `budget_enforces_max_concurrent_limit` and `budget_circuit_breaker_stops_new_spawns_after_failure`; the historical gate `cd /Users/asher/tower && cargo test -p mana-pool budget -- 2>&1 | grep ''test result'' | grep -v ''0 passed''` passes; and explicit circuit-breaker ownership remains in `mana/crates/mana-core/src/ops/close.rs` and `mana/crates/mana-cli/src/commands/close/failure.rs`. If future product intent wants a real pool-side policy surface, create a new child under 27 with explicit files, behavior, and tests rather than attempting to reuse inaccessible 27.1 metadata.'
- Former 27.1 recovery should be handled as a metadata-repair exercise unless new explicit product intent says otherwise.
- 'Grounding: inspected `mana-pool` scheduling remains mechanical (`types.rs`, `dispatch.rs`), `mana run` still handles unresolved decisions as operator warnings/confirmation, and imp owns visible confidence-based continuation in `imp-core/src/agent.rs`. Unresolved decisions, reviewability, verify quality, evidence quality, risk, provenance, and freshness should become explicit gating inputs only if a real cross-run dispatch need is proven.'
- 'After inspecting current `mana-pool`, `mana run`, and `imp` continuation code, preserve the current boundary: do not add pool-side autonomy/confidence inputs now. If a real cross-run scheduling need later emerges, the only valid pool-facing extension is a narrow typed eligibility/disposition record with explicit blocker reason codes, review/human-pause flags, verify/evidence quality bands, risk band, and provenance/freshness. imp retains ownership of confidence and evidence interpretation.'
- '2026-04-09 recovery rule from closed fact 47 / former 27.1 verify drift: when an archived or recovered 27.x slice has ambiguous body metadata, first verify the visible narrow test gate against the checkout and inspect the named tests before proposing new `mana-pool` behavior. If the targeted coverage already exists, treat the work as metadata repair and record the corrected gate in root mana; only create a new child if product intent explicitly asks for additional pool-side behavior beyond the existing dispatch tests.'
- Verified by closed fact 48 against current repo state.
---
