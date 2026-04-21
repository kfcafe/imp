# imp vNext

Status: proposed roadmap (source-of-truth draft)
Audience: `imp` maintainers
Scope: product/runtime direction for `~/tower/imp`

This document turns the Hermes comparison work (mana units `266`, `266.1`, `266.2`, `266.3`, epic `266.4`) and current Tower architecture direction into a ranked, concrete vNext roadmap.

It is intentionally opinionated about sequencing.

---

## Executive proposal

`imp vNext` is **not** a rewrite and **not** a Hermes clone.

`imp vNext` is: a reliability-first evolution of `imp` as the flagship Rust-native agent product on mana, with:
- stronger provider reliability and auth behavior,
- stronger context continuity across long-running work,
- first-class evaluation/observability for quality and trust,
- cleaner extension/hostability seams,
- selective (not sprawling) product-surface expansion after runtime seams are stable.

In architecture terms:
- `mana` remains the platform and lower substrate owner.
- `imp` remains the flagship agent product and agent runtime experience.
- vNext sharpens that boundary instead of blurring it.

---

## What imp vNext is

1. **A ranked execution plan layered on top of the rebuild plan** (`imp_rebuild_plan.md`).
2. **A product-quality push** focused on reliability, continuity, and operator trust.
3. **A concrete adoption strategy**: borrow specific proven patterns from Hermes where they fit Tower’s ownership model.
4. **A constrained scope**: ship core behavior improvements before optional broader surfaces.

## What imp vNext is not

- Not a ground-up rewrite.
- Not “Hermes parity” as a goal.
- Not moving durable substrate ownership from `mana` back into `imp`.
- Not reducing `imp` into a thin client.
- Not treating TypeScript extensions as already shipped behavior.

---

## Positioning relative to `imp_rebuild_plan.md`

`imp_rebuild_plan.md` defines the runtime refactor order (I0→I8).
This vNext roadmap defines **what product/runtime capabilities to emphasize in that order**.

In short:
- rebuild phases provide architectural preconditions,
- vNext phases prioritize user-visible reliability and quality outcomes.

---

## Hermes comparison: adoption stance

Use Hermes as a benchmark source, not a template.

### Borrow now (high fit)

- reliability patterns for provider fallback/failover behavior,
- better auth/session ergonomics,
- explicit eval loops and outcome instrumentation,
- cleaner hostability seams for embedding and controlled extension.

### Borrow later (conditional fit)

- expanded operator surfaces beyond terminal-first workflows,
- broader product packaging layers once core runtime evidence is strong.

### Avoid

- importing broad product breadth as phase-1 scope,
- coupling `imp` UX decisions to non-Tower ownership assumptions,
- collapsing `mana`/`imp` boundaries for speed.

---

## Where imp is already stronger (double down)

1. **Mana-native durable workflow integration** (task context, verify gates, handoff durability).
2. **Explicit `/ask → /plan → /work → /improve` workflow loop** as product behavior.
3. **Rust-native runtime + tooling coherence** with strong local execution ergonomics.
4. **Mode/capability direction** that can become real runtime policy (not prompt-only policy).
5. **File-native inspectability and operator legibility** across sessions and work history.

vNext should amplify these strengths instead of diluting them with broad, early surface-area expansion.

---

## Ranked roadmap

## Phase 0 — lock architecture preconditions (now)

Depends on: rebuild I0–I2

Outcome:
- shared contracts adoption and worker-runtime seam are treated as non-negotiable prerequisites,
- no new architectural drift into `imp-core` hot spots.

Why first:
- without stable boundaries, reliability features become rework.

Scope:
- align active work to I0/I1/I2 sequencing,
- define canonical mapping from vNext lanes to rebuild phases.

---

## Phase 1 — Provider resilience (first ship lane)

Primary lane: **Provider resilience**

Depends on: rebuild I1–I3

Outcome:
- resilient provider/auth behavior under degraded conditions,
- predictable fallback/failover semantics,
- clearer error classes and operator recovery paths.

Includes:
- auth-state clarity in CLI/TUI flows,
- bounded failover policy (opt-in/controlled, not silent chaos),
- provider capability/profile awareness for safer defaults,
- explicit telemetry around fallback decisions.

Why now:
- reliability incidents directly block day-to-day trust and worker uptime.

---

## Phase 2 — Context continuity (second ship lane)

Primary lane: **Context continuity**

Depends on: rebuild I3–I5

