---
id: '45'
title: Tower rebuild around explicit contracts, durable leases/evidence, and clearer worker boundaries
slug: tower-rebuild-around-explicit-contracts-durable-le
status: open
priority: 1
created_at: '2026-04-08T17:14:47.950841Z'
updated_at: '2026-04-13T20:45:38.644056Z'
acceptance: A root rebuild feature exists with child phase epics in the planned order, plus initial worker-safe jobs for the earliest phases. The graph should be detailed enough that future workers can continue decomposition and execution from mana state rather than from transient chat context.
notes: |-
  ---
  2026-04-08T17:22:05.180576+00:00
  Graph bootstrapped as the canonical rebuild execution tree. Existing units 28, 29, and 44 remain useful architecture/research context, but future decomposition for the Tower rebuild should continue under feature 45 rather than extending those older units as the primary graph.

  ---
  2026-04-08T17:45:12.974666+00:00
  Decision from user on 2026-04-08: prefer docs/specs first for the rebuild. Planning/specification work should land before code-migration slices where a meaningful design question remains. Future workers should not rush into early implementation just because a code path is available; they should first close the spec/docs jobs that define the relevant surface and migration order.

  ---
  2026-04-08T17:48:49.824872+00:00
  Units 28, 29, and 44 remain context/reference inputs for the rebuild, but they are not the canonical execution graph. Continue decomposition under 45 and its child phase epics/jobs.

  ---
  2026-04-08T17:50:17.695548+00:00
  Continue decomposition under 45 and its child phase epics/jobs.

  ---
  2026-04-08T17:50:54.045280+00:00
  Verification follow-up for 45.1.3: adding the exact lowercase phrase required by the current verify gate — continue decomposition under 45.

  ---
  2026-04-09T04:07:12.428727+00:00
  New user framing from 2026-04-09: think of agents as effectively string-in/string-out systems. Mana is the substrate of the work and knowledge an agent consumes and produces. The original mana idea was engineering on top of a Ralph-style loop: instead of prompting in the same general direction repeatedly, give each agent a specialized prompt around its task so planning, prototyping, final-plan creation, and implementation can happen against a comprehensive task contract/spec. Imp is the native agent/interface that can fully utilize mana as both tool and substrate.

  ---
  2026-04-09T04:47:50.120363+00:00
  Decision from user on 2026-04-09: treat `mana run` as the wrong boundary language and move the architecture toward `imp run {mana_id}` as the primary execution model. Update docs and planning to reflect that imp is the orchestrator/runtime reading mana state and performing tasks, while `mana run` should be treated as legacy/compatibility behavior rather than the intended long-term boundary.

  ---
  2026-04-09T05:18:16.738139+00:00
  User clarified on 2026-04-09 that prior specs should be revised now to match the new imp-centered boundary. Do not leave `mana run`-centered rebuild specs as merely historical artifacts; update them so the root documentation and planning graph consistently reflect imp as orchestrator/runtime and `imp run {mana_id}` as the preferred execution framing.

  ---
  2026-04-09T05:35:00.000000+00:00
  Superseding decision: the architecture should move away from `mana run`-centered framing and toward `imp run {mana_id}` as the preferred execution model. Legacy `mana run` protocol shapes may still need compatibility mappings during migration, but earlier specs that treated `mana run` as the intended center are now superseded.

  ---
  2026-04-09T05:42:40.473851+00:00
  Architecture answer snapshot from current repo inspection on 2026-04-09:

  1. Mana is the cross-agent durable substrate/work medium. It owns the work and knowledge agents consume and produce durably.
  2. Imp is the native worker/runtime and orchestrator. It reads mana state, performs the task, and writes back structured durable outcomes.
  3. The happy path is mana → imp → mana.
  4. Mana should hand imp a specialized task packet, not just a raw unit file. That packet should include the task contract, acceptance/verify target, relevant facts, dependency context, prior failures/attempts, path hints, and important constraints/decisions.
  5. Imp should hand back a rich durable result, not just narration. That result should include status, summary, what changed, what checks ran, pass/fail, blocker/failure summary, next-worker notes, and useful artifact/log refs.
  6. Durable rule: if another worker should inherit it cold, it belongs in mana. Ephemeral rule: local reasoning flow, transient tool chatter, UI/session state, and temporary caches can stay in imp.
  7. Retrieval rule: store rich when cheap and useful; reveal selectively when assembling the next worker context.
  8. Repo-grounded ownership reading: `imp/crates/imp-core/src/mana_worker.rs`, `imp/crates/imp-cli/src/main.rs`, and `imp/crates/imp-core/src/system_prompt.rs` already support imp-centered execution. `mana/crates/mana-cli/src/spawner.rs` and `mana/crates/mana-core/src/prompt.rs` represent overlapping/legacy behavior that should be treated as transitional rather than the intended long-term center.
  9. Command boundary answer: move toward `imp run {mana_id}` as the primary execution model. Treat `mana run` as legacy/compatibility behavior on the path to disappearing.
  10. Shared-type ownership naming is no longer the primary blocker. First formalize the handoff vocabulary from the existing coexistence, then decide the narrowest code home; do not let crate naming block real work.

  ---
  2026-04-09T05:47:27.813311+00:00
  Decision from user on 2026-04-09: proceed with parallel coordinated sequencing for the next decomposition wave. Create/refine both tracks now — A) imp-centered execution path and B) task/result handoff model — but keep them dependency-aware with an explicit later integration point rather than forcing one track to fully finish before the other begins.

  ---
  2026-04-09T05:55:01.686091+00:00
  User correction on 2026-04-09: some newly created rebuild jobs were phrased too much like fresh definitions and not enough like audits of current codebase alignment. Adjust the handoff/result/integration jobs so they first audit current runtime reality, confirm what is already defined well, and only then identify the narrowest gaps to fix.

  ---
  2026-04-09T07:23:07.517541+00:00
  2026-04-09T07:27Z Phase-1 contracts checkpoint: the first shared-contract ownership move is now landed. `tower-contracts::worker` owns the canonical worker handoff/result vocabulary (`WorkerAssignment`, `WorkerAttempt`, `WorkerResult`, `WorkerStatus`). `imp/crates/imp-core/src/mana_worker.rs` no longer defines those canonical types; it now consumes and re-exports them to preserve current imp-core call sites during migration. Keep the decomposition explicit from here: worker assignment/outcome vocabulary is migrated first; run/view/event protocol work remains on the runner/execution path units, durable result/evidence lineage remains on the evidence units, and migration-shim cleanup remains on the renameability/shim unit. Do not treat 45.2.3 as a signal to broaden into run-stream/view/event moves in the same slice.

  ---
  2026-04-09T13:26:51.162792+00:00
  2026-04-09 durable implementation delta: unit 45.8 landed the first mana-owned ApprovalRecord / PromotionRecord schema in mana-core. Implementation constraint discovered and now externalized as fact 70: mana-review currently depends on mana-core, so mana-core cannot directly embed mana-review crate types without a dependency cycle. As a result, the first durable approval schema stores review/risk lineage through refs plus snapshots (review_refs, review_decision_refs, risk_level snapshot, RiskFlagRecord) rather than direct mana-review types. Follow-up unit created to decide long-term ownership for shared review/risk vocabulary.

  ---
  2026-04-13T07:02:05.225278+00:00
  2026-04-13 architecture clarification from user: preserve the rebuild around `mana` as a headless durable control plane and `imp` as the live execution shell. Migration consequence: `mana run` should converge into thin compatibility/delegation behavior over the same attach path used by `imp run {mana_id}`, not remain a separate orchestration center. Staged cutover to preserve in downstream phase work: (1) define canonical mana run/node/lease/event/artifact/checkpoint semantics first, (2) dual-write current execution through mana-owned semantics, (3) shadow-schedule in mana until outputs match, (4) cut imp over to lease-based execution where imp attaches, receives leases, heartbeats, checkpoints, and resolves via mana, (5) move transcript ownership fully into imp while mana remains the canonical recovery substrate through structured events/checkpoints/artifacts, (6) publish the embeddable/library-first mana surface and deprecate the old `mana run` path.

  ---
  2026-04-13T19:39:14.442981+00:00
  2026-04-13 rebuild framing revalidated against current root docs (`README.md`, `VISION.md`). Feature 45 should continue to serve the active two-part foundation rather than broader ecosystem scope: `mana` as durable substrate/control plane and `imp` as the live execution shell/runtime, with the canonical loop `mana -> imp -> mana`. Migration and phase work under this feature should explicitly strengthen that contract — durable memory/evidence/coordination on the mana side, safe focused execution on the imp side — and should keep `wizard/` / `familiar/` deferred unless a unit explicitly targets later-phase reference material.

  ---
  2026-04-13T20:45:38.644053+00:00
  Project-wide blocker review (2026-04-13): current rebuild-planning chokepoints are 45.4.2 (imp-local runner adapter plan), 45.4.3 (runner migration sequence), and 45.7.1 (capability-hardening map). These in turn gate 45.5.1, 45.5.3, and 45.7.2 respectively. Prioritize those planning artifacts to collapse blocked phase-4/phase-6 follow-ons.
