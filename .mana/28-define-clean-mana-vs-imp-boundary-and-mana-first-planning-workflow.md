---
id: '28'
title: Define clean mana vs imp boundary and mana-first planning workflow
slug: define-clean-mana-vs-imp-boundary-and-mana-first-planning-workflow
status: open
priority: 1
created_at: '2026-04-07T04:45:00Z'
updated_at: '2026-04-14T08:06:12.845297Z'
notes: |-
  ---
  2026-04-09T05:22:46.873629+00:00
  Decomposition update from 28.1: the mana-first planning workflow is now specified as continuous conversation-time externalization rather than end-of-task logging. The durable split under this parent is now clearer: 28.1 owns the policy for automatic externalization triggers, artifact mapping (epic/job/note/decision), confirmation thresholds, and between-turn mana delta summaries; 28.4 owns the user-facing review affordances that expose those deltas and graph state between turns; 28.5 should later convert the agreed policy outputs from 28.1–28.4 into an implementation sequence across imp runtime/prompt behavior, mana-native update/create flows, and review surfaces.

  ---
  2026-04-09T05:23:36.139755+00:00
  28.1 durable policy output, surfaced onto the live parent graph for visibility: imp should treat planning/design conversations as conversation-time mana-maintenance work, not end-of-task logging. Automatic externalization triggers are: plan/design/architecture/decomposition requests; multi-outcome or multi-turn structure; discovered blockers/dependencies/handoffs; consequential forks worth preserving; user-supplied constraints or acceptance changes; and any case where a future worker would otherwise need transcript reconstruction. Artifact mapping: epic for stable multi-outcome goals, child job for concrete executable/reviewable sub-outcomes, notes for evolving rationale/constraints/risks/sequencing, and decisions for consequential forks or commitments. Confirmation threshold: imp should auto-capture low-risk structure but still ask before consequential scope/ownership/architecture commitments or before launching execution when the user asked only for planning. Between substantive planning turns, imp should update mana first when durable state changed and then summarize the mana delta in the reply so the graph remains continuously reviewable.

  ---
  2026-04-09T05:23:49.865572+00:00
  Boundary spec landed in `docs/rebuild/mana-imp-ownership-boundary.md`. Practical decomposition captured from the spec: (1) mana side should converge from `PromptResult { system_prompt, user_message, file_ref }` toward a structured execution bundle carrying durable task contract + inherited context + provenance/trust metadata; (2) imp side remains the model-facing assembly layer that combines that bundle with identity, tools, AGENTS.md, session/personal memory, environment, and live repo reads; (3) retrieval split is mana = durable shared project retrieval semantics, imp = live retrieval application and token-budgeted reveal.

  ---
  2026-04-09T05:24:16.116742+00:00
  Memory/retrieval taxonomy spec landed in `docs/rebuild/memory-taxonomy-and-retrieval-boundary.md` after inspecting `imp/crates/imp-core/src/memory.rs`, `imp/crates/imp-core/src/tools/session_search.rs`, `mana/crates/mana-core/src/ops/fact.rs`, and `mana/crates/mana-core/src/ops/recall.rs`.

  Durable decisions captured from that spec:
  - `imp` owns only session/runtime memory and personal/private persistent memory.
  - `session_search` remains transcript recall over imp sessions; it is not project-memory retrieval and should not become the shared-project source of truth.
  - `mana` owns all shared project memory: project work/attempt/decision memory, verified facts, and future synthesized project knowledge.
  - Canonical project retrieval belongs to `mana` (`mana recall`, fact retrieval, `mana context`/unit briefings); `imp` should consume that retrieval rather than own a second durable shared project-memory system.
  - Project memory should be maintained as workflow-shaped mana artifacts (units, attempts, decisions, facts, later wiki pages), not as a dedicated project session/chat log.
  - Explicit answer to the open question: dedicated mana workflow yes, dedicated mana session no.

  Trust/precedence rule from the spec for downstream migration work:
  1. verified mana facts
  2. mana unit state / decisions / attempt history
  3. synthesized mana knowledge pages
  4. imp session-search results
  5. imp personal memory (preferences/environment only)

  Migration direction from the spec:
  - clarify naming/doctrine first
  - make imp consume mana project memory more directly in interactive and headless flows
  - move any remaining project-memory pressure out of imp and into mana rather than adding a third imp memory store

  ---
  2026-04-09T05:24:19.802298+00:00
  Memory/retrieval consolidation output from archived child 28.3 has now been materialized as `docs/design/memory-taxonomy-and-retrieval.md`. Key decisions to preserve on the live parent graph: (1) four memory classes — session memory and personal/private memory stay in imp; project work memory and project knowledge memory belong in mana; (2) retrieval contract — `imp session_search` remains transcript recall only, `imp` personal memory remains private preference/env memory, `mana fact` is the verified project-knowledge workflow, and `mana recall` should become the canonical query surface for shared project memory rather than remaining unit-history-only; (3) migration direction — do not add an imp-local project memory store, and treat transcript discoveries as candidates to externalize into mana during conversation; (4) project memory structure — workflow yes, session no: maintain shared memory through explicit mana workflows/records, not a pseudo-session transcript.

  ---
  2026-04-09T05:24:52.013070+00:00
  Cleaned up duplication after reviewing archived child 28.3: `docs/rebuild/memory-taxonomy-and-retrieval-boundary.md` is the canonical memory-taxonomy spec recorded by the archived unit, and the accidentally-created duplicate `docs/design/memory-taxonomy-and-retrieval.md` was removed so future work has one authoritative path.

  ---
  2026-04-09T05:26:46.947127+00:00
  2026-04-09 — externalized from the 28.2 boundary-spec execution session. The landed spec is `docs/rebuild/mana-imp-ownership-boundary.md`. Durable outputs to preserve on the live graph: `mana` owns durable/shared/coordinated/verified work state and inherited project memory; `imp` owns live/local/model-facing/user-personal runtime behavior; canonical handoff is `mana -> ExecutionBundle -> imp -> WorkerResult -> mana`; final prompt assembly stays in `imp`; `mana/crates/mana-core/src/prompt.rs` is the current wrong-owner overlap because it renders final prompt text rather than only structured execution context. Also observed a root-mana traceability inconsistency during closeout: active graph resolution no longer exposed child units 28.1/28.2/28.3 even though parent 28 and 28.5 still carry their outputs/dependencies. Treat the landed docs plus parent notes as current truth until graph traceability is repaired.

  ---
  2026-04-09T06:28:32.424808+00:00
  2026-04-09 — externalized the 28.4-derived review/decomposition into live root mana. Added child jobs under 28.5 to preserve the agreed order: `28.5.3` defines the canonical turn-batched mana delta contract with explicit `no_change`; `28.5.4` defines the imp-first compact between-turn review block and expanded textual inspector that asks only on consequential unresolved choices; `28.5.5` defines the later Wizard review-queue/focus-room layer as an optional richer consumer of the same delta semantics. This preserves the core decision that imp ships the mandatory review affordance first and Wizard remains a later, non-required visual layer.

  ---
  2026-04-13T19:39:14.273343+00:00
  2026-04-13 repo-grounded orientation refresh after reading `README.md` and `VISION.md` from the Tower root. Preserve this current top-level decomposition on the live graph: Tower is the active umbrella root for `mana` + `imp` only; `wizard/` and `familiar/` remain deferred reference material and are not active implementation scope unless a task explicitly targets them. `mana` is the durable cross-agent substrate/control plane owning work graph, dependencies, facts, attempts, verification/evidence history, and shared project memory. `imp` is the live worker/runtime shell owning context assembly, tool use, session/runtime behavior, edits/command execution, runtime policy, and structured result reporting back into mana. The intended happy path remains `mana -> imp -> mana`. The project goal stated in `VISION.md` is reliable autonomy for software work through durable memory, verified execution, runtime policy, and coordinated/isolation-aware multi-agent work. Use this as the baseline repo explanation for future boundary/onboarding/planning work.

  ---
  2026-04-14T07:59:46.111782+00:00
  Execution checkpoint on 2026-04-14 for the continue-until-blocked loop work. Implemented another `imp/crates/imp-core/src/agent.rs` slice that broadens execution-side evidence grounding for `NextActionStopReason::WorkCompleted`: `tool_results_indicate_work_completed(...)` now treats a turn as completion-evidence when it contains both a successful edit-like tool result (`write`, `edit`, `multi_edit`) and a successful check-like command result with structured `command` + `exit_code == 0` (`check`, `test`, `verify`, `pytest`, `cargo check`, `cargo test`), in addition to the earlier close/verify-backed paths. Focused tests passed: `tool_results_indicate_work_completed_detects_edit_plus_successful_check`, `tool_results_indicate_work_completed_detects_closed_unit_details`, `execution_stops_after_mana_close_tool_result_without_done_text`, `execution_stops_after_work_completed_text`, `agent_multiple_tool_calls`, plus `cargo check -p imp-core`. Repo-state oddity to preserve: direct root lookup for transient follow-on unit id `28.6` failed after prior close/create churn, so this checkpoint is being attached to parent 28 to avoid losing the implementation delta while root mana lookup consistency is unclear.

  ---
  2026-04-14T08:06:12.845292+00:00
  SuperProduct alignment assessment (from 2026-04-14 discussion): current Tower maps most strongly below the 4-part product core rather than directly implementing it. mana is closest to a proto-SuperState for software-work autonomy: durable work graph, facts, evidence, verification, dependency state, attempt/failure memory, and coordination. imp is closest to a proto-SuperAction plus proto-SuperShell: live execution/runtime, tool use, policy-gated actions, CLI/TUI shell, and emerging review/approval UX. Current extension seams (Lua tools/skills/config, runtime contracts, planned embedding/runner seams) are only a partial proto-SuperComp, not yet a declarative versioned module system. Major gap versus the 4 core: Tower lacks a general Object/Assertion/Event/Rule substrate and stable product-level action vocabulary; most current concepts are still specialized around coding work and agent orchestration. Conclusion: strong structural alignment with the runtime/substrate stance in PROJECT 1.4, medium alignment with SuperState/SuperAction directionally, weak-to-medium alignment with SuperComp/SuperShell as full business-product layers.
