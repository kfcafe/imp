---
id: '81'
title: Design imp-native delegation tool around imp run and mana orchestration
slug: design-imp-native-delegation-tool-around-imp-run-a
status: closed
priority: 1
created_at: '2026-04-10T02:57:25.818332Z'
updated_at: '2026-04-24T05:37:29.290199Z'
notes: |-
  ---
  2026-04-10T02:57:50.784717+00:00
  Inspected current repo state. Key grounding: (1) `imp run <unit-id>` already exists in `imp-cli` and `run_headless_mode()` loads `WorkerAssignment` through `imp_core::mana_worker`, assembles context/prefill, runs the session, and optionally verifies/closes. (2) `mana-cli run_native()` still treats `mana run` as a compatibility orchestration entrypoint and checks for `imp` on PATH in direct mode. (3) `mana-cli spawner.rs` still shells out via `sh -c`, claims units, sets `IMP_MODE`, and captures logs. (4) `imp-core tools/mana.rs` already consumes native mana run/status/state surfaces; `builder.rs` registers `ManaTool` as a native tool. Early design read: the first `imp`-calls-`imp` surface should likely sit in `imp` as a native tool/runtime seam, but durable delegation should still externalize work into mana and use the canonical `imp run` worker path rather than inventing a second durable orchestration channel.

  ---
  2026-04-10T02:58:58.251517+00:00
  Externalized initial plan in mana before further drafting. Working recommendation to carry into the spec unless contradicted by deeper repo evidence: use a two-layer delegation model where `imp` owns live child-worker invocation, but any durable or mutating delegation must route through mana and reuse the canonical `imp run <unit-id>` worker path. Keep direct prompt-only child calls, if allowed at all, explicitly transient and non-authoritative in v1.

  ---
  2026-04-10T02:59:39.561877+00:00
  Decomposition externalized into root mana now. Added child units: 81.2 for the delegation target model decision, 81.3 for the parent-visible result contract, and 81.4 for policy/isolation/migration constraints. 81.1 tracks the decomposition layer itself so the visible graph captures the architecture split before implementation drafting continues.

  ---
  2026-04-10T03:18:10.990487+00:00
  User confirmed the consequential choice: adopt the two-layer v1 delegation model. Treat this as the settled design baseline unless repo constraints force revision. Interpretation: `imp` may gain a native delegation surface, but durable or mutating delegation must externalize into mana and reuse the canonical `imp run <unit-id>` worker path; any direct prompt-only child calls remain explicitly transient and secondary.

  ---
  2026-04-10T03:21:49.198750+00:00
  Drafted `docs/rebuild/imp-delegation-tool-and-runtime.md` and verified the parent gate strings. The note now locks in the user-confirmed two-layer model: mana-backed delegation is the primary path for durable or mutating work and reuses canonical `imp run <unit-id>` / `WorkerAssignment`; ad hoc delegation is explicitly transient, read-oriented, and non-authoritative in v1. The note also defines a parent-visible result contract, conservative mode/policy rules, and migration guidance that keeps `mana run` compatibility secondary while leaving room for a later runner-backed adapter.

  ---
  2026-04-10T03:24:16.401532+00:00
  Externalized the next implementation decomposition into root mana. Added 81.5 to extract a reusable imp-core worker runner from the existing `imp run` path, 81.6 to implement the first native `imp` tool with unit-backed delegation over that canonical worker path, 81.7 to add the restricted transient ad hoc delegation mode, and 81.8 to enforce mode/policy gating and focused tests. Dependency shape: 81.6 and 81.7 both depend on 81.5; 81.8 depends on 81.6 and 81.7.

  ---
  2026-04-10T03:43:12.191907+00:00
  Tightened the implementation decomposition for 81.5–81.8 using the loaded `mana` skill guidance. The units now name exact files/functions/patterns from inspected repo state, specify the chosen approach instead of leaving design decisions to the worker, and use sharper verify gates. Grounded details embedded into the child units include: `imp-cli/src/main.rs::run_headless_mode()`, `imp-core/src/mana_worker.rs`, `builder.rs::register_native_tools()`, `config.rs::allowed_tool_names()`, and the existing `agent_mode_enforcement_*` test area in `imp-core/src/agent.rs`.

  ---
  2026-04-13T05:25:29.347495+00:00
  Repo-grounded model-selection finding from inspection on 2026-04-13:

  - There is still no native `imp` tool module in `imp-core` today. The current real single-unit path is `imp run <unit-id>` in `imp-cli`, backed by `imp_core::mana_worker`.
  - Current CLI `imp run` already supports transient runtime overrides through `SessionOptions`: `model`, `provider`, `api_key`, `thinking`, `max_turns`, `max_tokens`, `system_prompt`, and `no_tools`.
  - Current durable worker-assignment contract is weaker: `tower-contracts::worker::WorkerAssignment` carries only `model: Option<String>` as a unit-level model override.
  - `mana-core::unit::Unit` likewise currently exposes only `model: Option<String>` in frontmatter; there is no durable per-unit `provider` or `thinking` field today.
  - `81.5` already plans a shared `WorkerRunOptions` type carrying runtime overrides including model/provider/thinking, so the canonical worker-runner extraction is the right seam for transient delegation-time model selection.
  - The remaining gap for the future native `imp` tool is schema/contract shape in `81.6` and possibly durable unit metadata if we want per-unit provider/thinking instead of delegation-time-only overrides.

  Likely implementation direction if pursued:
  1. Keep `81.5` owning reusable transient runtime override plumbing (`model`, `provider`, `thinking`) in the shared worker runner.
  2. Expand `81.6` tool schema/results so unit-backed delegation can pass optional `model` / `provider` / `thinking` overrides and report the resolved selection back in structured details.
  3. Treat durable per-unit provider/thinking support as a separate cross-project decision because it changes `mana-core::Unit` and `tower-contracts::worker::WorkerAssignment`, not just imp tooling.

  ---
  2026-04-13T05:26:30.985559+00:00
  Conversation-time decomposition update from the latest model-selection discussion:

  Chosen two-step plan for better model selection in the future native `imp` delegation/run surface:

  1. Near-term / implementation path (keep scoped to imp-side runtime/tooling):
     - Improve transient delegation-time model selection first.
     - `81.5` should make the shared worker runner accept runtime overrides for at least `model`, `provider`, and `thinking` via `WorkerRunOptions`.
     - `81.6` should expose those optional override fields in the native `imp` tool schema for unit-backed delegation and return the resolved selection in structured result details.
     - This path should not require changing mana durable unit schema.

  2. Separate follow-up / cross-project contract decision:
     - Decide later whether durable per-unit `provider` and `thinking` belong in mana-backed unit metadata and `tower-contracts::worker::WorkerAssignment`, rather than being delegation-time-only overrides.
     - Treat that as a distinct root-scope architecture/unit-contract question because it affects `mana-core::Unit`, `tower-contracts`, and imp worker loading/runtime together.

  Reasoning grounded in current repo state:
  - CLI `imp run` already supports transient `model` / `provider` / `thinking` via `SessionOptions` and `ImpSession::create()`.
  - The durable worker contract currently only carries `model: Option<String>`.
  - Therefore transient override plumbing is the smallest correct first slice, while durable per-unit provider/thinking is broader contract work.

  ---
  2026-04-13T07:21:26.617191+00:00
  Current implementation-status audit (2026-04-13) externalized from chat. Repo-grounded findings: the delegation architecture/design is in place in `docs/rebuild/imp-delegation-tool-and-runtime.md`, but there is still no native `imp` tool implementation in `imp-core`. Verified gaps from inspected code: `imp/crates/imp-core/src/tools/imp.rs` is missing; `imp/crates/imp-core/src/tools/mod.rs` has no `mod imp;`; `imp/crates/imp-core/src/builder.rs::register_native_tools()` does not register `ImpTool`; `imp/crates/imp-core/src/config.rs::allowed_tool_names()` does not include `"imp"` for `Orchestrator`; and `imp/crates/imp-core/src/mana_worker.rs` does not yet expose a shared `run_worker_assignment(...)`, `WorkerRunOptions`, or `WorkerRunOutcome`. Practical decomposition/status: MVP implementation remains 81.5 (extract shared worker runner), 81.6 (unit-backed native imp tool), 81.7 (restricted transient ad hoc mode), 81.8 (mode/policy gating), with 81.9 as the still-open architecture decision on whether `provider` and `thinking` belong durably in mana-backed worker metadata. Product-grade but non-MVP follow-on work is already tracked elsewhere rather than duplicated here: runner/attach-path migration under 45.4.x, capability hardening under 45.7.x, and richer result/presentation surfaces under existing imp CLI/TUI/runtime units such as 28.5.7 / 50.14 / 50.17 / 44.1.12.

  ---
  2026-04-13T07:33:59.269930+00:00
  Conversation-time planning clarification (2026-04-13): full MVP planning/decomposition for the native `imp` delegation tool can proceed now without waiting on new blocker answers. The repo-grounded implementation path 81.5 -> 81.6 -> 81.7 -> 81.8 is already specific enough to execute. Remaining open questions are real but non-blocking for MVP decomposition: (1) 81.9 durable ownership of `provider` / `thinking` versus transient runtime overrides, (2) later runner/attach-path integration under 45.4.x, and (3) deeper isolation/approval/productization follow-ons. These should be treated as post-MVP or adjacent architecture work unless the implementation goal expands beyond the current v1 two-layer delegation slice.

  ---
  2026-04-13T07:50:19.985350+00:00
  Conversation-time planning externalization (2026-04-13): the current v1 native `imp` delegation slice is fully plannable now with no new blocker questions. Durable MVP execution order remains: 81.5 extract the reusable single-unit worker runner out of `imp-cli::run_headless_mode()` into `imp-core::mana_worker`; 81.6 build the native unit-backed `imp` tool on top of that shared runner; 81.7 add the restricted transient `ad_hoc` child-delegation path; 81.8 enforce registration-time + execution-time mode policy and focused tests. Treat 81.9 (durable ownership of `provider` / `thinking`) as non-blocking architecture follow-up by assuming transient runtime overrides for v1. Treat root runner/attach-path migration work under 45.4.x and deeper hardening under 45.7.x as important post-MVP integration work, not blockers for planning or implementing the first two-layer native `imp` tool.

  ---
  2026-04-13T07:56:42.375084+00:00
  Delegation/subagent relevance from current memory-architecture discussion: improving mana from unit-centric memory to object-native durable memory directly improves `imp` subagent delegation quality. Child workers should inherit not just raw unit fields and attempt summaries, but compact checkpoint/handoff objects, relevant evidence/claim context, promoted lessons, and receipts that prevent repeated side effects. This strengthens `imp run <unit>` and native imp delegation by giving subagents a better specialized contract without making mana the final prompt renderer.

  ---
  2026-04-13T07:58:08.038065+00:00
  Concrete delegation/runtime decomposition from the current discussion: native imp delegation should continue to ride the canonical worker seam rather than inventing a prompt-only child runtime path for durable work. Near-term, v1 delegation can keep using the current unit-centric `WorkerAssignment`/`TaskContext` seam. Follow-on architecture work should define a `WorkerAssignment`/task-packet vNext that upgrades child-worker inheritance from raw unit fields + attempt summaries to object-backed context: latest checkpoint, accepted/current-state summary, evidence refs, lessons/runbooks, receipts, and open-decision/conflict context. This follow-on should remain non-blocking for the current 81.5 -> 81.8 MVP, but it is the right path for making subagents inherit more specialized durable state without making mana the final prompt renderer.

  ---
  2026-04-13T07:58:49.229660+00:00
  Created follow-on root architecture unit 245.1.1 to define the vNext mana→imp subagent handoff packet / `WorkerAssignment` shape for object-native memory. This is the contract-level follow-on that connects the memory migration to future delegation quality, but it is explicitly non-blocking for the current 81.5 -> 81.8 MVP implementation path.

  ---
  2026-04-13T19:03:55.822098+00:00
  2026-04-13 review findings from direct code inspection, externalized before reply:

  - Native `imp` delegation tool is still NOT implemented in repo reality. Confirmed gaps: no `imp/crates/imp-core/src/tools/imp.rs`; `imp-core/src/tools/mod.rs` has no `mod imp;`; `builder.rs::register_native_tools()` registers `ask/bash/edit/extend/mana/memory/multi_edit/read/write/scan/session_search/web` only; `config.rs::allowed_tool_names()` does not include `"imp"` for any non-Full mode.
  - Canonical single-unit `imp run` path IS real and reasonably well-wired today. `imp-cli::run_headless_mode()` loads `WorkerAssignment` through `imp_core::mana_worker::load_assignment_with_mana_dir()`, assembles file prefill via `assemble_prefill()`, builds `TaskContext` via `build_task_context()`, loads task facts via `mana_prompt_context::load_task_prompt_context()`, passes all of that into `ImpSession::create()`, prompts with `build_task_prompt()`, then optionally runs verify and `mana close` inline.
  - Prompt assembly from mana exists in two layers today: (1) session/system prompt assembly in `AgentBuilder` uses `mana_prompt_context::load_session_prompt_context()` for relevant facts + project memory status; (2) `imp run` task mode adds task-scoped facts filtered by `context_paths` using `load_task_prompt_context()`. This is real mana-backed prompt/context assembly, not just ad hoc text.
  - "Codemap" support is NOT yet a first-class surface in the current `imp run` path. The nearest present behavior is `context_prefill`: it detects explicit file/path references (including simple `:tail:N` and `:start-end` suffixes) and injects file contents as cached prefix messages. That is useful prefill, but it is not a semantic codemap / symbol graph / workspace map.
  - Related repo evidence: ripgrep finds no current runtime surface named `codemap` or `code map`; current code-intelligence work is tracked separately in root mana under `44.*` and related follow-ons. So if the requirement is 'prompt assembly based on mana and codemaps', mana is partially wired today, codemap is still a substantive gap rather than a near-finished integration.
  - Practical MVP implication for 81.5 -> 81.8: native `imp` tool implementation should reuse the current canonical `imp run` worker seam and inherit its mana-backed task context + prefill behavior first. Treat semantic codemap/context-map integration as an explicit follow-on seam unless the MVP scope is expanded intentionally.

  ---
  2026-04-13T19:28:49.903096+00:00
  2026-04-13 durable planning update from latest conversation:

  - User confirmed the execution priority: codemap integration is a separate feature and should not be folded into the current native imp-tool MVP.
  - V1 delegation context baseline is now explicit across the 81.5 -> 81.8 sequence: reuse existing mana-backed prompt assembly plus current reference-file prefill from the `imp run` path.
  - Longer-term desired direction remains a semantic/LSP-backed workspace map so a parent imp can curate richer subagent context similarly to mana job shaping, but that belongs to the separate follow-on codemap/context seam tracked under 81.10.
  - For hard or non-obvious architecture questions during implementation, the intended workflow is now also externalized: use a self-contained GPT-5.4-pro prompt grounded in current repo facts, then bring the answer back for review rather than letting workers silently invent unsupported behavior.
  - Practical implementation order remains unchanged and is now fully confirmed by user scope: 81.5 shared worker-runner extraction, 81.6 native unit-backed imp tool, 81.7 restricted ad hoc mode, 81.8 mode/policy/tests; 81.10 stays a separate planning thread for future codemap-backed context.

  ---
  2026-04-13T19:36:49.199221+00:00
  2026-04-13 workflow clarification from user: when escalating hard questions to GPT-5.4-pro, do not assume it knows `imp`, `mana`, or any repo-specific terminology. External prompts should be highly abstracted and system-design-oriented, with local project names and repo facts translated into generic roles/boundaries first. Bring the abstract answer back and then map it onto Tower locally.

  ---
  2026-04-13T20:02:27.979717+00:00
  2026-04-13 durable prompt-workflow decomposition from the latest conversation:

  Reusable abstract external-LLM question families for the native delegation/runtime thread:

  1. Shared runtime extraction question (maps to 81.5)
  - Ask in generic terms: what is the smallest correct reusable runtime seam to extract when a CLI/front-end currently owns too much of a single-task execution flow?
  - Use this when deciding what should move into the shared runtime versus remain in the CLI shell.

  2. Durable delegation API question (maps to 81.6)
  - Ask in generic terms: what should a v1 native delegation API look like when durable delegation must target tracked work and reuse the canonical single-task worker path?
  - Use this when refining tool schema, result envelope, and runtime ownership.

  3. Transient delegation constraint question (maps to 81.7)
  - Ask in generic terms: what are the safest useful constraints for transient sub-worker delegation so it remains helpful without becoming an untracked execution backdoor?
  - Use this when shaping the ad hoc path and capability restrictions.

  4. Nested-capability policy question (maps to 81.8)
  - Ask in generic terms: how should a system enforce both registration-time and execution-time policy for a nested delegation capability?
  - Use this when refining allowlists, runtime guards, and test strategy.

  Prompting rule:
  - Describe the system abstractly as a durable work substrate plus a live worker runtime.
  - State that an existing single-work-item execution path already handles load/context/prefill/run/verify.
  - State that richer semantic workspace mapping is explicitly deferred and should not be made an MVP blocker unless there is a strong reason.
  - Bring the external answer back and map it onto repo reality locally instead of treating the abstract answer as directly implementable truth.

  This prompt workflow is now part of the durable planning context for 81 rather than living only in chat.

  ---
  2026-04-13T20:45:38.215744+00:00
  Project-wide blocker review (2026-04-13): 81.5 is a current chokepoint for the delegation stack. Finishing reusable worker-runner extraction unblocks 81.6 (unit-backed native imp tool) and 81.7 (restricted ad hoc delegation), which together unblock 81.8 (mode-policy enforcement/tests). Treat 81.5 as the collapse point for downstream delegation implementation.

  ---
  2026-04-13T20:58:31.138727+00:00
  2026-04-13 durable external-model prompt template captured from the latest conversation:

  Reusable master-prompt structure for GPT-5.4-pro or similar general-purpose models:

  - Describe the system abstractly as:
    1. a durable work substrate that stores work items, dependencies, status, facts/evidence, attempts/history, and verification requirements
    2. a live worker runtime that reads a work item, assembles execution context, invokes tools/models, performs work, and reports results back
  - State current situation abstractly:
    - a real single-work-item execution path already exists
    - it already loads work, builds task context, includes durable facts, prefills referenced file/context snippets, runs the worker loop, and may verify/mark complete
    - too much of that flow still lives in a CLI/front-end layer instead of a reusable runtime function
    - a native delegation capability is planned so one worker can invoke another
    - v1 requires durable delegation to target tracked work, transient delegation to remain secondary/non-authoritative, current reference-file/context prefill to be sufficient, and richer semantic/LSP workspace mapping to be explicitly out of scope unless there is a strong reason otherwise
  - State constraints abstractly:
    - durable substrate owns durable truth/inherited state
    - live runtime owns execution behavior
    - avoid creating a second orchestration center outside the substrate
    - avoid duplicating the current single-work-item path
    - prefer extracting a reusable runtime seam over shelling out to a CLI command
    - treat semantic workspace mapping as later work by default
  - Require answer structure:
    1. direct answer
    2. smallest correct v1 design
    3. reusable runtime seam
    4. what remains in CLI/front-end
    5. durable delegation
    6. transient delegation constraints
    7. policy/safety implications
    8. risks/tradeoffs
    9. what to defer
    10. final recommendation
  - Operational rule:
    - vary only the final hard-question line for each use case (runner extraction, durable delegation API, transient delegation constraints, nested capability policy, etc.)
    - bring the abstract answer back and map it onto Tower locally rather than treating it as directly repo-aware guidance.

  This note supersedes earlier narrower prompt sketches by giving one reusable master prompt shape for the whole 81.x delegation/runtime thread.

  ---
  2026-04-13T21:25:41.847291+00:00
  2026-04-13 external-model design input review (GPT-5.4-pro abstract systems answer) captured as non-authoritative planning input to map onto Tower locally:

  High-alignment recommendations from the abstract answer:
  - Keep one reusable runtime-level single-work-item execution seam and make CLI/front-end a thin adapter. This strongly reinforces 81.5.
  - Keep durable delegation and transient helper delegation as two distinct capabilities/APIs rather than one generic spawn surface. This strongly reinforces the current 81.6 vs 81.7 split.
  - Enforce nested-delegation policy twice: registration-time capability graph plus execution-time checks against actual caller attempt/ancestry/budgets. This strongly reinforces 81.8.
  - Keep v1 context simple: durable substrate facts + explicit referenced-file/context prefill; explicitly defer semantic/LSP workspace-map expansion. This matches the current user-scoped MVP and 81.10 separation.

  Potentially useful follow-on details from the abstract answer that are not yet fully reflected in current 81.x units:
  - Add idempotency keys to durable and transient delegation calls to prevent duplicate child/helper work from model retries.
  - For durable delegation, record parent/child provenance and create the child relationship transactionally before child execution begins.
  - Treat child effective policy as the intersection of parent remaining authority/budget and child-profile policy, not a fresh unrestricted budget.
  - Keep durable child execution from depending on opaque parent in-memory state; durable child inputs should live in child spec, durable refs, or explicit snippet/file refs.

  Caveat for future workers:
  - This was an abstract repo-agnostic design answer, not inspected Tower repo evidence. Use it as design input to refine 81.5-81.8, not as direct proof that current code already supports these mechanisms.

  ---
  2026-04-13T21:46:37.711954+00:00
  2026-04-13 review of `5.4pro_answers.md` against current Tower planning:

  Mapping back onto current repo/work plan:
  - Strong fit / adopt into current implementation direction:
    - one canonical runtime-level tracked-work execution seam owned by imp-core (reinforces 81.5)
    - durable tracked-work delegation and transient helper delegation as separate capabilities (reinforces 81.6 vs 81.7)
    - CLI/front-end stays thin and should stop owning execution behavior (reinforces 81.5)
    - two-layer policy model: registration-time plus execution-time checks (reinforces 81.8)
    - keep v1 context simple: mana-backed facts + explicit reference-file/context prefill, defer semantic workspace map (matches current MVP scope / 81.10 split)
  - Good likely v1 additions to carry forward if implementation stays tractable:
    - idempotency keys for durable and transient delegation calls to avoid duplicate child/helper work on retries
    - explicit provenance/lineage recording before durable child execution begins
    - child/helper authority should be bounded by intersection of parent remaining authority/budget and child profile constraints
  - Important Tower-specific caution / do not over-import from the abstract answer yet:
    - current Tower contracts do not yet have fields like `parent_attempt_id`, `root_work_item_id`, `delegation_depth`, or generalized worker profiles in `tower-contracts::worker`; treat those as optional shape directions, not mandatory MVP schema changes
    - v1 should not expand into scheduler-native suspension/resume, broad target-profile graphs, or a second substrate transaction model unless repo-grounded implementation work proves they are needed
    - keep 81.5 -> 81.8 focused on the native imp-tool MVP rather than importing every stronger near-term or longer-term abstraction from the abstract answer in one pass

  Operational interpretation:
  - Use the abstract answer as validation that current decomposition is pointed the right way.
  - Keep implementation conservative: land the shared runner first, then the unit-backed tool, then the transient helper path, then mode/policy enforcement.
  - Pull in idempotency/provenance/budget-intersection ideas only where they can be implemented cleanly without broadening the MVP into a larger contract migration.

  ---
  2026-04-13T22:44:06.514416+00:00
  2026-04-13 durable decomposition refresh from the latest review:

  What remains for the native `imp` tool in the current MVP is now restated explicitly:

  1. 81.5 — shared runner extraction
  - Extract the real single-unit worker execution flow out of `imp-cli::run_headless_mode()` and into one authoritative `imp-core` seam in `mana_worker.rs`.
  - Preserve current mana-backed prompt assembly and reference-file/context prefill.
  - Keep `imp-cli` as a thin adapter for flags, rendering, and exit codes.

  2. 81.6 — native unit-backed `imp` tool
  - Create/register `tools/imp.rs`.
  - Implement only the durable `delegate` + `mode=unit` path first.
  - Reuse the shared runner from 81.5 instead of shelling out.
  - Return structured result details to the parent runtime.

  3. 81.7 — restricted transient helper path
  - Add the secondary `mode=ad_hoc` / transient helper flow.
  - Keep it read-oriented, non-authoritative, and explicitly bounded.
  - No automatic durable write-back.

  4. 81.8 — gates/tests
  - Allow the native `imp` tool only in `Full` and `Orchestrator` for v1.
  - Add execution-time guard behavior plus focused tests.
  - Keep policy implementation Tower-small unless the implementation proves more machinery is necessary.

  Good-if-clean carry-ins from the external-model review that should not broaden the MVP by themselves:
  - idempotency keys on delegation/helper calls
  - explicit parent/child provenance
  - child authority bounded by parent remaining authority rather than refreshed from scratch

  This note is the current durable answer to the question 'what remains for the native imp tool?'.

  ---
  2026-04-13T23:20:09.594254+00:00
  2026-04-13 durable implementation decomposition externalized from the latest code-grounded review:

  The concrete native-imp-tool MVP plan is now pushed into the child implementation units and summarized here at the parent level.

  Code-grounded remaining stack:
  - 81.5: extract the shared imp-core worker runner from the CLI-owned `imp run` path
  - 81.6: implement the native unit-backed `imp` tool on top of that runner
  - 81.7: extend the same tool with the bounded transient/ad-hoc helper path
  - 81.8: add mode/policy gates and focused tests

  Important implementation tension discovered from inspection:
  - `imp-cli::run_headless_mode()` currently both owns runtime setup/finalization logic and streams live events from `ImpSession`.
  - A naive fully-opaque `run_worker_assignment(...)` helper would make CLI/event streaming awkward.
  - Therefore 81.5 should likely keep one authoritative runtime home in `imp-core::mana_worker` while using either:
    1. one public shared runner plus carefully exposed session/result handles, or
    2. a small internal prepare/run/finalize split that preserves caller-side event streaming without re-spreading execution semantics back into CLI.
  - This is an implementation-shape constraint, not a reason to change the overall 81.5 -> 81.8 ordering.

  Child-unit checklist placement now externalized:
  - 81.5 carries the proposed shared runner/options/outcome shape and the exact code currently living in `run_headless_mode()` that should move.
  - 81.6 carries the proposed `tools/imp.rs` schema/registration/result shape for the durable unit-backed path.
  - 81.7 carries the proposed transient helper/ad-hoc shape and bounded child-session approach.
  - 81.8 carries the concrete registration-time allowlist + execution-time guard + focused-test checklist.

  This parent note is the durable top-level answer to the instruction to externalize the plan into mana now; future workers should start from 81 plus 81.5-81.8 rather than reconstructing the implementation checklist from chat.

  ---
  2026-04-14T00:16:22.526667+00:00
  2026-04-13 durable decomposition update from the latest 81.5 refactor review:

  Refined implementation shape for the first blocker unit (81.5) is now explicit:
  - Do not start by collapsing the current CLI path into one fully opaque `run_worker_assignment(...)` helper, because current `imp-cli::run_headless_mode()` both owns runtime setup/finalization logic and streams live events from `ImpSession` for human/json output.
  - The recommended first extraction is Tower-small and preserves streaming:
    1. `WorkerRunOptions`
    2. `PreparedWorkerRun`
    3. `prepare_worker_run(...)`
    4. `finalize_worker_run(...)`
  - In this shape, `imp-core::mana_worker` becomes the runtime owner of:
    - prefill assembly
    - task context + mana facts
    - `SessionOptions` construction
    - `ImpSession::create()`
    - task prompt construction
    - verify/close timing
    - structured outcome creation
  - `imp-cli::run_headless_mode()` becomes a thin adapter that keeps only:
    - startup timing marks
    - printing prefill warnings/notices
    - invoking `session.prompt(...)`
    - consuming `recv_event()` for human/json output
    - `session.wait()`
    - final bool / exit-code translation
  - After 81.5 lands and the native tool call sites exist, a convenience `run_worker_assignment(...)` wrapper can still be introduced for non-streaming callers if it remains a thin compose-over-prepare/finalize layer rather than a second execution center.

  This parent note exists so future workers can understand the 81.5 design tension from the root 81 thread without needing to inspect only the child notes.

  ---
  2026-04-14T00:32:21.947709+00:00
  2026-04-13 durable 81.5 API-shape update from the latest code-grounded review:

  The first concrete shared-runner extraction proposal is now pinned down against current Tower reality and stored on 81.5. Parent-level summary here:
  - Keep 81.5 compatible with the existing `tower-contracts::worker` surface (`WorkerAssignment`, `WorkerResult`, `WorkerStatus`) rather than forcing lineage/depth contract expansion now.
  - Preferred first extraction shape in `imp-core::mana_worker` is:
    - `WorkerRunOptions`
    - `PreparedWorkerRun`
    - `WorkerRunOutcome`
    - `prepare_worker_run(...)`
    - `finalize_worker_run(...)`
  - This shape lets callers keep the current event-streaming loop (`prompt` -> `recv_event` -> `wait`) while moving setup/finalization ownership into `mana_worker.rs`.
  - If the exact ownership of `PreparedWorkerRun` proves awkward in Rust, a local variant such as `PreparedWorkerArtifacts + ImpSession` is acceptable, but the architectural goal remains the same: one runtime home for execution semantics, thin caller adapters, no second execution model.

  This note exists so the parent 81 thread contains the current recommended API-shape answer without requiring workers to inspect only the child unit.

  ---
  2026-04-14T00:33:14.496940+00:00
  2026-04-14 durable decomposition refresh from the latest 81.5 API-shape review:

  The current top-level plan for the native delegation MVP now includes an explicit 81.5 extraction shape:
  - preserve current event streaming in the first extraction
  - move setup/finalization ownership into `imp-core::mana_worker`
  - use `prepare_worker_run(...)` + caller-owned prompt/event/wait + `finalize_worker_run(...)` as the first shared-runner boundary
  - defer any convenience all-in-one `run_worker_assignment(...)` wrapper until after the event-streaming path is preserved cleanly

  This parent note exists so future workers can see the shared-runner decomposition from the root 81 thread without reconstructing it from chat or only from the child unit.

  ---
  2026-04-14T01:59:48.974967+00:00
  2026-04-14 durable implementation refresh from the latest 81.5 code-grounded rewrite plan:

  The first PR-sized 81.5 slice is now explicit enough to implement cold:
  - keep `run_headless_mode()`'s startup timing, assignment/config load, event rendering loop, and final bool/exit-code translation in `imp-cli`
  - move prefill/task-context/task-facts/session-creation/task-prompt logic into `prepare_worker_run(...)`
  - move verify/auto-close/final outcome synthesis into `finalize_worker_run(...)`
  - leave current human/json output helpers unchanged in the first pass

  This matters because it turns 81.5 from general architecture intent into a surgical edit plan that preserves current UX while relocating execution ownership into `imp-core::mana_worker`.

  Future workers should treat this as the direct implementation handoff for the first extraction slice rather than re-deriving the split from `run_headless_mode()` manually.

  ---
  2026-04-14T02:00:24.497069+00:00
  2026-04-14 explicit externalization checkpoint requested by user before continuing:

  The durable native-imp-tool decomposition described in the latest exchange has been externalized in root mana and is now considered the authoritative planning state.

  Current externally-recorded plan relevant to the immediate blocker:
  - Parent unit `81` carries the top-level native-imp-tool MVP decomposition and repeated scope decisions.
  - Child unit `81.5` now carries:
    - the reason the current CLI path must be split without losing event streaming
    - the preferred first extraction shape (`WorkerRunOptions`, `PreparedWorkerRun`, `WorkerRunOutcome`, `prepare_worker_run(...)`, `finalize_worker_run(...)`)
    - the proposed type/signature bundle grounded in current `tower-contracts::worker` reality
    - the surgical `run_headless_mode()` ownership split showing exactly what stays in CLI versus what moves into `mana_worker.rs`
  - Child units `81.6`, `81.7`, and `81.8` already carry the next-slice implementation checklists for the native tool, transient helper mode, and policy/tests.

  This note exists specifically as a visible externalization checkpoint responding to the user's instruction to keep the durable plan in mana before continuing.

  ---
  2026-04-14T02:39:32.298457+00:00
  2026-04-14 durable 81.6 API-shape update from direct tool-surface inspection:

  The first-pass native `imp` tool contract is now pinned down and stored on child unit 81.6.

  Parent-level summary:
  - create `imp-core/src/tools/imp.rs` with tool name `imp`
  - first-pass schema should reserve `action=delegate` and `mode=unit|ad_hoc`, but 81.6 implements only the durable `mode=unit` path and returns a clear not-yet-supported error for `mode=ad_hoc`
  - `unit_id` is the only required mode-specific field in 81.6
  - runtime override fields (`model`, `provider`, `thinking`, `max_turns`, `max_tokens`, `system_prompt`, `no_tools`, `defer_verify`) may be accepted and passed through to the shared runner
  - success results should follow the existing tool convention: concise human-readable `content` plus structured JSON `details` with delegation mode, durability, unit id, status, verify outcome, and lightweight run metadata
  - do not shell out from the tool; call the shared 81.5 runtime seam or a thin non-streaming helper layered over it

  This parent note exists so the root 81 thread now contains both the 81.5 shared-runner decomposition and the 81.6 first-pass native-tool contract.

  ---
  2026-04-14T02:40:09.978080+00:00
  2026-04-14 explicit externalization checkpoint requested by user before continuing:

  The latest native-imp-tool decomposition described in chat has been externalized in root mana and is now the authoritative planning state for the immediate next slices.

  What is now durable at root scope:
  - `81.5` carries the shared-runner extraction contract:
    - preserve current `ImpSession` event streaming
    - move setup/finalization ownership into `imp-core::mana_worker`
    - first extraction shape is `WorkerRunOptions` + `PreparedWorkerRun` + `WorkerRunOutcome` with `prepare_worker_run(...)` and `finalize_worker_run(...)`
    - `run_headless_mode()` rewrite split is specified line-by-line at a surgical ownership level
  - `81.6` carries the first-pass native `imp` tool contract:
    - create `tools/imp.rs`
    - tool name `imp`
    - schema reserves `action=delegate` and `mode=unit|ad_hoc`
    - 81.6 implements only the durable `mode=unit` path
    - `mode=ad_hoc` returns a clear not-yet-supported error until 81.7
    - success result shape is concise text plus structured details including delegation mode, durability, unit id, verify outcome, and lightweight run metadata
  - Root `81` now contains both of these contracts so future workers can inherit the runner/tool decomposition from the top-level thread without reconstructing it from chat.

  This note exists specifically to satisfy the instruction to externalize the durable plan into mana before continuing.

  ---
  2026-04-14T03:51:22.082324+00:00
  2026-04-14 durable wiring/policy update from direct builder/config inspection:

  The native-imp-tool plan now includes an exact repo-grounded wiring/gating slice recorded on 81.8.

  Parent-level summary:
  - 81.6 wiring path is straightforward and now explicit:
    - add `pub mod imp;` to `imp-core/src/tools/mod.rs`
    - register `ImpTool` in `imp-core/src/builder.rs::register_native_tools(...)`
  - 81.8 registration-time gate is now explicit:
    - add `"imp"` only to `AgentMode::Orchestrator` in `imp-core/src/config.rs`
    - rely on Full's implicit-all behavior for Full mode
  - 81.8 execution-time gate is now explicit:
    - `tools/imp.rs` should allow execution only in `Full` or `Orchestrator`
    - blocked modes should receive a clear structured error even if invoked directly
  - focused tests now have a concrete home and shape:
    - config/mode tests for allowlist behavior
    - registry/tool tests for inclusion/exclusion and blocked execution behavior

  This note exists so the root 81 thread now carries the shared-runner shape, first-pass tool contract, and exact wiring/gating hooks together.

  ---
  2026-04-14T03:52:06.058214+00:00
  2026-04-14 explicit externalization checkpoint requested by user before continuing:

  The durable implementation state just described in chat has now been externalized in root mana.

  Current concretely specified slices on the native imp-tool thread:
  - 81.5 — shared-runner extraction plan:
    - preserve `ImpSession` event streaming in the first extraction
    - move setup/finalization ownership into `imp-core::mana_worker`
    - use `WorkerRunOptions` / `PreparedWorkerRun` / `WorkerRunOutcome` with `prepare_worker_run(...)` and `finalize_worker_run(...)`
    - `run_headless_mode()` ownership split is already recorded as a surgical edit plan
  - 81.6 — first-pass native `ImpTool` contract:
    - create/register `tools/imp.rs`
    - reserve `action=delegate` and `mode=unit|ad_hoc`
    - implement only durable `mode=unit` in this slice
    - use concise text plus structured details as the result contract
  - 81.8 — exact wiring/gating hooks:
    - `pub mod imp;` in `tools/mod.rs`
    - register `ImpTool` in `builder.rs`
    - allow `imp` only in `Full` and `Orchestrator`
    - add runtime guard in `tools/imp.rs`
    - add focused allowlist/registry/blocked-execution tests

  What remains to be sharpened next at the same level of precision:
  - 81.7 transient/ad-hoc helper behavior, including child mode, tool narrowing, fixed v1 limits, and result envelope.

  This note exists specifically as a visible root-scope checkpoint that the latest durable decomposition has been written into mana before continuing.

  ---
  2026-04-14T03:57:29.397265+00:00
  2026-04-14 durable 81.7 contract update from direct runtime-surface inspection:

  The remaining transient/ad-hoc helper slice is now also pinned down and stored on child unit 81.7.

  Parent-level summary:
  - use `ImpSession::create(...)` directly for the first-pass transient helper path
  - build the child session as:
    - `SessionChoice::InMemory`
    - `mode: Some(AgentMode::Reviewer)`
    - no task/facts/context-prefill inheritance in the first pass
    - optional runtime overrides only
  - use `session.prompt_and_wait(...)` for the first pass; no event-streaming complexity required here
  - extract best-effort final assistant text from `session.session_manager().get_active_messages()` and return it as non-authoritative helper output
  - keep the helper explicit-input-only, non-durable, non-recursive, and reviewer-mode-narrow by default

  This means root mana now contains concrete implementation contracts for all four v1 slices:
  - 81.5 shared-runner extraction
  - 81.6 durable unit-backed native tool
  - 81.7 transient/ad-hoc helper path
  - 81.8 wiring/gating/tests

  Future workers should now treat the remaining task as implementation sequencing and code execution rather than further high-level design invention unless direct repo evidence forces a local change.

  ---
  2026-04-14T03:58:22.894138+00:00
  2026-04-14 explicit externalization checkpoint requested by user before continuing:

  The durable native-imp-tool plan just described in chat has been externalized in root mana and is now the authoritative implementation-ready planning state.

  Authoritative root-scope decomposition now recorded:
  - 81.5 — shared-runner extraction from the current CLI-owned `imp run` path, preserving `ImpSession` event streaming while moving setup/finalization ownership into `imp-core::mana_worker`
  - 81.6 — first-pass native `ImpTool` implementing durable `action=delegate` + `mode=unit` over the shared runtime seam
  - 81.7 — bounded transient/ad-hoc helper path using an in-memory `Reviewer`-mode child session with explicit-input-only, non-authoritative behavior
  - 81.8 — exact module wiring, registration-time mode allowlisting, execution-time runtime guard, and focused tests

  Current planning status:
  - The v1 design/decomposition layer is complete enough to start implementation from mana.
  - The next meaningful work is either:
    1. execution-ordered implementation checklisting, or
    2. direct code work beginning with 81.5.
  - Additional abstract decomposition is no longer the default next step unless direct repo evidence forces a local change.

  This note exists specifically to satisfy the user's instruction to externalize the durable plan into mana before continuing.

  ---
  2026-04-14T04:05:13.781845+00:00
  2026-04-14 execution-ordered implementation checklist externalized from the completed v1 planning layer:

  Recommended implementation order:
  1. 81.5 — shared runner extraction
  2. 81.6 — durable unit-backed native `ImpTool`
  3. 81.7 — bounded transient/ad-hoc helper path
  4. 81.8 — final wiring/policy/test closure

  Recommended PR-sized sequence:

  PR1 / 81.5a — prepare/finalize extraction without behavior change
  - Add `WorkerRunOptions`, `PreparedWorkerRun`, `WorkerRunOutcome` to `imp-core::mana_worker`.
  - Implement `prepare_worker_run(...)` with current prefill/task/facts/session/prompt setup.
  - Implement `finalize_worker_run(...)` with current verify/close/outcome logic.
  - Rewrite `run_headless_mode()` to use the new prepare -> prompt/event/wait -> finalize flow.
  - Verify by running the existing `imp run <unit>` path on a known unit and confirming current headless behavior still works.

  PR2 / 81.6a — durable `ImpTool` skeleton + wiring
  - Create `imp-core/src/tools/imp.rs`.
  - Implement schema parsing + mode guard skeleton.
  - Register tool in `tools/mod.rs` and `builder.rs`.
  - Add config allowlist entry for `Orchestrator` and runtime guard for `Full|Orchestrator`.
  - Return not-yet-supported for `mode=ad_hoc`.
  - Verify with focused tests that the tool appears only where intended and blocked modes fail clearly.

  PR3 / 81.6b — durable unit-backed execution path
  - Connect `mode=unit` to assignment loading + shared runner.
  - Return concise text + structured details.
  - If clean, add idempotency-key passthrough/echo and lightweight provenance fields.
  - Verify against a small known mana unit using the native tool from an allowed mode or focused integration test harness.

  PR4 / 81.7 — transient helper path
  - Extend `tools/imp.rs` with `mode=ad_hoc`.
  - Build in-memory reviewer-mode child session.
  - Use `prompt_and_wait(...)` and extract best-effort final assistant text.
  - Keep explicit-input-only and non-authoritative behavior.
  - Verify with focused tool tests.

  PR5 / 81.8 — focused test/policy hardening
  - Add allowlist tests.
  - Add direct blocked-execution tests.
  - Add result-shape tests for `mode=unit` and `mode=ad_hoc`.
  - If idempotency lands earlier, add one dedupe-oriented test.

  Execution notes:
  - 81.6 depends on 81.5 because the native tool should call the shared runner, not shell out.
  - 81.7 depends on 81.6 because it extends the same tool surface.
  - 81.8 should close after both 81.6 and 81.7 because policy/tests should validate the final v1 surface, not a partial one.

  This note is the current durable answer to 'what should be implemented next, in what order?'.

  ---
  2026-04-14T04:05:45.550752+00:00
  2026-04-14 explicit externalization checkpoint requested by user before continuing:

  The execution-ordered implementation plan described in the latest exchange has been externalized in root mana.

  Durable implementation order now recorded:
  1. 81.5 — shared runner extraction
  2. 81.6 — durable unit-backed native `ImpTool`
  3. 81.7 — bounded transient/ad-hoc helper path
  4. 81.8 — final wiring/policy/test hardening

  Durable PR-sized sequence now recorded:
  - PR1 / 81.5a: prepare/finalize extraction with no intended behavior change
  - PR2 / 81.6a: native tool skeleton + registration + mode gate
  - PR3 / 81.6b: durable `mode=unit` execution path
  - PR4 / 81.7: transient helper path
  - PR5 / 81.8: focused policy/test hardening

  Graph state now aligned with execution order:
  - 81.6 depends on 81.5
  - 81.7 depends on 81.6
  - 81.8 depends on both 81.6 and 81.7

  This note exists specifically as the visible root-scope checkpoint that the sequencing layer has been externalized into mana before continuing.

  ---
  2026-04-14T04:06:29.777431+00:00
  2026-04-14 explicit externalization checkpoint requested by user before continuing:

  The durable plan/decomposition just described in chat has been externalized in root mana and is now the authoritative planning state.

  What is now explicitly durable at root scope:
  - The v1 native `imp` tool is decomposed across 81.5-81.8.
  - Each child unit carries a repo-grounded implementation contract:
    - 81.5 shared-runner extraction preserving current `ImpSession` event streaming
    - 81.6 durable unit-backed `ImpTool`
    - 81.7 bounded transient/ad-hoc helper path
    - 81.8 wiring, mode gates, and focused tests
  - The execution order and dependency structure are explicit:
    - 81.6 depends on 81.5
    - 81.7 depends on 81.6
    - 81.8 depends on 81.6 and 81.7
  - The current authoritative immediate next step is now explicit:
    - PR1 / 81.5a
    - add `prepare_worker_run(...)`
    - add `finalize_worker_run(...)`
    - rewrite `run_headless_mode()` around prepare -> prompt/event/wait -> finalize
    - preserve current event streaming and UX
    - avoid intentional behavior changes in this first slice

  This note exists specifically to satisfy the user's request to externalize the durable plan into mana before continuing.

  ---
  2026-04-14T04:55:34.679698+00:00
  2026-04-14 implementation progress update from direct code changes + verification:

  Landed code slices in repo reality:

  1. 81.5a shared-runner extraction landed
  - Added new runtime seam in `imp/crates/imp-core/src/mana_worker.rs`:
    - `WorkerRunOptions`
    - `PreparedWorkerRun`
    - `WorkerRunOutcome`
    - `prepare_worker_run(...)`
    - `finalize_worker_run(...)`
    - convenience non-streaming `run_worker_assignment(...)`
  - Rewrote `imp/crates/imp-cli/src/main.rs::run_headless_mode()` to use the new flow:
    - prepare -> prompt/event/wait -> finalize
  - Preserved CLI event streaming / human-json output behavior in the first pass.

  2. 81.6 durable native `ImpTool` landed
  - Created `imp/crates/imp-core/src/tools/imp.rs`.
  - Implemented native tool name `imp` with first-pass schema for:
    - `action=delegate`
    - `mode=unit|ad_hoc`
  - Implemented only durable `mode=unit` execution.
  - `mode=ad_hoc` currently returns a clear not-yet-supported error (81.7 still deferred).
  - Durable path loads a mana worker assignment and calls the new shared runner instead of shelling out.
  - Returns concise text plus structured details.

  3. 81.8 wiring/gating landed
  - Added `pub mod imp;` in `imp-core/src/tools/mod.rs`.
  - Registered `ImpTool` in `imp-core/src/builder.rs`.
  - Added `"imp"` to `AgentMode::Orchestrator` allowlist in `imp-core/src/config.rs`.
  - Added runtime guard in `tools/imp.rs` so only `Full | Orchestrator` may execute it.

  Verification completed:
  - `cargo check -p imp-core` ✅
  - `cargo check -p imp-cli` ✅
  - `cargo test -p imp-core schema_requires_unit_id_for_unit_mode` ✅
  - `cargo test -p imp-core blocked_modes_fail_clearly` ✅
  - `cargo test -p imp-core ad_hoc_reserved_until_follow_up` ✅
  - `cargo test -p imp-core non_orchestrator_modes_block_imp` ✅
  - `cargo test -p imp-core agent_mode_orchestrator_allows_read` ✅

  Remaining gap relative to the original 81.x stack:
  - 81.7 transient/ad-hoc helper path is still not implemented; it remains reserved in schema and explicitly returns not-yet-supported.

  Minor residual issue observed during verification:
  - `cargo check -p imp-cli` still emits a pre-existing/adjacent warning for an unused local `config_path` in `imp-cli/src/main.rs:1188`. This did not block the current slice and was left untouched because it is not part of the native imp-tool behavior.

  This note captures repo reality after the first implementation pass rather than just planning intent.

  ---
  2026-04-14T05:01:20.149396+00:00
  2026-04-14 implementation progress update after 81.7 work:

  81.7 landed in repo reality.

  What changed:
  - Extended `imp-core/src/tools/imp.rs` so `mode=ad_hoc` is now implemented instead of returning not-yet-supported.
  - Added schema requirement that `prompt` is required when `mode=ad_hoc`.
  - Implemented the transient helper path by constructing an in-memory `ImpSession` with:
    - `SessionChoice::InMemory`
    - `mode: Some(AgentMode::Reviewer)`
    - no task/facts/context-prefill inheritance
    - optional runtime overrides only
  - Execution uses `prompt_and_wait(...)`.
  - Best-effort final assistant text is extracted from session history and returned in structured tool details.
  - Result contract now distinguishes:
    - durable `mode=unit`
    - transient `mode=ad_hoc` with `durable: false`

  Additional verification completed:
  - `cargo test -p imp-core schema_requires_prompt_for_ad_hoc_mode` ✅
  - `cargo test -p imp-core blocked_modes_fail_clearly` ✅
  - `cargo test -p imp-core ad_hoc_returns_transient_details` ✅
  - `cargo check -p imp-core` ✅
  - `cargo check -p imp-cli` ✅ (still with the unrelated `config_path` warning in `imp-cli/src/main.rs:1188`)

  Net result after the full implementation pass:
  - 81.5 shared-runner extraction landed
  - 81.6 durable native `ImpTool` landed
  - 81.7 transient/ad-hoc helper path landed
  - 81.8 wiring/gating/tests landed to the intended v1 level

  This note reflects repo reality after implementing the remaining native-imp-tool MVP slice.

  ---
  2026-04-14T05:07:06.555252+00:00
  2026-04-14 repo review pass after implementation:

  Verification findings:
  - `cargo check -p imp-core` passed
  - `cargo check -p imp-cli` passed
  - focused imp-tool tests passed
  - semgrep quality scan found no blocking findings
  - qlty could not run because the repository is not initialized for qlty (`qlty init` required), so there is no qlty-based review signal

  Important repo-grounded review finding:
  - The durable `imp` tool path in `imp-core/src/tools/imp.rs` currently builds `WorkerRunOptions` with `lua_loader: None` for `mode=unit`.
  - By contrast, the canonical CLI `imp run` path uses `build_lua_loader(...)` and passes a loader into the shared runner.
  - That means native durable delegation does NOT yet inherit the full Lua-extension tool surface that `imp run` can load.
  - Since the original goal was full imp-run functionality reuse, this is a real parity gap worth fixing before calling the implementation fully complete/commit-ready.

  Minor nits from the review pass:
  - `ImpTool::parameters()` still describes `mode='ad_hoc'` as reserved for follow-up work even though it is now implemented.
  - `cargo check -p imp-cli` still emits an unrelated unused-variable warning for `config_path` in `imp-cli/src/main.rs:1188`.

  Git-state observation:
  - At the Tower root, the imp source files show no working-tree diff relative to HEAD during this review pass; only the root mana unit markdown and an unrelated untracked `imp/draft.html` remain uncommitted. Future commit instructions should account for that actual git state rather than assuming the imp source edits are currently unstaged.

  ---
  2026-04-14T05:08:14.280983+00:00
  2026-04-14 explicit externalization checkpoint requested by user before continuing:

  Repo review findings from the latest pass have now been externalized in root mana.

  Durable post-review state:
  - The native `imp` tool implementation landed and verified at the compile/test level.
  - However, there is one meaningful parity gap relative to the original goal of reusing full `imp run` functionality: durable `mode=unit` delegation in `imp-core/src/tools/imp.rs` currently builds `WorkerRunOptions` with `lua_loader: None`, while the canonical CLI `imp run` path supplies a Lua-extension loader via `build_lua_loader(...)`.
  - Result: delegated unit runs triggered through the native `imp` tool may not inherit the same Lua-extension tool surface as direct `imp run` executions.
  - Additional minor cleanup noted in the review:
    - `ImpTool::parameters()` text still says `ad_hoc` is reserved for follow-up work even though it is now implemented.
    - `cargo check -p imp-cli` still emits an unrelated warning for unused `config_path` in `imp-cli/src/main.rs:1188`.
  - Commit-readiness conclusion from the review pass:
    - do not call the native imp-tool work fully commit-ready until the Lua-loader parity gap is fixed and the stale ad-hoc description text is updated.

  Git-state note from the same review:
  - At the Tower root, the imp source files appeared to match `HEAD` during inspection; only the root mana markdown and unrelated `imp/draft.html` showed as uncommitted. Future commit handling should account for actual git state rather than assuming the imp source edits are currently unstaged.

  This note exists specifically to satisfy the user's instruction to externalize the durable review/decomposition into mana before continuing.

  ---
  2026-04-14T06:43:56.104468+00:00
  2026-04-14 follow-up implementation result for 81.11 (post-review parity fix):

  The Lua-extension parity gap identified in the repo review has been fixed.

  What changed:
  - Introduced a cloneable runtime Lua-loader handle in `imp-core/src/tools/mod.rs` (`LuaToolLoader`) and propagated it through the live runtime/session/tool context seams.
  - `SessionOptions.lua_loader` and `WorkerRunOptions.lua_loader` now use the cloneable loader handle type.
  - `AgentBuilder` / `Agent` / `ToolContext` / Lua bridge/sandbox paths now carry the inherited loader through execution context.
  - CLI `build_lua_loader(...)` now returns the shared cloneable loader handle and all current CLI session construction paths use it.
  - Native durable `imp` delegation (`tools/imp.rs`, `mode=unit`) now forwards `ctx.lua_tool_loader.clone()` into the shared runner instead of passing `lua_loader: None`.
  - Updated stale `ImpTool::parameters()` description text so `mode=ad_hoc` is described as implemented bounded helper delegation rather than reserved future work.

  Verification after the parity fix:
  - `cargo check -p imp-core` ✅
  - `cargo check -p imp-cli` ✅
  - `cargo test -p imp-core schema_requires_unit_id_for_unit_mode` ✅
  - `cargo test -p imp-core schema_requires_prompt_for_ad_hoc_mode` ✅
  - `cargo test -p imp-core blocked_modes_fail_clearly` ✅
  - `cargo test -p imp-core ad_hoc_returns_transient_details` ✅

  Residual note:
  - `cargo check -p imp-cli` still emits the unrelated unused-variable warning for `config_path` in `imp-cli/src/main.rs:1188`.

  Updated review conclusion:
  - The previously identified parity blocker is resolved.
  - No remaining repo-reviewed blockers are known for commit-readiness on the native imp-tool MVP itself, aside from the unrelated non-blocking warning above.

  ---
  2026-04-14T06:45:16.539324+00:00
  2026-04-14 explicit externalization checkpoint requested by user before continuing:

  The latest durable repo-review conclusion has now been externalized in root mana.

  Current root-scope state for the native `imp` tool thread:
  - The v1 native `imp` tool MVP is implemented in repo reality.
  - The follow-up parity fix from 81.11 is landed:
    - cloneable Lua loader handle added in `imp-core`
    - loader propagated through runtime/session/tool context seams
    - native durable `mode=unit` delegation now forwards the inherited loader and therefore matches canonical `imp run` more closely on Lua-extension/tool-surface behavior
    - stale `ad_hoc` parameter/description text was updated to match reality
  - Verification after the parity fix passed:
    - `cargo check -p imp-core`
    - `cargo check -p imp-cli`
    - focused imp-tool schema/blocked-mode/transient-detail tests
  - Updated durable review conclusion:
    - there are no remaining repo-reviewed blockers for the native `imp` tool MVP itself
    - the work is now considered commit-ready
  - Remaining residual note:
    - `imp-cli/src/main.rs:1188` still emits an unrelated non-blocking unused-variable warning for `config_path`

  This note exists specifically to satisfy the user's instruction to externalize the durable plan/review conclusion into mana before continuing.

  ---
  2026-04-14T06:45:34.150060+00:00
  2026-04-14 parity-fix child checkpoint collapsed back onto the parent thread for durability:

  The follow-up parity-fix slice that resolved the last known repo-review blocker had the following durable end state:
  - native durable `imp` unit delegation now inherits the same cloneable Lua-extension loader path used by canonical `imp run`
  - `ImpTool::parameters()` / description text no longer claims `mode=ad_hoc` is only future work
  - focused imp-tool schema/blocked-mode/transient-detail verification passed along with `cargo check -p imp-core` and `cargo check -p imp-cli`
  - only residual note after the fix is the unrelated non-blocking `config_path` warning in `imp-cli/src/main.rs:1188`

  This parent note preserves the parity-fix conclusion directly on root 81 even though the follow-up child lookup was not available by id during the latest externalization step.

  ---
  2026-04-24T05:37:29.290177+00:00
  Graph hygiene pass 7 2026-04-24: verify passed (`docs/rebuild/imp-delegation-tool-and-runtime.md` exists and contains imp run/WorkerAssignment/mana/delegation markers). Closing stale in_progress design unit; follow-up implementation units already exist separately.
