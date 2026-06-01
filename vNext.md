# imp vNext

Status: proposed roadmap (source-of-truth draft)
Audience: `imp` maintainers
Scope: product/runtime direction for `~/imp`

This document turns the Hermes comparison work (mana units `266`, `266.1`, `266.2`, `266.3`, epic `266.4`) and the recent imp documentation set into a ranked, concrete vNext roadmap.

It is intentionally opinionated about sequencing.

It should be read alongside:
- `README.md`
- `imp_rebuild_plan.md`
- `imp_ontology.md`
- `docs/proposals/guest-runtime-extension-substrate.md`
- `docs/proposals/script-tool-boundaries-and-policy.md`
- `docs/proposals/imp-memory-architecture-and-mana-boundary.md`
- `../tower/docs/rebuild/imp-shared-runtime-startup-map.md`
- `../tower/docs/rebuild/imp-command-grammar.md`
- `../tower/docs/rebuild/imp-delegation-tool-and-runtime.md`
- `../tower/docs/rebuild/mana-imp-ownership-boundary.md`

---

## Executive proposal

`imp vNext` is **not** a rewrite and **not** a Hermes clone.

`imp vNext` is: a reliability-first evolution of `imp` as the flagship Rust-native agent product on mana, with:
- a cleaner shared runtime/bootstrap path across CLI, TUI, RPC, and headless worker surfaces,
- stronger provider reliability and auth behavior,
- stronger context continuity across long-running work,
- first-class evaluation/observability for quality and trust,
- a clearer guest-runtime extension substrate under Rust host control,
- a CLI-first product shape with explicit worker/runtime and fullscreen surfaces,
- selective (not sprawling) product-surface expansion after runtime seams are stable.

In architecture terms:
- `mana` remains the platform and lower substrate owner.
- `imp` remains the flagship agent product and agent runtime experience.
- `mana` owns the structured execution bundle and durable work truth.
- `imp` owns final prompt assembly, live execution, and the human-facing product loop.
- vNext sharpens that boundary instead of blurring it.

---

## What imp vNext is

1. **A ranked execution plan layered on top of the rebuild plan** (`imp_rebuild_plan.md`).
2. **A product-quality push** focused on reliability, continuity, operator trust, and runtime coherence.
3. **A concrete adoption strategy**: borrow specific proven patterns from Hermes where they fit Tower’s ownership model.
4. **A convergence plan** around the recent imp docs: CLI-first shell direction, canonical worker runtime, shared startup/bootstrap, guest-runtime substrate, script-tool separation, and layered memory boundaries.
5. **A constrained scope**: ship core behavior improvements before optional broader surfaces.

## What imp vNext is not

- Not a ground-up rewrite.
- Not “Hermes parity” as a goal.
- Not moving durable substrate ownership from `mana` back into `imp`.
- Not reducing `imp` into a thin client.
- Not treating TypeScript extensions as already shipped behavior.
- Not letting the TUI define runtime boundaries that CLI/RPC/headless should share.
- Not merging script execution, durable extensions, and worker execution into one concept.

---

## Positioning relative to `imp_rebuild_plan.md`

`imp_rebuild_plan.md` defines the runtime refactor order (I0→I8).
This vNext roadmap defines **what product/runtime capabilities to emphasize in that order**.

In short:
- rebuild phases provide architectural preconditions,
- vNext phases prioritize user-visible reliability and quality outcomes,
- recent docs refine the intended product shape and boundary language so the roadmap does not drift back toward old ambiguities.

---

## Recent documentation that changes the roadmap shape

The Hermes comparison was useful, but the more important update is that recent imp documentation already narrows several key design questions.

### 1. CLI-first is now a product-direction input, not an optional UX taste

From `../tower/docs/rebuild/imp-command-grammar.md`:
- long-term target: `imp == imp chat`
- `imp tui` becomes the explicit fullscreen path
- `imp run <unit-id>` remains the canonical machine-facing single-unit runtime
- `imp view ...` remains the browse/inspection surface

Implication for vNext:
- command-surface and runtime-boundary work must assume a CLI-first default product shape,
- but this should not cause a premature TUI rewrite.

### 2. Shared runtime bootstrap is a first-order architectural seam