Outcome:
- stronger session recall, compaction quality, and selective continuity,
- better cold-start inheritance via durable mana handoff + focused local context.

Includes:
- improved compaction boundaries and handoff structure,
- retrieval/relevance tuning for long sessions,
- clearer split between session transcript data and durable task memory,
- operator-visible continuity controls in existing surfaces.

Why here:
- after baseline reliability, continuity is the highest leverage quality multiplier.

---

## Phase 3 — Evaluation and observability (third ship lane)

Primary lane: **Evaluation and observability**

Depends on: rebuild I6–I8 (plus output/evidence plumbing)

Outcome:
- measurable quality over time, not anecdotal confidence,
- evidence-oriented run outputs that are inspectable and comparable.

Includes:
- stable run/result instrumentation points,
- focused eval harnesses tied to real failure classes,
- normalized verification/evidence summaries for `mana` handoff,
- operator-readable diagnostics for “why this run succeeded/failed.”

Why here:
- this phase converts prior reliability/continuity work into compounding improvement loops.

---

## Phase 4 — Extension and hostability (fourth ship lane)

Primary lane: **Extension and hostability**

Depends on: rebuild I6–I8 and prior observability primitives

Outcome:
- cleaner seams for embedding `imp` runtime behavior safely,
- extension growth without surrendering host/runtime authority.

Includes:
- explicit host integration contracts (SDK/runtime boundaries),
- clearer guest/runtime capability boundaries,
- migration-ready extension architecture posture (without claiming TS extension GA today),
- improved cancellation and policy propagation across embedded/worker paths.

Why after eval:
- opening host/extension seams before observability and policy hardening increases risk.

---

## Phase 5 — Selective surface expansion (later, optional)

Primary lane: **Selective surface expansion**

Depends on: proven wins in phases 1–4

Outcome:
- targeted product-surface growth where evidence shows operator value,
- no uncontrolled expansion of UX breadth.

Includes (examples):
- deeper operator inspection flows,
- constrained new workflow views/surfaces,
- selective GUI-facing enhancements that consume stable runtime state.

Explicit guardrail:
- surface expansion is a multiplier on reliable core behavior, not a substitute for it.

---

## Why this order

1. **Boundary stability before feature ambition** (avoid rework).
2. **Reliability before convenience** (trust is table stakes).
3. **Continuity before breadth** (quality of ongoing work beats more UI).
4. **Measurement before expansion** (ship what we can prove improves outcomes).
5. **Hostability before ecosystem scale** (safe seams before bigger surface area).

---

## Adopt-now vs later

## Adopt now (commit in current planning horizon)

- Provider resilience/auth/failover lane.
- Context continuity/session recall/compaction lane.
- Evaluation and observability foundations tied to evidence handoff.

## Adopt next (after foundations prove out)

- Extension and hostability hardening as a first-class lane.

## Adopt later (optional, evidence-gated)

- Selective surface expansion.
- Any broader cross-product UX breadth not required for core agent quality.

---

## Concrete implementation lanes mapped to rebuild phases

- **Provider resilience/auth/failover** → mainly I1–I3, then hardening in I7.
- **Context continuity/session recall/compaction** → mainly I3–I5.
- **Evaluation and observability** → mainly I6–I8, with early instrumentation seeds in I3.
- **Extension/runtime hostability seams** → mainly I6–I8, after policy hardening signals.
- **Selective later product-surface expansion** → post-I8 and evidence-gated.

---

## Explicit non-goals

- Rewriting `imp` from scratch.
- Chasing Hermes breadth as an immediate roadmap target.
- Moving durable workflow ownership out of `mana`.
- Positioning `imp` as only a UI shell over mana.
- Announcing TypeScript extension maturity as current shipped behavior.
- Running broad, parallel architectural redesigns that conflict with rebuild sequencing.

---

## Success criteria for imp vNext

`imp vNext` is successful when:

- `imp` remains clearly the flagship Rust-native agent product on mana.
- Reliability under provider/auth disruption is materially better.
- Long-session/task continuity is materially better without context bloat.
- Evidence and verification outputs are more structured, comparable, and reusable.
- Hostability/extension seams are clearer without weakening policy boundaries.
- Later surface growth is selective and justified by measured outcomes.

---

## Maintenance note

Keep this file updated as roadmap truth changes.
If sequencing changes, update both:
1. phase ordering in this document,
2. mapping notes to `imp_rebuild_plan.md` I0–I8.