labels:
- architecture
- imp
- mana
- tools
- orchestration
- delegation
verify: cd /Users/asher/imp && test -f docs/rebuild/imp-delegation-tool-and-runtime.md && rg -q 'imp run' docs/rebuild/imp-delegation-tool-and-runtime.md && rg -q 'WorkerAssignment' docs/rebuild/imp-delegation-tool-and-runtime.md && rg -q 'mana' docs/rebuild/imp-delegation-tool-and-runtime.md && rg -q 'delegation' docs/rebuild/imp-delegation-tool-and-runtime.md
checkpoint: cfc6cee411f353d311fb044002b2c84346ab1ac4
verify_hash: '30b3c150f0a6cfd72f7d6bbb64bfb3aea99f092b99803aae2b5b427bd96a6568'
kind: job
attempt_log:
- num: 1
  outcome: abandoned
  agent: imp
  started_at: '2026-04-10T02:58:58.238240Z'
  finished_at: '2026-04-24T05:37:22.532084Z'
decisions:
- 'Working direction: v1 should prefer a two-layer delegation model. `imp` owns live child-worker invocation, but durable or mutating delegation must externalize into mana and reuse the canonical `imp run <unit-id>` path. Any direct prompt-only child calls, if allowed in v1, should remain explicitly transient, read-oriented, and non-authoritative until a stronger runner/policy contract exists.'
- 'Implementation decomposition rule now externalized: 81.5 extracts the reusable canonical worker runner from `run_headless_mode()` into `imp-core::mana_worker`; 81.6 builds only unit-backed native `imp` delegation on top of that runner; 81.7 adds the restricted transient `ad_hoc` path separately; 81.8 applies mode/tool gating and tests after both delegation paths exist. This ordering is intentional so workers do not invent alternate runtime centers or mix policy work into runtime extraction.'
- Treat the vNext subagent handoff packet as follow-on contract work that improves delegation quality but does not block the current native imp delegation MVP (81.5 -> 81.8). v1 delegation may continue on the current unit-centric worker seam while the richer object-native packet is specified separately.
- Record the repo-grounded architecture decision from the current native-imp-tool review so future workers do not re-litigate whether codemap is required for the first implementation slice.
- Capture the user's clarified scoping decision from the conversation so 81.5 -> 81.8 proceeds without re-opening codemap as an MVP blocker.
- Capture the workflow rule for external-model escalations so future workers do not send repo-specific imp/mana assumptions into a model that lacks local context.
- Capture the durable external-escalation rule from the latest conversation so future workers use the same abstract prompt shape for hard architecture questions instead of inventing repo-specific prompts ad hoc.
- Capture the durable architectural conclusions mapped from the abstract GPT answer so future workers inherit the implementation shape directly from mana rather than reconstructing it from chat.
- Capture the durable implementation sequencing and scope-boundary conclusions from the latest review of the external-model answer so workers inherit the narrowed MVP plan directly from root mana.
- Capture the current durable execution decomposition for the native imp-tool MVP so future workers inherit the exact remaining work and ordering from mana rather than chat.
- Capture the durable architecture decision from the latest 81.5 review so future workers inherit the event-stream-preserving extraction shape directly from root mana.
- Record the user's workflow requirement and the current externalization checkpoint so future workers keep the native-imp-tool planning state in mana rather than only in chat.
- Capture the latest runner+tool decomposition as an explicit root-level architecture checkpoint before continuing the native imp-tool thread.
- Record the latest planning checkpoint so future workers can see from root 81 exactly which slices are now concretely specified and what remains to be sharpened next.
- Record the latest planning checkpoint from the conversation so future workers can see from root 81 what is already concretely specified and what remains to be pinned down next.
- Record the completion of the v1 planning layer so future workers know the native imp-tool thread has moved from decomposition into implementation-ready state.
- Record the explicit planning checkpoint requested by the user so future workers can see that the native imp-tool design phase has been externalized and closed enough to begin implementation.
- Record the practical starting point now that the planning and sequencing layers are externalized, so a future worker can begin at the right slice without re-deriving the first move.
- Record the explicit user-requested externalization checkpoint so the current plan and immediate next move are durably visible from the root architecture thread.
- Capture the repo-grounded post-implementation priority shift so future workers start with the actual remaining work rather than the old pre-implementation sequence.
- Record the completion of the native imp-tool MVP after the current implementation pass so future workers start from repo reality instead of the earlier planning state.
- Capture the durable post-review decision from the latest conversation so future workers see the commit-readiness blocker directly on the root native-imp-tool thread.
- Capture the durable post-review commit-readiness conclusion on the root native-imp-tool thread so future workers inherit the current repo-reviewed state rather than the earlier blocked verdict.
- |-
  Architecture clarification from current code inspection: mana already owns the active scheduling engine in Tower (`mana run`, `mana-pool`, `mana-core::ops::run`). The old Pi mana extension scheduler is useful historical reference but should not be treated as the canonical scheduler for new imp surfaces. Therefore future imp command/tool design should distinguish:
  - one-worker-on-one-unit execution (`imp mana <id>`-style surface)
  - mana-backed multi-unit orchestration/monitoring (`imp mana run/status/...`-style surfaces)
  without creating a second independent scheduler inside imp.