From `../tower/docs/rebuild/imp-shared-runtime-startup-map.md`:
- `ImpSession::create()` is already the strongest canonical startup path,
- RPC and TUI still manually duplicate provider/auth/model/runtime assembly,
- the first high-value extraction seam is a shared runtime bootstrap consumed by `ImpSession`, RPC, and TUI.

Implication for vNext:
- runtime/bootstrap consolidation belongs in the earliest precondition phase, not as a late cleanup task.

### 3. `imp run` and the native `mana` tool already define the intended worker/orchestration split

From `docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md` and `../tower/docs/rebuild/imp-delegation-tool-and-runtime.md`:
- native `mana` inside imp is the first-class orchestration UX,
- `mana run` remains orchestration/dispatch,
- `imp run` is the canonical single-unit worker runtime,
- durable child work should route through mana-backed execution, not a second ad hoc orchestration substrate.

Implication for vNext:
- worker-runtime clarity is part of the core roadmap, not a side-note.

### 4. Extension direction is sharper than “TS later, Lua now”

From `docs/proposals/guest-runtime-extension-substrate.md` and `docs/proposals/script-tool-boundaries-and-policy.md`:
- Rust remains the authority boundary,
- Lua should be treated as the first shipped guest runtime, not the whole extension architecture,
- durable extensions, ephemeral scripting, and native worker execution must stay separate,
- host policy must remain authoritative.

Implication for vNext:
- the right phase-4 story is a **guest-runtime substrate + policy-hardening** story,
- not generic plugin sprawl,
- and not premature TypeScript-extension productization.

### 5. Memory and ownership boundaries are more explicit now

From `docs/proposals/imp-memory-architecture-and-mana-boundary.md` and `../tower/docs/rebuild/mana-imp-ownership-boundary.md`:
- imp has layered memory scopes,
- `mana` owns project-durable work memory and future synthesized knowledge,
- `imp` owns personal/global memory, live context assembly, and final prompt rendering,
- `mana` should provide structured execution bundles, not canonical final prompts.

Implication for vNext:
- context continuity work must improve retrieval and compaction without collapsing durable mana state into session memory.

### 6. Deep review findings make policy hardening part of the roadmap, not just hygiene

From `IMP_DEEP_REVIEW.md`:
- Lua currently bypasses some policy boundaries,
- cancellation propagation is weak,
- provider/auth resolution is duplicated,
- shell backend behavior and config trust are not yet aligned.

Implication for vNext:
- policy hardening and canonical runtime resolution should appear explicitly in the roadmap, especially in phases 0, 1, and 4.

---

## Hermes comparison: adoption stance

Use Hermes as a benchmark source, not a template.

### Borrow now (high fit)

- reliability patterns for provider fallback/failover behavior,
- better auth/session ergonomics,
- explicit eval loops and outcome instrumentation,
- clearer hostability seams for embedding and controlled extension.

### Borrow later (conditional fit)

- expanded operator surfaces beyond terminal-first workflows,
- broader product packaging layers once core runtime evidence is strong.

### Avoid

- importing broad product breadth as phase-1 scope,
- coupling `imp` UX decisions to non-Tower ownership assumptions,
- collapsing `mana`/`imp` boundaries for speed,
- copying Hermes plugin/gateway breadth before imp’s host policy and runtime seams are trustworthy.

---

## Where imp is already stronger (double down)

1. **Mana-native durable workflow integration** (task context, verify gates, handoff durability).
2. **Explicit `/ask → /plan → /work → /improve` workflow loop** as product behavior.
3. **Rust-native runtime + tooling coherence** with strong local execution ergonomics.
4. **Mode/capability direction** that can become real runtime policy (not prompt-only policy).
5. **File-native inspectability and operator legibility** across sessions and work history.
6. **A clearer current worker-runtime story** than many agent products once `imp run` and shared startup seams are made canonical.

vNext should amplify these strengths instead of diluting them with broad, early surface-area expansion.

---

## Cross-cutting product constraints

These are not separate roadmap phases; they are constraints that should hold across all phases.

### A. CLI-first default, explicit fullscreen

The target shape remains:

```bash
imp        == imp chat
imp tui    == explicit fullscreen UI
imp run    == canonical worker/runtime path
imp view   == inspection/browse surface
```

Do not let fullscreen/TUI assumptions distort runtime architecture or worker contracts.

### B. `imp run` stays Rust-native and canonical for one unit