labels:
- architecture
- imp
- mana
- memory
verify: test -n "boundary-spec"
kind: job
decisions:
- Carry the key 28.1 planning-policy outputs onto the live parent so future boundary and migration work does not depend on archived child visibility.
- |-
  Boundary decisions from 28.2 to preserve at the parent level for downstream migration work:
  1. Semantic ownership split: mana owns durable/shared/coordinated/verified work state and inherited project memory; imp owns live/local/model-facing/user-personal runtime behavior.
  2. Contract split: mana should provide structured execution context as an execution bundle; imp remains the owner of final prompt assembly. Do not move final prompt rendering into mana.
  3. Redundancy direction: remove mana-side final prompt rendering, user-message generation, generic worker coaching prose, and live file-to-prompt embedding from the long-term mana boundary; retain imp-side projection/adaptation types and UX-local state mirrors as acceptable layering when mana remains the durable source of truth.
- 'Memory boundary decision: `imp` keeps session/runtime memory and personal/private persistent memory only; `mana` owns all shared project memory, including work/attempt/decision memory, verified facts, and future synthesized project knowledge.'
- 'Retrieval boundary decision: `session_search` is transcript recall only. Canonical project retrieval belongs to `mana` via recall, fact retrieval, and context/unit briefing assembly; `imp` may consume these surfaces but should not own a second durable shared project-memory retrieval system.'
- 'Project-memory modeling decision: project memory should be workflow-shaped mana state, not a dedicated project session/chat log. Dedicated workflow yes; dedicated mana session no.'
- '2026-04-13 boundary clarification from user: mana is the headless control plane for work; imp is the live execution shell for a user/agent session. Refined ownership rule: mana owns inter-step truth while imp owns intra-step experience. Preferred command boundary is `imp run {mana_id}` meaning attach a live runtime/session to a mana-owned run. Anything that must survive a crash or affect global orchestration must be projected back into mana as a typed, verified state change.'
---