- 'Native imp-tool boundary decision from the latest discussion: the native `imp` tool should participate in the same split as the CLI. It may launch one child worker on one unit or one bounded transient helper, but it should not become an independent multi-unit scheduler. When a parent agent wants multiple subagents over workable jobs / an epic, the imp-side surface should hand that off to mana-backed orchestration/monitoring rather than owning a second scheduler.'
- 'Scheduler-of-record decision: mana remains the scheduler of record for ready-queue selection, dependency ordering, parallel dispatch, verify batching, and durable run state. Imp owns worker execution and operator UX surfaces, including native tool/CLI entrypoints, but orchestration initiated from inside imp should still route through mana-owned scheduling rather than reviving the old Pi extension scheduler or inventing a new imp-local one.'
- 'Native-tool/runtime principle from the latest discussion: for ordinary agent work, mana-backed behavior should be available as first-class native tools/runtime surfaces so it ''just works'' for the agent. Do not make agents shell out to `imp` CLI commands via bash for the common path. The `imp` CLI remains an operator-facing surface and compatibility entrypoint, while agent-to-mana / agent-to-worker interactions should prefer native tool calls and embedded runtime seams.'
- 'Native tool boundary tightened by the latest discussion: for agent-to-agent work, the parent should call native tools/runtime seams directly, not spawn `imp` CLI commands through bash for normal operation. The native `imp` tool remains the agent-facing child-worker surface; native `mana` remains the durable substrate/orchestration surface. Shelling out to `imp` should be reserved for operator workflows, compatibility paths, or debugging, not the common in-agent execution path.'
---