Do not move worker execution into guest runtimes.
Do not invent a second durable child-work substrate inside imp.
Strengthen the existing worker path instead.

### C. Native `mana` remains the orchestration UX inside imp

The vNext path is:

```text
imp native mana tool
  -> mana durable orchestration
    -> imp run worker runtime
```

### D. Final prompt assembly stays in imp

`mana` should provide structured execution bundles and durable context.
`imp` should continue owning final prompt wording, packing, and token budgeting.

### E. Guest-runtime substrate is extension architecture; script tool is separate

Keep these concepts distinct:
- native worker/runtime execution,
- ephemeral run-scoped scripting,
- durable packaged extensions.

---

## Ranked roadmap

## Phase 0 — lock shared runtime/bootstrap, worker-runtime, and product-shape preconditions (now)

Primary lane: **shared runtime/bootstrap + worker-runtime consolidation**

Depends on: rebuild I0–I2

Outcome:
- shared contracts adoption, shared runtime/bootstrap, and the worker-runtime seam are treated as non-negotiable prerequisites,
- CLI/TUI/RPC/headless do not keep drifting apart in startup/runtime behavior,
- the product shape is anchored around CLI-first + explicit worker/view/fullscreen surfaces.

Includes:
- shared runtime/bootstrap extraction from the current `ImpSession` path,
- canonical provider/auth/model resolution reused across surfaces,
- explicit reinforcement of `imp run` as the canonical single-unit worker contract underneath native `mana` orchestration,
- command-surface/documentation alignment around CLI-first defaults,
- no new architectural drift into `imp-core` hot spots.

Why first:
- without stable startup/runtime boundaries, a legible worker contract, and a stable product shape, reliability work becomes rework.

---

## Phase 1 — Provider resilience

Primary lane: **Provider resilience**

Depends on: rebuild I1–I3 and phase-0 bootstrap work

Outcome:
- resilient provider/auth behavior under degraded conditions,
- predictable fallback/failover semantics,
- one canonical provider/auth resolution path serving all major runtime surfaces,
- clearer operator recovery paths.

Includes:
- auth-state clarity in CLI/TUI flows,
- bounded failover policy (opt-in/controlled, not silent chaos),
- provider capability/profile awareness for safer defaults,
- explicit telemetry around fallback decisions,
- removal or reduction of duplicate provider/auth resolution logic across CLI, TUI, and headless runtime paths.

Why now:
- reliability incidents directly block day-to-day trust and worker uptime,
- recent docs and deep review both point to duplicated provider startup logic as a core risk.

---

## Phase 2 — Context continuity

Primary lane: **Context continuity**

Depends on: rebuild I3–I5

Outcome:
- stronger session recall, compaction quality, and selective continuity,
- better cold-start inheritance via durable mana handoff + focused local context,
- clearer memory-layer boundaries across session, personal memory, mana work memory, and future synthesized knowledge.

Includes:
- improved compaction boundaries and handoff structure,
- retrieval/relevance tuning for long sessions,
- stronger session index lifecycle and richer recall surfaces,
- clearer split between session transcript data, personal memory, and durable mana state,
- future-facing room for mana-backed wiki/project knowledge without collapsing it into session memory.

Why here:
- after baseline reliability, continuity is the highest leverage quality multiplier,
- recent memory/ownership docs make this lane much easier to scope cleanly.

---

## Phase 3 — Evaluation and observability

Primary lane: **Evaluation and observability**

Depends on: rebuild I6–I8 (plus output/evidence plumbing)

Outcome:
- measurable quality over time, not anecdotal confidence,
- evidence-oriented run outputs that are inspectable and comparable,
- practical harnesses that catch regressions in the reliability and continuity work above.

Includes:
- stable run/result instrumentation points,
- focused eval harnesses tied to real failure classes,
- normalized verification/evidence summaries for `mana` handoff,
- operator-readable diagnostics for “why this run succeeded/failed,”
- expansion of the current A/B harness and related runtime-quality proving slices.

Why here:
- this phase converts prior reliability/continuity work into compounding improvement loops,
- recent harness notes show imp already has the beginnings of a useful proving-slice strategy.

---

## Phase 4 — Guest-runtime substrate, SDK, and hostability

Primary lane: **guest-runtime substrate, SDK, and hostability**

Depends on: rebuild I6–I8, prior observability primitives, and policy hardening

