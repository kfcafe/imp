---
id: '248'
title: Comprehensive imp UI/UX review, upgrade, and polish across CLI shell and TUI surfaces
slug: comprehensive-imp-uiux-review-upgrade-and-polish-a
status: open
priority: 1
created_at: '2026-04-13T07:39:59.252552Z'
updated_at: '2026-04-14T01:54:38.353926Z'
notes: |-
  ---
  2026-04-13T07:48:52.112151+00:00
  Inspected current imp surfaces and docs before decomposition. Grounding inspected this session: imp/README.md; ../VISION.md; imp/crates/imp-cli/src/main.rs; imp/crates/imp-tui/src/app.rs; imp/crates/imp-tui/src/keybindings.rs; imp/crates/imp-tui/src/theme.rs; imp/crates/imp-tui/src/views/{status.rs,top_bar.rs,editor.rs}; scan summaries for imp-cli, imp-core, and imp-tui; prior review memos IMP_REVIEW.md and IMP_DEEP_REVIEW.md; rebuild docs ../docs/rebuild/{imp-command-grammar.md,imp-cli-command-taxonomy.md,imp-simplified-tui-role.md}.

  Initial grounded findings:
  - Product direction and implementation are misaligned: docs lean CLI-first while the fullscreen TUI still carries much of the richest interaction and settings/auth/session UX.
  - The TUI visual language is competent but not premium yet: only default/light themes, dense status chrome, placeholder copy still references slash commands, and hierarchy/animation/spacing feel functional rather than intentional.
  - Editor/composer quality is decent and recently hardened, but surrounding affordances (titles, status, queue/progress legibility, command discoverability, tool focus) feel uneven.
  - App/controller architecture is too concentrated in imp-tui/src/app.rs and imp-cli/src/main.rs, which will slow cohesive polish unless shared presentation/runtime seams are extracted.
  - Prior review findings still matter for UX trust: checkpoint productization, visible planning, explicit approval policy, cancellation propagation, shell backend consistency, and large-module splits all affect operator confidence.

  User asked for a comprehensive UI/UX review with a premium driving feel ('Lotus Elise' precision + 'Rolls Royce' composure). I asked which surface should be the flagship; no answer was provided. Proceeding assumption for planning unless corrected: dual-track polish around shared interaction primitives, while honoring current CLI-first direction in docs and treating the TUI as the premium cockpit/inspector.

  ---
  2026-04-13T07:49:25.976964+00:00
  Execution decomposition externalized in mana.

  Planned sequence:
  1. 248.1 — grounded UX audit of current CLI shell, TUI, and setup/inspection flows.
  2. 248.2 — shared premium interaction-system spec translating the desired feel into concrete shell/TUI/view rules.
  3. 248.3 — first implementation proving slice after audit/spec, focused on status/composer/discovery polish with targeted verification.

  Current open product choice:
  - Flagship premium surface is still unresolved in chat. Until the user decides otherwise, proceed with shared primitives and maintain alignment with current CLI-first docs while treating the TUI as the premium cockpit/inspector.

  Definition of done for the epic:
  - durable audit doc exists
  - durable interaction-system spec exists
  - at least one verified polish slice lands in code
  - follow-on work is decomposed for deeper UX upgrades

  ---
  2026-04-13T07:58:07.057985+00:00
  Conversation nuance captured: user is explicitly curious whether a powerful interactive CLI mode may ultimately be better than the TUI for primary use. Current answer from inspected product direction: yes for the default/daily-driver path, provided the shell gains stronger status, steering, planning/checkpoint/review affordances, and clean handoff into view/cockpit surfaces. This should shape 248.2 and later decomposition: optimize the shell for the majority path; keep TUI focused on composed fullscreen monitoring, inspection, and rich control-room editing rather than trying to own every workflow.

  ---
  2026-04-13T08:00:07.966943+00:00
  Durable decomposition update after audit/spec/wireframes landed:

  Completed foundation artifacts:
  - 248.1 closed — grounded UX audit in docs/review/imp-ui-ux-audit.md
  - 248.2 closed — premium interaction system in docs/design/imp-premium-interaction-system.md
  - 248.4 closed — TUI cockpit wireframes in docs/design/imp-tui-cockpit-wireframes.md

  Current durable product stance:
  - flagship primary experience: powerful interactive CLI shell
  - fullscreen TUI: premium cockpit for focused composition, run monitoring, tool inspection, blocked/review attention, and rich settings/personality/auth/setup editing
  - viewer: browse/select-heavy inspection surface

  Refined implementation sequence now externalized:
  1. Premiumize the interactive CLI shell first.
     Focus: composed status header, stronger turn summaries, clearer handoff language, better command discovery, and shell-native steering/trust affordances.
  2. Add shared trust surfaces across shell + TUI.
     Focus: visible planning artifacts, checkpoints, review/approval/blocked state vocabulary, and shared status semantics.
  3. Align TUI implementation to the cockpit wireframes.
     Focus: simplify hierarchy, make sidebar/tool inspection feel first-class, and reserve fullscreen for composed cockpit/editor states rather than owning every workflow.
  4. Extract shared UX/runtime seams from oversized surface-specific controllers so polish lands once and propagates.
     Focus: shared status renderers/copy, shared handoff phrasing, shared plan/checkpoint/review terminology, and reduced divergence between imp-cli and imp-tui.

  Key design insight to preserve:
  - the shell likely beats the TUI as the default daily-driver if it becomes a genuinely powerful interactive CLI mode rather than a minimal REPL. The shell should be optimized for speed, composability, steering, and trust; the TUI should optimize for cockpit-level focus and inspection rather than replacing the shell.

  Next recommended proving slice after 248.3:
  - define a concrete premium interactive CLI shell spec, then implement its first visible polish slice before broader TUI cleanup.

  ---
  2026-04-13T08:29:44.295712+00:00
  Durable execution update after the first shell implementation slice landed.

  Completed so far:
  - 248.1 closed — grounded UX audit
  - 248.2 closed — shared premium interaction system
  - 248.4 closed — premium interactive CLI shell spec
  - 248.5 closed — first shell polish slice implemented and verified

  Current durable implementation order:
  1. 248.6 — trust-surface sequencing is the next highest-leverage planning step.
     Why: the next major gain in premium feel comes from operator confidence rather than more copy polish. The shell and TUI both need a coherent plan for checkpoints, visible planning artifacts, review/approval states, and blocked-state language.
  2. 248.7 — shared UX/runtime seam planning should follow close behind.
     Why: shell/TUI/view consistency will degrade if status/copy/trust terminology continues to live separately in imp-cli and imp-tui.
  3. 248.3 — the older broad proving-slice unit should now be treated as partially superseded by 248.5. Keep it open only if we want a follow-on shared status/composer/discovery polish pass that is explicitly more TUI- and cross-surface-focused than the completed shell slice.

  Current recommendation to future workers:
  - do not jump straight into more visual TUI polish before 248.6 establishes the shared trust-surface sequence
  - use the shell as the primary proving ground for trust-surface language, then reflect the same semantics in the TUI cockpit
  - treat checkpoints/plans/review/blocked states as product surfaces, not secondary implementation details

  Residual implementation facts from 248.5 worth inheriting:
  - shell help/handoff language and TUI placeholder discovery copy were updated successfully
  - verification required fixing an existing imp-cli compile drift after AgentEvent::TurnEnd gained mana_review
  - a non-blocking imp-cli warning about unused variable `config_path` still exists and was intentionally left alone

  ---
  2026-04-13T18:59:09.924139+00:00
  Durable trust-surface decomposition update:
  - 248.6.1 closed — checkpoint surface contract completed
  - next trust-surface unit is 248.6.2 visible planning/checklist flow
  - then 248.6.3 review/approval/blocked-state contract

  This sequence is now encoded in the root mana graph so future workers inherit the intended order rather than reconstructing it from chat.

  ---
  2026-04-13T19:04:57.207106+00:00
  New user ask: start building the interactive CLI tool in earnest and consider routing the default entrypoint to `imp chat` so they can begin using it and giving feedback. Before changing behavior, inspect current imp-cli entrypoint dispatch and choose a safe migration path (likely explicit default flip or gated compatibility switch) grounded in code rather than docs alone.

  ---
  2026-04-13T19:05:35.139773+00:00
  User explicitly wants to start using the interactive CLI tool and provide feedback. New implementation direction for this thread: route the default `imp` entrypoint to the chat shell now, with `imp tui` remaining the explicit fullscreen cockpit and piped stdin / print mode behavior preserved. Treat this as an intentional early default flip for feedback collection, not merely a docs-only posture.

  ---
  2026-04-13T19:07:56.603495+00:00
  Durable plan update after flipping bare `imp` to the interactive CLI shell.

  Implemented direction now reflected in code and docs:
  - bare `imp` is the flagship CLI shell by default
  - `imp chat` remains the explicit shell spelling
  - `imp tui` remains the explicit fullscreen cockpit
  - `imp view` remains the inspector surface

  Immediate next-step decomposition for real-world feedback collection:
  1. 248.9 — capture and sequence real-user feedback on the new default shell so observations do not live only in chat transcripts
  2. shell quality follow-on: status/header + turn-summary refinement under a dedicated implementation job
  3. shell-native onboarding/discovery follow-on under a dedicated job
  4. continue trust-surface sequencing in 248.6 so feedback lands against a shell that is becoming more trustworthy, not just more polished

  This decomposition is intentionally root-scoped because it affects imp's cross-surface product posture, docs, and follow-on UX sequencing rather than only one isolated local file change.

  ---
  2026-04-13T20:06:59.644569+00:00
  Immediate post-248.10 execution queue externalized from the latest conversation.

  Recommended next order:
  1. 248.6.2 — define the first visible planning artifact and checklist flow for imp.
     Rationale: planning/checklists are the next biggest step up in premium feel and operator trust for the flagship shell. This should come before more ornamental shell polish.
  2. 248.11 — refine shell-native onboarding and command discovery after making bare `imp` default to chat.
     Rationale: once the default shell path is stable, onboarding/discovery should improve without interrupting the planning-trust work.
  3. 248.12 — follow-on shell polish for `:status` detail formatting, viewer return messaging, and clearer blocked/review summaries after planning semantics are explicit.
     Rationale: these are worthwhile, but should build on the visible planning/checklist vocabulary rather than racing ahead of it.

  Guidance to future workers:
  - keep the shell as the proving ground for trust surfaces
  - do not let additional shell chrome work outrun 248.6.2
  - treat viewer return language and blocked/review summaries as dependent on the explicit planning and trust contract, not just copy cleanup

  ---
  2026-04-13T20:48:39.822647+00:00
  Latest UX direction from the user: after status/summary polish, the next interest is a working animation plus better shell formatting. Durable guidance:
  - yes to subtle shell liveness animation next, but keep it transcript-safe and terminal-aware
  - yes to better formatting, but be honest that actual font family/size cannot be controlled by the app; improvements should focus on terminal-native typography (spacing, hierarchy, Unicode glyphs, ANSI emphasis, semantic color/styling)

  New units created to preserve this distinction:
  - shell liveness animation implementation
  - shell typography/formatting contract work

  ---
  2026-04-14T01:54:01.236544+00:00
  Implemented the first shell typography/formatting polish slice after 248.14, even though the new implementation unit reused a previously closed ID in mana metadata. Grounded code changes landed in imp/crates/imp-cli/src/main.rs and were verified.

  What improved in this slice:
  - startup shell intro now uses a cleaner two-line hierarchy (`imp chat` header + indented guidance)
  - `:status` detail output now uses a titled block with cleaner alignment and more human `session id` labeling
  - `:help` now uses a simpler section heading plus a separate compatibility section instead of one long dense paragraph block
  - compaction unavailable messaging is split into cleaner short lines
  - error follow-up wording now uses `next:` instead of `hint:` to align with the shell formatting contract's preferred guidance language

  What this slice intentionally did not do yet:
  - no ANSI color/bold/dim styling was introduced yet
  - formatting improvements are currently structural (spacing, line rhythm, hierarchy, labels) rather than terminal-style escapes

  Open follow-on question now externalized:
  - do we want a second formatting slice that introduces restrained ANSI emphasis in the shell (prompt/status/help labels, summary/tool/error labels) using crossterm or similar, while preserving graceful plain-text fallback?

  ---
  2026-04-14T01:54:38.353918+00:00
  Durable decomposition update requested explicitly by the user after the first shell typography/formatting polish slice.

  What is now externalized:
  - the first shell formatting slice already landed as a structural/hierarchy pass only (startup copy, `:status`, `:help`, compaction messaging, and `error` -> `next` guidance wording) and was verified in imp-cli
  - the next recommended formatting step is a second shell typography slice focused on restrained ANSI emphasis rather than more ad hoc copy changes

  Next formatting sequence now captured in mana:
  1. structural formatting pass — complete
  2. restrained ANSI emphasis pass — new explicit job under the root epic
     Focus: prompt/status/help labels plus `tool:`, `summary:`, `error:`, and `next:` hierarchy, with graceful plain-text fallback

  Guardrails preserved from the shell formatting contract:
  - no flashy color or heavy decoration
  - no reliance on color alone for meaning
  - no regression in transcript readability or portability