## Task
Design the first imp-native delegation/tool surface where one imp agent can invoke another imp agent.

## Current repo evidence
- `imp/crates/imp-cli/src/main.rs` already exposes `imp run <unit-id>` via `Commands::Run` and `run_headless_mode()`.
- `run_headless_mode()` loads a canonical `WorkerAssignment` through `imp_core::mana_worker::load_assignment_with_mana_dir`, assembles prefill/context, runs the session, and optionally verifies/closes inline.
- `mana/crates/mana-cli/src/commands/run/mod.rs` still treats `mana run` as a native orchestration entrypoint and checks for `imp` on PATH when using direct spawn mode.
- `mana/crates/mana-cli/src/spawner.rs` still shells out through `sh -c`, claims units, sets `IMP_MODE`, and captures logs.
- `imp/crates/imp-core/src/tools/mana.rs` already consumes mana run/status/state surfaces as a native tool.
- `imp/crates/imp-core/src/builder.rs` registers native tools, including `ManaTool`, and `imp/crates/imp-core/src/tools/mod.rs` defines the shared Tool/ToolRegistry surface.

## Design target
Specify a first-class `imp` tool/runtime seam whose core behavior is: one running imp can delegate bounded work to another imp worker/runtime instance.

## Required design questions
1. What is the canonical delegation target?
   - mana-addressed unit IDs only
   - ad hoc prompt delegation
   - or a two-layer model where ad hoc delegation is transient but durable work must route through mana