Outcome:
- cleaner seams for embedding `imp` runtime behavior safely,
- a clearer guest-runtime extension substrate under Rust host control,
- a more explicit SDK/hostability story for non-TUI surfaces,
- explicit separation between worker runtime, script tool, and durable extensions.

Includes:
- explicit host integration contracts (SDK/runtime boundaries),
- guest-runtime-neutral extension language and metadata under Rust host ownership,
- stronger policy/capability enforcement across extension surfaces,
- script-tool design and policy kept separate from durable extension packaging,
- migration-ready extension architecture posture without claiming TS extension GA today.

Why after eval:
- opening host/extension seams before observability and policy hardening increases risk,
- recent docs make it clear that extension work should tighten host authority before expanding power.

---

## Phase 5 — Selective surface expansion

Primary lane: **Selective surface expansion**

Depends on: proven wins in phases 1–4

Outcome:
- targeted product-surface growth where evidence shows operator value,
- no uncontrolled expansion of UX breadth.

Includes (examples):
- deeper operator inspection flows,
- constrained new workflow views/surfaces,
- selective GUI-facing enhancements that consume stable runtime state,
- optional mana-state/knowledge affordances that sit on top of stable runtime and continuity work.

Explicit guardrail:
- surface expansion is a multiplier on reliable core behavior, not a substitute for it.

---

## Why this order

1. **Boundary stability and product-shape clarity before feature ambition** (avoid rework).
2. **Reliability before convenience** (trust is table stakes).
3. **Continuity before breadth** (quality of ongoing work beats more UI).
4. **Measurement before expansion** (ship what we can prove improves outcomes).
5. **Policy-hard hostability before ecosystem scale** (safe seams before bigger surface area).

---

## Adopt-now vs later

## Adopt now (commit in current planning horizon)

- Shared startup/bootstrap and worker-runtime seam cleanup.
- Provider resilience/auth/failover lane.
- Context continuity/session recall/compaction lane.
- Evaluation and observability foundations tied to evidence handoff.

## Adopt next (after foundations prove out)

- Guest-runtime substrate, SDK, and hostability hardening as a first-class lane.
- Script-tool separation and host-authoritative policy carried through extension work.

## Adopt later (optional, evidence-gated)

- Selective surface expansion.
- Any broader cross-product UX breadth not required for core agent quality.
- Broader gateway/scheduler/backend breadth unless earlier phases prove the need.

---

## Concrete implementation lanes mapped to rebuild phases

- **Shared runtime/bootstrap + worker-runtime clarity** → mainly I0–I3.
- **Provider resilience/auth/failover** → mainly I1–I3, then hardening in I7.
- **Context continuity/session recall/compaction** → mainly I3–I5.
- **Evaluation and observability** → mainly I6–I8, with early instrumentation seeds in I3.
- **Guest-runtime substrate, SDK, and hostability seams** → mainly I6–I8, after policy hardening signals.
- **Selective later product-surface expansion** → post-I8 and evidence-gated.

---

## Explicit non-goals

- Rewriting `imp` from scratch.
- Chasing Hermes breadth as an immediate roadmap target.
- Moving durable workflow ownership out of `mana`.
- Positioning `imp` as only a UI shell over mana.
- Announcing TypeScript extension maturity as current shipped behavior.
- Letting the TUI lead the runtime architecture.
- Moving worker execution into guest runtimes.
- Merging ephemeral scripting, durable extensions, and worker execution into one runtime concept.
- Running broad, parallel architectural redesigns that conflict with rebuild sequencing.

---

## Success criteria for imp vNext

`imp vNext` is successful when:

- `imp` remains clearly the flagship Rust-native agent product on mana.
- `imp` has one canonical runtime/bootstrap path reused across major surfaces.
- `imp run` is clearly legible as the canonical single-unit worker runtime.
- Reliability under provider/auth disruption is materially better.
- Long-session/task continuity is materially better without context bloat.
- Evidence and verification outputs are more structured, comparable, and reusable.
- Hostability, guest-runtime, and extension seams are clearer without weakening policy boundaries.
- CLI-first product direction is real without forcing a premature TUI rewrite.
- Later surface growth is selective and justified by measured outcomes.

---

## Maintenance note

Keep this file updated as roadmap truth changes.
If sequencing changes, update both:
1. phase ordering in this document,
2. mapping notes to `imp_rebuild_plan.md` I0–I8.
