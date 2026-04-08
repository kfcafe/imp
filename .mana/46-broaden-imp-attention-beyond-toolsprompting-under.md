---
id: '46'
title: 'Broaden imp attention beyond tools/prompting: under-attended subsystem maturity pass'
slug: broaden-imp-attention-beyond-toolsprompting-under
status: open
priority: 1
created_at: '2026-04-07T17:45:30.525167Z'
updated_at: '2026-04-07T17:47:04.189852Z'
acceptance: |-
  1. Under-attended imp subsystems outside the heavily iterated native tools/system-prompt work are grouped into concrete follow-through tracks.
  2. Each track distinguishes existing open mana work from missing or lightly implemented areas discovered in the current codebase.
  3. Each track has an execution-ready next-step unit or a clear reconciliation plan so another agent can pick it up cold.
  4. Priorities reflect current source evidence, not just historical docs or intuition.
notes: |-
  ---
  2026-04-07T17:47:04.189843+00:00
  Source-backed attention map from current session:

  Highest-attention areas outside the native tool + system-prompt core:
  1. Session/runtime safety and long-session behavior
     - Evidence: `crates/imp-core/src/agent.rs`, `compaction.rs`, `tools/bash.rs`, `tools/mod.rs`, `session.rs`, `imp_session.rs`
     - Existing work: `.4.2`, `.5`, `.6.7`, `32`
     - Gaps/concerns: stale or drifted `.4.2` notes, auto-compaction still open, cancellation propagation still in progress, shell backend setting likely non-authoritative, file cache/history only partially wired per prior deep review.

  2. Operator UX / discoverability / surface cohesion
     - Evidence: `crates/imp-tui/src/app.rs`, `views/{welcome,command_palette,settings,sidebar,status,chat}.rs`
     - Existing work: `27`, `29`, `30`, `32.3`, `37.5`
     - Gaps/concerns: discoverability epic still open, mana status surfaces still partial, settings surface is large, capabilities like memory/session search/planning/checkpoints remain easy to miss.

  3. Policy and capability boundaries
     - Evidence: `crates/imp-lua/src/{bridge,sandbox,lib}.rs`, `crates/imp-core/src/{builder,config,guardrails,hooks}.rs`
     - Existing work: `31.2`-`31.4`, `.6.6`, `44`
     - Gaps/concerns: Lua/runtime capability policy still deserves direct follow-through, guardrails design landed but implementation follow-through is still open, extension architecture is still in design.

  4. Auth/provider/runtime cohesion
     - Evidence: `crates/imp-llm/src/auth.rs`, provider modules under `crates/imp-llm/src/providers/`, `crates/imp-cli/src/main.rs`, `crates/imp-tui/src/app.rs`, `crates/imp-core/src/imp_session.rs`
     - Existing work: `.3`, `37`, `41`, `44`
     - Gaps/concerns: TUI/CLI/core provider-auth logic appears duplicated, auth persistence hardening still worth attention, usage/reporting docs/polish still unfinished, provider-specific implementations are uneven in polish.

  5. Large-module decomposition / maintainability / change velocity
     - Evidence from current scans: `imp-tui/src/app.rs` ~5145 lines, `imp-core/src/agent.rs` ~3578, `imp-cli/src/main.rs` ~2915, `imp-llm/src/providers/anthropic.rs` ~2451, `imp-core/src/session.rs` ~2026, `imp-core/src/tools/mana.rs` ~1804.
     - Existing work: `.6.5` closed analysis-only, but there is no clear active decomposition roadmap.
     - Gaps/concerns: several breadth features likely stayed light because these modules absorb too many responsibilities.

  Recommendation for this epic: drive a child-job pass that (a) reconciles existing open units, (b) captures missing under-attended tracks, and (c) leaves worker-ready next steps rather than just another abstract review.
labels:
- imp
- analysis
- roadmap
- maturity
- coverage
verify: test -n "imp-breadth-maturity-epic-tracked"
kind: epic
---

Goal: identify the imp subsystems that have not received the same implementation depth, polish, and follow-through as the native tool work and system prompt builder, then turn that analysis into mana-tracked follow-up.

Current state:
- Native tools and system prompting have seen concentrated iteration, but breadth across the rest of imp is uneven.
- Current scans show multiple large, mixed-responsibility modules that likely slowed polish elsewhere: `crates/imp-tui/src/app.rs` (~5145 lines), `crates/imp-core/src/agent.rs` (~3578), `crates/imp-cli/src/main.rs` (~2915), `crates/imp-llm/src/providers/anthropic.rs` (~2451), `crates/imp-core/src/session.rs` (~2026), `crates/imp-core/src/tools/mana.rs` (~1804).
- Existing open work already clusters around under-attended areas: mana/TUI surfaces (`27`, `29`), guardrails/policy (`31.2`-`31.4`), usage/reporting follow-through (`37.5`), compaction and long-session behavior (`.4.2`, `.5`), auth/secrets (`.3`), extension architecture (`44`), and exploratory planning/runtime architecture (`45`).
- `IMP_REVIEW.md` and `IMP_DEEP_REVIEW.md` point to specific maturity gaps: underexposed checkpoints/planning/memory, Lua policy bypasses, cancellation not propagating into active tools, shell backend settings not being authoritative, brittle auth persistence, partially wired file cache/history, duplicated provider auth resolution, and large-file maintenance risk.

In scope:
- Build a cross-subsystem attention map for imp.
- Reconcile existing open mana work with current source evidence.
- Create or refine worker-ready follow-up units for the highest-leverage under-attended areas.

Out of scope:
- Doing the broad implementations directly in this umbrella epic.
- Re-litigating already-solid native tool or system-prompt work unless it blocks a breadth area.

Initial tracks to drive with child jobs:
1. Session/runtime safety and long-running session behavior.
2. Operator UX and discoverability of existing capabilities.
3. Policy/capability boundaries across Lua, hooks, guardrails, and shell execution.
4. Auth/provider/runtime cohesion across CLI, TUI, core, and imp-llm.
5. Large-module decomposition and shared-logic extraction for change velocity.

Do not:
- Duplicate existing open units without first reconciling whether they already cover the need.
- Create vague 'investigate X' follow-ups with no file focus, current-state summary, or verify gate.
- Let speculative architecture ideas crowd out user-visible or safety-critical maturity work.