2. Which project owns what?
   - `imp` owns live child-worker invocation/runtime
   - `mana` owns durable orchestration state, work graph, and inherited outcomes
3. What should the tool return to the parent agent?
   - final text only
   - structured worker result/outcome
   - streamed status/events
   - pointers into mana/log/session artifacts
4. How should approval/policy/isolation work for nested calls?
5. How should this relate to the longer-term runner protocol and the `mana run` → `imp run` migration?

## Deliverables
- A repo-grounded design note under `docs/rebuild/` describing the first delegation model, boundaries, and migration sequence.
- A decomposition into follow-on units for implementation if the design stabilizes.
- Explicit statement of whether direct prompt-to-imp delegation is allowed in v1 or deferred.

## Constraints
- Keep the first slice narrow and compatible with the current canonical single-unit `imp run` path.
- Do not bypass mana for durable orchestration claims without an explicit decision.
- Prefer `imp` as the runtime owner and `mana` as durable truth, consistent with root docs.

## Candidate files to inspect/update
- `imp/crates/imp-cli/src/main.rs`
- `imp/crates/imp-core/src/mana_worker.rs`
- `imp/crates/imp-core/src/tools/mana.rs`
- `imp/crates/imp-core/src/tools/mod.rs`
- `imp/crates/imp-core/src/builder.rs`
- `mana/crates/mana-cli/src/commands/run/mod.rs`
- `mana/crates/mana-cli/src/spawner.rs`
- `docs/rebuild/imp-worker-boundary-plan.md`
- `docs/rebuild/imp-run-primary-execution-path.md`
- `docs/rebuild/tower-contracts-surface.md`

## Acceptance
- Design names the v1 delegation boundary clearly enough that a worker could implement a first slice without inventing ownership.
- Design explains how parent/child imp calls interact with mana durability, verification, and logs.
- Follow-on units are mechanical and worker-ready.

## Verify
Create `docs/rebuild/imp-delegation-tool-and-runtime.md` and ensure it mentions `imp run`, `mana`, `WorkerAssignment`, and `delegation` explicitly.