labels:
- tower
- rebuild
- architecture
- mana
- imp
- runner
- contracts
kind: epic
paths:
- rebuild_plan.md
- mana/mana_rebuild_plan.md
- imp/imp_rebuild_plan.md
- Cargo.toml
- mana/crates/mana-cli/src/spawner.rs
- imp/crates/imp-core/src/mana_worker.rs
- imp/crates/imp-core/src/tools/mana.rs
feature: true
---

Goal: execute the Tower rebuild described in `rebuild_plan.md`, `mana/mana_rebuild_plan.md`, and `imp/imp_rebuild_plan.md` as an explicit mana graph. Current inspected state: the root plans now freeze a top-level order of operations; the root workspace currently has only `mana/*` and `imp/*` crates in `Cargo.toml`; there is not yet a Tower-level shared contracts crate; `mana/crates/mana-cli/src/spawner.rs` is still shell-template driven; `imp/crates/imp-core/src/mana_worker.rs` still defines canonical worker assignment/outcome types inside `imp`; `imp/crates/imp-core/src/tools/mana.rs` currently consumes `mana-cli` run types directly. Existing units `28`, `29`, and `44` remain useful architectural context but are not the canonical execution graph for this rebuild. This feature should create a clean phase-ordered graph that another worker can execute cold without reconstructing the architecture from chat history.

Planned phase order:
1. Freeze architecture and graph shape
2. Create Tower shared contracts spine
3. Rebuild `mana` durable substrate seams
4. Introduce runner protocol and local adapter
5. Rebuild `imp` around runtime/context/tools/policy/worker seams
6. Make evidence/review/approval first-class workflow stages
7. Harden policy, isolation, and migration surfaces

Use child epics and jobs with narrow verify gates and explicit dependencies. Prefer worker-safe descriptions with file paths, concrete outputs, and anti-goals.