labels:
- imp
- ux
- ui
- cli
- tui
- review
- design
kind: epic
paths:
- imp/crates/imp-cli/src/main.rs
- imp/crates/imp-tui/src/app.rs
- imp/crates/imp-tui/src/views
- imp/crates/imp-core/src/config.rs
- imp/README.md
- docs/rebuild/imp-command-grammar.md
- docs/rebuild/imp-cli-command-taxonomy.md
- docs/rebuild/imp-simplified-tui-role.md
feature: true
decisions:
- 'Open product decision: flagship premium surface remains unresolved. Default execution assumption for this thread is dual-track/shared-primitives, aligned with CLI-first docs while preserving TUI as the premium cockpit until the user chooses otherwise.'
- 'Resolved with user input: imp should be CLI-first, with the shell as the flagship daily-driver and the fullscreen TUI treated as the premium cockpit/inspector. UX work should optimize the shell path first while designing the TUI as the composed high-trust control room.'
---

Goal: make imp feel markedly more refined, legible, and trustworthy across its human-facing surfaces, with a premium interaction character ('Lotus Elise' precision and responsiveness, 'Rolls Royce' calm confidence and finish).

Current state:
- imp already has substantial UX surface area across a CLI shell, fullscreen TUI, setup/settings/personality flows, session browsing, tree browsing, tool sidebars, and auth flows.
- The product direction in current docs is CLI-first with TUI as an explicit adapter, but the implementation still carries significant fullscreen-first history and duplicated/uneven affordances.
- Prior reviews already identify underexposed power, missing checkpoint/planning/approval productization, large UI control modules, and several trust/consistency gaps.