Goal: define the clean ownership boundary between mana and imp, and specify a stronger mana-first workflow where planning/design conversations are proactively externalized into mana as a persistent work and idea graph.

Current state:
- mana owns coordination/work graph while imp owns runtime, but there is overlap around prompt shaping, retrieval, durable memory, and planning behavior.
- imp currently underuses mana during conversations like this by logging work late instead of continuously externalizing plans and architecture decisions.
- the user wants imp to automatically write implementation plans and idea decomposition into mana during conversations, only asking for decisions it cannot make autonomously.
- the user also wants visual space in imp/Wizard to review mana information between turns.

Steps:
1. Define the boundary: what must be durable/shared/coordinated vs what must stay live/local/model-facing.
2. Define how planning conversations should automatically create/update mana structure without waiting for explicit user requests.
3. Define how shared/project memory should move toward mana while session and personal memory stay in imp.
4. Decompose the migration into concrete child units with clear ownership.

Files:
- /Users/asher/tower/AGENTS.md (read — root ownership rules)
- /Users/asher/tower/README.md (read — umbrella intent)
- /Users/asher/tower/VISION.md (read — product direction)
- /Users/asher/tower/UMBRELLA.md (read — cross-project structure)
- imp/crates/imp-core/src/system_prompt.rs (read — current doctrine)
- imp/crates/imp-core/src/tools/mana.rs (read — native mana surface)
- mana/crates/mana-core/src/prompt.rs (read — mana-side execution context ideas)

In scope:
- architecture boundary
- mana-first planning workflow
- memory/retrieval ownership
- concrete migration plan

Out of scope:
- implementing the migration in this unit
- UI polish details beyond boundary implications

Do not:
- collapse mana and imp into one runtime
- move final prompt rendering out of imp
- treat personal memory and project memory as the same thing
