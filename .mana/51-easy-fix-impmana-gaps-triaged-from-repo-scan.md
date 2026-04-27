---
id: '51'
title: Easy-fix imp+mana gaps triaged from repo scan
slug: easy-fix-impmana-gaps-triaged-from-repo-scan
status: open
priority: 2
created_at: '2026-04-09T06:19:57.978218Z'
updated_at: '2026-04-09T23:16:46.275004Z'
notes: |-
  ---
  2026-04-09T06:24:32.184686+00:00
  Initial decomposition from repo scan on 2026-04-09:
  1. Highest-leverage runtime fix: wire mana facts + project memory context into imp session-start prompt assembly so interactive imp is mana-aware by default.
  2. Highest-leverage mana fix: make the human review queue skip already-reviewed units and score risk from real diff evidence instead of placeholder empty file changes.
  3. Cheap trust fix: add dependency/security audit coverage to root CI.
  4. Cleanup follow-up: refresh stale architecture/docs text that still claims CI is missing or overstates API incompleteness.

  These were inspected in-repo before decomposition:
  - imp/crates/imp-core/src/builder.rs currently passes `facts: &[]`
  - mana/crates/mana-core/src/ops/memory_context.rs already assembles project memory context
  - mana/crates/mana-review/src/queue.rs has TODOs for reviewed-state skipping and real diff integration
  - .github/workflows/ci.yml exists at root but does not yet include dependency/security audit

  ---
  2026-04-09T08:25:58.001561+00:00
  Decomposition update after landing `51.1`: the facts-only slice is complete and verified. Deferred remainder has now been externalized as a follow-up unit for a distinct interactive prompt seam for mana project-memory status (`warnings` / `working_on` / `recent_work`) so the graph reflects the split between durable fact injection and status-like context injection.

  ---
  2026-04-09T08:28:38.683409+00:00
  Recovered missing durable record for the landed mana-review queue fix by creating replacement child `51.6` under this feature and immediately closing it with verify evidence. Context: earlier status output referenced child `51.2`, but the underlying root `.mana` file for that unit was no longer present/resolvable. `51.6` now captures the implemented delta: queue skips persisted reviews via `state::has_review`, uses `diff::compute(..., checkpoint)` for file stats when available, and degrades to empty stats when checkpoint/diff evidence is unavailable without failing queue generation. Verify evidence captured in the replacement unit: `cargo test -p mana-review && cargo test -p mana-cli review`.

  ---
  2026-04-09T08:30:03.650258+00:00
  Easy-fix decomposition update: 51.1 landed the first session-start mana context slice by wiring bounded relevant facts into imp interactive prompt assembly. Follow-on dynamic project-memory status context is intentionally split into 51.5 so facts remain a clean Layer 4 seam and warnings / working-on / recent-work can land in a separate prompt layer.

  ---
  2026-04-09T08:33:34.714499+00:00
  Visible decomposition update after landing `51.1`: root graph now records the split explicitly. `51.1` is the closed, verified fact-injection slice for interactive imp startup. `51.5` remains the open follow-up for a distinct prompt seam covering compact dynamic mana project-memory status (`warnings` / `working_on` / `recent_work`) without polluting the fact layer.

  ---
  2026-04-09T08:34:02.152919+00:00
  Durable graph note: native mana `list` shows closed child `51.1`, but direct `show/update` resolution for `51.1` currently fails in root scope. I therefore recorded the landed/deferred decomposition delta on this parent (`51`) and on follow-up child `51.5` instead of fabricating a replacement closure record. Practical meaning remains the same in the graph: the fact-injection slice is treated as landed, and the remaining dynamic project-memory status layer is explicitly tracked as follow-up work in `51.5`.

  ---
  2026-04-09T12:03:54.820975+00:00
  Durable follow-up captured from docs cleanup unit 51.4:
  - Fact 62 records the inspected current CI reality: root and mana-local workflows exist, cover build/test/lint/format (plus mana-local MSRV), and still lack dedicated dependency/security audit jobs.
  - Fact 63 records the inspected current mana-core API reality: api/mod.rs already exposes broad discovery/query/mutation/orchestration helpers; the remaining embedding gap is uneven top-level wrapper coverage plus no single stable full-spawn library entry point.
  These facts are intended to anchor future docs cleanup and easy-fix prioritization without requiring reconstruction from chat history.

  ---
  2026-04-09T12:05:35.582605+00:00
  Execution note for completed child unit 51.3 (root CI dependency/security audit coverage):
  - Inspected root `.github/workflows/ci.yml` and confirmed baseline jobs were check/test/clippy/format only.
  - Chose the smallest useful root-level security signal for the active Tower workspace: one Rust dependency-vulnerability audit job rather than broader default secret scanning.
  - Implemented `dependency-audit` / `Dependency Audit` in `.github/workflows/ci.yml` using maintained action `rustsec/audit-check@v2.0.0` against the root workspace/root Cargo.lock.
  - Added inline workflow comments documenting the intentional scope limit: dependency audit for the active root Rust workspace, not broad secret/repo scanning in default CI.
  - Verified with the child unit gate and a local YAML parse sanity check.

  This note externalizes the durable decomposition/result because closed unit 51.3 was not accepting append/update in the current mana state.

  ---
  2026-04-09T12:05:51.469663+00:00
  Visible decomposition delta from child 51.3: root CI dependency/security coverage was intentionally implemented as one narrow root-workspace dependency audit rather than a broad security sweep. The chosen slice adds `dependency-audit` / `Dependency audit (RustSec)` to `.github/workflows/ci.yml` using `rustsec/audit-check@v2.0.0`, scoped by the root Cargo.lock and documented as covering the active mana + imp workspace members rooted in Tower. Broader secret scanning/full-repo security checks remain intentionally out of default root CI for now to preserve signal and runtime cost.

  ---
  2026-04-09T23:16:46.274998+00:00
  Visible decomposition delta from closed fact 63: repo inspection confirmed `mana-core::api` is already broad enough that the remaining work is no longer 'add an API' in the abstract. The durable next slice is now tracked explicitly in child `51.6`: decide, from inspected ownership seams, whether to (a) promote a small set of remaining embedding-relevant `ops::*` helpers to stable `api::*` wrappers, or (b) leave those helpers lower-level and instead define one stable full-spawn/library entry point boundary. This keeps the next embedding decision visible in root scope instead of living only in chat or doc caveats.
labels:
- triage
- imp
- mana
- easy-fix
kind: epic
feature: true
---

Capture a short, durable backlog of smaller/high-leverage gaps surfaced by repo inspection on 2026-04-09. Current evidence points to: (1) imp session-start prompt assembly still passes `facts: &[]` in `imp/crates/imp-core/src/builder.rs`, despite mana facts/memory-context machinery existing; (2) mana review queue still includes already-reviewed units and uses placeholder empty file-change data in `mana/crates/mana-review/src/queue.rs` even though `mana-review/src/state.rs` and `mana-review/src/diff.rs` exist; (3) human review persistence in `mana/crates/mana-cli/src/commands/review_human.rs` has annotation support stubbed to `vec![]`; (4) mana-core public API still lacks the mutation/orchestration coverage called out in `mana/ARCHITECTURE.md`; (5) no repo CI/security audit automation is present, per `mana/ARCHITECTURE.md`. Use this epic to spin out small, concrete jobs if we choose to implement any of these next.