Desired outcome:
- Produce a grounded UX review, a prioritized polish/spec plan, and then implement a first proving slice of visible quality improvements.
- Align interaction quality, visual system, terminology, command grammar, and runtime feedback across chat shell, TUI, and inspect/browse flows.

Investigation areas:
1. Interaction model clarity: what plain `imp`, `imp chat`, `imp tui`, and `imp view` should feel like in practice.
2. Visual language: theme, spacing, hierarchy, animation, status density, tool rendering, empty states, and tone.
3. Operator confidence: approval cues, checkpoints, plans, progress, background/blocked/error states, and 'what is happening now' legibility.
4. Input/composer quality: editor behavior, command affordances, palette/file attach ergonomics, session naming, history, and steer/follow-up flows.
5. Inspection quality: tool sidebar/detail, session tree, session picker, settings/personality/setup, and mana review surfaces.
6. Architecture pressure: oversized app/controller modules, duplicated auth/runtime resolution, and shared renderer seams needed for coherent UX across CLI and TUI.

Expected deliverables:
- A concise UX audit with concrete findings tied to inspected code and docs.
- A phased implementation plan with proving slices and verify gates.
- Follow-on child jobs for high-leverage improvements.

Open decision to resolve:
- Whether the flagship premium experience should optimize first for the CLI-first shell path with TUI as the premium cockpit, or whether the fullscreen TUI should be re-elevated as the primary identity.

Verify: test -n "imp-ui-ux-review-captured"
