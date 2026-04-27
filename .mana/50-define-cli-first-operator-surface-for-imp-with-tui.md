---
id: '50'
title: Define CLI-first operator surface for imp with TUI as an optional adapter
slug: define-cli-first-operator-surface-for-imp-with-tui
status: open
priority: 1
created_at: '2026-04-09T05:55:28.758827Z'
updated_at: '2026-04-09T18:04:29.592665Z'
notes: |-
  ---
  2026-04-09T05:56:48.993884+00:00
  Initial grounded decomposition after repo inspection:
  - Current code reality already leans CLI/runtime-first: `imp/crates/imp-cli/src/main.rs` owns `run_print_mode()`, `run_headless_mode()`, `run_rpc_mode()`, and `run_interactive()`.
  - `run_interactive()` is thin and mainly delegates to `imp_tui::interactive::InteractiveRunner`.
  - `imp/README.md` already exposes strong CLI-native entrypoints (`imp -p`, `imp run <unit-id>`, secrets/login/config, usage export).
  - `imp/IMP_REVIEW.md` and `imp/imp_rebuild_strategy.md` both support making runtime/protocol surfaces cleaner before deeper TUI cleanup.

  Working recommendation to evaluate and likely implement:
  - Product/frame imp as a CLI-first terminal agent/runtime.
  - Treat the TUI as an optional adapter/presentation layer over shared runtime/session/event surfaces.
  - Preserve machine/event-stream and mana-worker behavior as first-class, not secondary.

  Open product question to resolve explicitly in this work:
  - Should plain `imp` eventually mean CLI chat/REPL, or remain the fullscreen interactive mode while `imp tui` becomes an explicit alias and other CLI-native surfaces expand around it?

  ---
  2026-04-09T06:06:18.307776+00:00
  Conversation update: user wants to actively design a CLI-first interactive path rather than only discussing abstract positioning. New ideas to evaluate in the spec work:
  - strengthen `imp` as a CLI tool at minimum regardless of what happens to the TUI
  - consider simplifying the TUI significantly instead of keeping the current full-screen center-of-gravity
  - possible simplified TUI shape: just the prompt box plus a small widget/status surface rather than the current heavier interaction model
  - user still likes the sidebar/tool-output inspection concept, but is open to moving deeper inspection into a separate viewer app/surface
  - possible split surfaces mentioned by user: `imp` for stronger CLI interaction, and `imp2` / `imp-view` as a console/log/navigation viewer for agent runs or sessions
  - possible TUI role: browsing logs for each agent rather than being the default authoring shell

  Implication for the architecture work under this parent:
  - command grammar and product-surface design should explicitly compare (a) lightweight CLI chat + optional viewer, (b) simplified composer TUI, and (c) current fullscreen TUI as an optional power surface.
  - do not assume the current imp-tui should remain the primary operator surface.

  ---
  2026-04-09T06:08:20.423886+00:00
  Durable decomposition from latest CLI-first design discussion:

  Recommended product/runtime split:
  1. runtime/worker layer
     - one-shot prompt mode
     - `imp run <unit>` single-unit worker runtime
     - machine/event-stream / rpc modes
  2. interactive authoring shell
     - should become CLI-first
     - line-oriented terminal interaction with streaming output, compact tool notices, inline ask/confirm, session persistence, and lightweight status/widget row
  3. viewer/inspector layer
     - browsing tool logs, session branches, checkpoints, agent runs, and mana-related execution state
     - may become `imp view` rather than living inside the default authoring shell

  Working recommendation from this discussion:
  - strengthen `imp` as the primary CLI shell first
  - keep TUI available but demote it from default center-of-gravity
  - likely add explicit surfaces such as `imp tui` and `imp view`
  - avoid introducing a separate product/fork name like `imp2`

  Proposed migration phases captured from the discussion:
  - Phase 1: strengthen CLI without deleting current fullscreen path; add explicit CLI shell design, explicit `imp tui`, and an inspector/viewer concept
  - Phase 2: flip the mental model so `imp` is the CLI-first shell and fullscreen becomes optional
  - Phase 3: simplify the TUI into either a lighter composer+widget surface or a viewer-oriented surface focused on logs/inspection

  Open decision still unresolved:
  - whether plain `imp` eventually becomes the line-oriented interactive shell, with fullscreen moved to `imp tui` and/or `imp view`

  ---
  2026-04-09T06:10:32.291486+00:00
  Important product constraint from follow-up discussion: do not lose the existing value in TUI-native settings/personality/login surfaces while moving CLI-first. Reframe them as capability surfaces with multiple adapters rather than TUI-only features.

  Direction to preserve:
  - auth/login should remain first-class and are already partly CLI-native (`imp login`, `imp secrets`, `imp web-login`)
  - settings and personality should stay pleasant to use, but should not depend on fullscreen TUI ownership
  - the same underlying config/auth/personality state should be editable from:
    1. explicit CLI commands/subcommands
    2. inline commands inside the CLI interactive shell
    3. optional TUI/editor surfaces for richer browsing/editing

  Architectural implication:
  - move toward shared config/auth/personality actions with multiple UI adapters, rather than letting the current TUI panels be the only strong surface.

  ---
  2026-04-09T06:11:34.968329+00:00
  Durable plan update from CLI-first discussion: preserve the current TUI strengths around settings, auth/login, and personality, but stop treating them as TUI-owned capabilities. Product rule: no important setup/control capability should require fullscreen UI. These should become shared capability surfaces over the same underlying config/auth/personality state, accessible from (1) explicit CLI commands/subcommands, (2) inline commands inside the CLI interactive shell, and (3) optional richer TUI/editor flows.

  Working decomposition implied by this decision:
  - fast/common controls should work directly in the CLI shell (`:model`, `:thinking`, `:login`, `:secrets`, `:settings`, `:personality`)
  - richer editing/browsing can still live in optional TUI/editor flows
  - TUI remains valuable as a rich control room/editor and viewer, not as the only strong path to configuration/auth/personality management
  - the migration should preserve current pleasant UX while removing fullscreen dependence from core operator workflows

  ---
  2026-04-09T06:18:02.922765+00:00
  Externalized durable decomposition for the CLI-first direction before further design work:

  Product-surface split now under active consideration:
  1. `imp` as the primary CLI-first interactive authoring shell
  2. `imp run <unit>` as the canonical single-unit worker/runtime path
  3. `imp view` as the browsing/inspection surface for sessions, logs, branches, checkpoints, and agent activity
  4. `imp tui` as an optional richer adapter, likely simplified or viewer-oriented rather than the default center of gravity

  Durable migration principle:
  - strengthen CLI first without deleting current richer surfaces
  - preserve existing high-value TUI work (settings, personality, auth, sidebar/log inspection) but stop treating it as fullscreen-owned capability
  - no important operator/setup/control capability should require the TUI
  - expose the same capabilities through shell commands, direct subcommands, and optional richer viewer/TUI flows as appropriate

  Execution sequencing implied by this:
  - design CLI shell UX and command taxonomy first
  - map current TUI commands/overlays into CLI capability clusters
  - define `imp view` role separately from the authoring shell
  - define the simplified/optional TUI role after the CLI shell is clear

  ---
  2026-04-09T06:26:13.766711+00:00
  Durable planning update from the latest command-taxonomy sketch:

  Target top-level product surfaces:
  - `imp` → target default CLI-first interactive shell
  - `imp chat` → explicit CLI shell during migration/testing before flipping the default
  - `imp run <unit-id>` → canonical single-unit worker/runtime path
  - `imp view` → viewer/inspector for sessions, logs, branches, checkpoints, tool activity, and browsing-heavy inspection tasks
  - `imp tui` → optional richer fullscreen adapter/control-room surface
  - one-shot path remains important; keep `imp -p` and evaluate a clearer long-form command such as `imp prompt`

  Migration intent:
  - add explicit shell/view/tui surfaces first
  - do not force immediate default flip
  - preserve current strong TUI capabilities while moving toward CLI-first defaults

  ---
  2026-04-09T06:28:40.206691+00:00
  Durable planning update from grounded inspection of current TUI control surfaces:
  - Current TUI owns more than lightweight commands; it includes substantial guided editors/wizards and browsing surfaces.
  - Settings, personality, and welcome/setup should be treated as first-class guided flows in the CLI-first design, not reduced to config-file paths.
  - Useful distinction for the CLI migration:
    1. simple commands
    2. guided flows/editors
    3. browsing-heavy surfaces
  - These should map across shell commands, direct CLI subcommands, `imp view`, and optional `imp tui` rather than all collapsing into one shell interaction style.

  ---
  2026-04-09T06:29:19.216561+00:00
  Conversation-time externalization requested by user. Durable decomposition being carried forward now:

  CLI-first operator surface work is no longer just an abstract positioning exercise. The current planning shape under this parent is:
  1. top-level product surfaces and defaults (`imp`, `imp chat`, `imp run`, `imp view`, `imp tui`)
  2. command taxonomy and shell namespace
  3. transcript-level shell UX
  4. guided CLI flows that preserve parity for current settings/personality/setup capabilities
  5. viewer/inspector separation for browsing-heavy tasks
  6. simplified optional TUI role after CLI-first defaults are clear

  Latest durable distinction to preserve:
  - simple commands
  - guided flows/editors
  - browsing-heavy surfaces

  This distinction is now the main design lens for migrating current TUI-owned behavior into a CLI-first imp without losing quality.

  ---
  2026-04-09T06:58:25.639258+00:00
  Durable decomposition checkpoint from current CLI-first design work:

  Current root-level work split under this parent now has two immediate design forks to resolve next:
  1. default-entrypoint migration (`imp` vs `imp chat`) captured in 50.6.5
  2. browsing-heavy companion surface (`imp view`) captured in 50.7.1

  This sits on top of the already-externalized structure:
  - 50.1 CLI-first surface spec
  - 50.2 command grammar/default-entrypoint migration
  - 50.3 shared runtime startup map
  - 50.4 TUI-as-adapter boundary
  - 50.5 CLI-native affordance sequence
  - 50.6 CLI-first interactive shell path
  - 50.7 viewer/inspector surface
  - 50.8 simplified TUI role

  Planning lens to preserve:
  - shell = fast commands + guided flows
  - viewer = browsing-heavy inspection
  - optional TUI = richer adapter/control-room, not required for core operation
  - no important operator/setup/control capability should require fullscreen UI

  ---
  2026-04-09T07:02:16.004492+00:00
  Durable checkpoint after grounding the viewer direction in current code:
  - existing TUI code already contains a meaningful set of browse/inspect-first components (session picker, tree view, sidebar tool-log/detail panes) that map naturally to a distinct `imp view` surface.
  - this further supports the architectural split:
    - CLI shell for authoring/control/guided flows
    - `imp view` for browsing/filtering/scrolling/inspection
    - optional `imp tui` as richer adapter/control-room, not the default authoring requirement

  This checkpoint strengthens the case that the CLI-first move is not deleting strong current capabilities; it is reassigning some of them to a clearer viewer role.

  ---
  2026-04-09T07:03:02.970187+00:00
  User requested explicit conversation-time externalization of the latest durable plan before continuing. Root checkpoint updated now.

  Latest cross-surface decomposition to preserve:
  - CLI-first shell owns authoring, fast commands, and guided flows.
  - `imp view` owns browse/filter/scroll/inspect tasks that are awkward in a line-oriented shell.
  - optional `imp tui` remains a richer adapter/control-room surface, but is not required for core operation.

  Latest grounded viewer decomposition from current TUI code:
  - session picker maps naturally to a dedicated session-browsing viewer surface
  - tree view maps naturally to a branch/history viewer surface
  - sidebar stream/split panes map naturally to log/tool inspection surfaces
  - checkpoints likely belong with the same focused viewer family rather than inline shell-heavy UX

  Current working recommendation:
  - prefer explicit focused `imp view <submode>` entrypoints over one ambiguous fullscreen blob
  - preserve explicit shell-to-view handoff rather than silently morphing the shell into a browser

  ---
  2026-04-09T07:04:30.204135+00:00
  Recommended next-step sequence after current planning checkpoint:
  1. Resolve the default-entrypoint migration decision in 50.6.5 (`imp` vs `imp chat`) because it affects docs, command grammar, and implementation rollout.
  2. Resolve `imp view` submodes/handoff in 50.7.1 / 50.7.1.1 / 50.7.1.2 so browsing-heavy tasks have a clear home.
  3. Then implement the smallest proving slice without deleting the current TUI:
     - add explicit `imp chat` and `imp view` entrypoints
     - introduce CLI-shell command parsing (`:model`, `:thinking`, `:help`, `:quit` first)
     - keep current fullscreen path available as `imp tui`
  4. After that, build parity flows incrementally:
     - settings/personality/setup guided CLI flows
     - session resume/tree handoff to `imp view`
     - log/tool inspection handoff to `imp view`

  Reasoning: entrypoint and viewer boundaries are the main remaining product decisions; once they are explicit, the first implementation slice can be small, reversible, and low-risk.

  ---
  2026-04-09T07:05:43.705745+00:00
  Conversation-time externalization per user request. Durable next-step sequence from the latest planning checkpoint:

  Immediate product decisions to resolve before implementation:
  1. 50.6.5 — decide the default-entrypoint migration (`imp` vs `imp chat`), because this affects docs, command grammar, rollout risk, and user expectation.
  2. 50.7.1 (+ children) — finalize `imp view` as the home for browsing-heavy tasks, so the CLI shell does not bloat.

  Then execute a small, reversible proving slice instead of a large rewrite:
  3. add explicit `imp chat`, `imp view`, and `imp tui` entrypoints while preserving the current fullscreen path during migration
  4. introduce minimal CLI-shell command parsing and behavior for a first useful subset (`:help`, `:quit`, `:model`, `:thinking`)
  5. keep implementation intentionally narrow until the shell/view boundaries are validated in use

  Then build parity incrementally:
  6. guided CLI flows for settings/personality/setup
  7. session/tree browsing handoff to `imp view`
  8. log/tool inspection handoff to `imp view`

  Reasoning to preserve:
  - default-entrypoint and viewer-boundary decisions are the main remaining product-shape decisions
  - once those are explicit, the first implementation slice can stay small, reversible, and low-risk
  - do not delete the current TUI before the CLI shell and viewer prove themselves

  ---
  2026-04-09T07:07:57.600095+00:00
  User-directed change in sequencing on 2026-04-09: rather than waiting to finish all boundary specs first, begin by adding explicit `imp chat` and the implementation surfaces that come with it, then continue defining/refining boundaries from the concrete work. This does not remove the need for boundary clarity, but it changes the immediate execution order toward a proving implementation slice first.

  ---
  2026-04-09T07:22:14.451400+00:00
  Spec checkpoint from closed unit 50.1 (`docs/rebuild/imp-cli-first-surface.md`): recommended architecture is CLI-first runtime with TUI as adapter, grounded in current repo reality rather than aspiration. Current grounding to preserve in the graph: `imp/crates/imp-cli/src/main.rs` already owns sibling top-level paths `run_print_mode()`, `run_headless_mode()`, `run_rpc_mode()`, and `run_interactive()`. `run_interactive()` is downstream of CLI dispatch and currently hands off into `imp_tui::interactive::InteractiveRunner`, so the TUI should be treated as a presentation/client surface over shared runtime behavior rather than the product's architectural center. First migration slice from the spec: treat these four entrypoints as the canonical current operator map; extract shared startup/runtime assembly across them; keep `run_interactive()` as an adapter handoff during the first slice; do not start with a deep TUI redesign.

  ---
  2026-04-09T07:25:51.372973+00:00
  Decomposition captured from docs/rebuild/imp-command-grammar.md: (1) default-entrypoint migration lives in 50.6.5 and should flip plain `imp` to `imp chat` only after minimum shell viability; (2) browse-heavy session/log/tree/checkpoint work belongs under the `imp view` family in 50.7/50.7.1 rather than many new top-level commands; (3) fullscreen behavior belongs under explicit `imp tui` and simplified adapter work in 50.8; (4) proving-slice implementation in 50.9 should add explicit `chat`/`prompt`/`view`/`tui` entrypoints while preserving compatibility aliases and keeping `imp run <unit-id>` separate from chat behavior.

  ---
  2026-04-09T07:37:57.552193+00:00
  Conversation-time externalization requested by user before further work. Durable checkpoint after landing the first real `imp view` slice:
  - The CLI/view split is now implemented enough to be real, not just planned.
  - Current proven surfaces in `imp-cli`:
    - `imp chat`
    - `imp tui`
    - `imp view sessions`
    - `imp view tree`
    - `imp view logs`
    - `imp view checkpoints`
  - The next implementation move is no longer abstract boundary discussion; it is wiring explicit shell-to-view handoff from inside `imp chat` so browse-heavy tasks can be launched from the shell without turning the shell itself into a browser.
  - Product rule to preserve: keep `imp chat` transcript-oriented and single-pane; use `imp view` for browse/filter/inspect workflows.

  ---
  2026-04-09T07:46:41.834903+00:00
  Checkpoint after landing shell→view handoff in `imp-cli`:
  - The CLI-first split is now concrete enough to guide the next implementation layer.
  - Proven surfaces in `imp-cli` now include:
    - `imp chat` as the line-oriented authoring shell
    - `imp view sessions|tree|logs|checkpoints` as focused plain-text viewer modes
    - `imp tui` as the explicit fullscreen path
  - Inside `imp chat`, `:view <area>` / `/view <area>` now explicitly invokes the same viewer path and then returns to the shell prompt.
  - This makes the shell/view boundary real enough that the next major value move should shift from boundary-proving to CLI parity ergonomics.

  Recommended next implementation layer from this checkpoint:
  - guided CLI flows for `settings`, `personality`, and `setup`
  - prioritize `imp settings` first, then `imp personality`, then `imp setup`
  - keep these flows terminal-native and fullscreen-optional
  - do not regress to making important control/setup work depend on `imp tui`

  ---
  2026-04-09T07:49:50.311695+00:00
  Checkpoint from unit 50.2 (`docs/rebuild/imp-command-grammar.md`) after verify pass:
  - The command grammar/default-entrypoint decision is now explicit rather than deferred.
  - Recommended canonical families are:
    - `imp chat` for human-facing interactive authoring
    - `imp prompt` as the canonical one-shot prompt/print path replacing `imp -p` in docs while keeping `-p/--print` as compatibility aliases
    - `imp run <unit-id>` as the machine-facing / mana-worker runtime contract
    - `imp view` as the browsing/inspection family for sessions, logs, tree, and checkpoints
    - `imp tui` as the explicit fullscreen adapter
  - Default-entrypoint recommendation: after migration, plain `imp` should equal `imp chat`; do not keep fullscreen as the implicit default long-term and do not introduce a heuristic chooser beyond normal tty/stdin behavior.
  - Migration sequencing captured in the doc:
    1. introduce explicit `chat` / `prompt` / `view` / `tui`
    2. move docs/help/examples to explicit names while preserving compatibility
    3. flip plain `imp` to `imp chat` only after minimum shell viability
    4. keep `imp tui` as the explicit fullscreen path and retain low-cost aliases through a compatibility window
  - Exact behavioral rule preserved durably: with a tty and no subcommand, target behavior is `imp chat`; with piped stdin and no subcommand, target behavior is one-shot prompt/print equivalent to `imp prompt`.
  - Implication for follow-on implementation work: proving-slice CLI changes should preserve `imp run` separation from chat behavior, keep `imp view` as the browse-heavy companion surface, and treat `imp tui` as explicit rather than default.

  ---
  2026-04-09T07:52:24.301118+00:00
  Checkpoint update after starting `imp settings` work:
  - the first guided CLI parity layer has begun in concrete code (`imp settings`, `:settings`)
  - crate-scoped `imp-cli` verification is passing
  - full binary smoke verification is temporarily blocked by an unrelated workspace compile break in `mana-core`, which has now been externalized as a follow-up unit under the settings work so verification can resume cleanly once unblocked
  - this is a verification blocker, not evidence that the `imp settings` design direction is wrong

  ---
  2026-04-09T07:52:52.305224+00:00
  Conversation-time externalization requested explicitly by user before continuing. Durable checkpoint for the CLI-first implementation sequence:
  - `imp chat` is real and operationally useful
  - `imp view sessions|tree|logs|checkpoints` is real
  - shell→view handoff via `:view <area>` is real
  - first guided CLI parity work has started via `imp settings` / `:settings`
  - crate-scoped `imp-cli` verification is passing for the new settings work
  - full binary smoke verification is currently blocked by an unrelated `mana-core` compile mismatch, which has been externalized as a dedicated follow-up unit rather than left implicit in chat

  Execution consequence:
  - the immediate next implementation step is to clear the `mana-core` verification blocker, then resume end-to-end smoke verification of the new settings flow before broadening to personality/setup.

  ---
  2026-04-09T07:55:47.939035+00:00
  Conversation-time externalization requested by user before continuing. Durable checkpoint after finishing the concrete command taxonomy doc: the CLI-first operator-surface work now has a fixed cross-surface command map in `docs/rebuild/imp-cli-command-taxonomy.md`. New durable decisions visible at the root: (1) staged migration toward `imp == imp chat`; (2) canonical shell grammar is `:command` with temporary `/command` compatibility; (3) direct CLI families are `settings`, `personality`, `models`, `sessions`, `memory`, `checkpoints`, `login`, `secrets`, `setup`, and `ext`; (4) `imp view` owns browse/select-heavy flows; (5) `imp tui` remains optional and explicit; (6) built-ins and Lua extension commands share one shell namespace and discovery surface. Implementation and downstream spec work should now consume this taxonomy instead of reconstructing the command surface from scratch.

  ---
  2026-04-09T08:16:41.137421+00:00
  Conversation-time externalization requested by user before further execution. Durable checkpoint after the first `imp settings` slice:
  - `imp chat`, `imp view ...`, and `imp tui` are real enough to support the CLI-first proving path.
  - The first guided CLI parity flow (`imp settings` / `:settings`) is now implemented in `imp-cli`.
  - Standalone `imp settings` is smoke-verified enough to show that the terminal-native settings flow works and respects temp config roots when the environment is applied correctly.
  - The remaining immediate blocker is specifically shell startup for `imp chat --no-tools`: provider/auth/model resolution currently happens before the shell prompt, which prevents shell-local commands like `:settings` from being exercised without remote auth.

  Execution consequence from this checkpoint:
  - do not jump ahead to personality/setup yet
  - first clear the `imp chat --no-tools` startup/auth blocker so the shell can host local command surfaces without requiring provider auth up front
  - then finish end-to-end verification of `:settings` inside the shell
  - only after that widen the CLI parity layer to personality/setup

  ---
  2026-04-09T08:24:09.244774+00:00
  Viewer decomposition externalized from closed child planning under 50.7.1.1 so open work can inherit it cold. Product-surface recommendation: expose focused `imp view` subcommands, not one tabbed viewer shell. Canonical entrypoints: `imp view sessions`, `imp view tree`, `imp view logs`, `imp view checkpoints`. Bare `imp view` should remain a lightweight index/chooser rather than a resident tabbed app. Grounding from current browse-oriented TUI components: `session_picker.rs` is a list/detail fuzzy picker with preview and narrow select/open behavior -> sessions; `tree.rs` is a dedicated branch inspector with filter cycling and node preview -> tree; `sidebar.rs` is already a read-heavy stream/split inspector -> logs; checkpoints fit the same list/detail browse family even though their dedicated component is still to come. Comparison frame preserved: single viewer shell with internal modes vs focused subcommands. Recommendation remains focused subcommands at the operator-facing surface, with optional shared runtime/layout internals underneath for reuse.

  ---
  2026-04-09T08:34:11.671306+00:00
  Checkpoint after clearing the `imp chat --no-tools` startup/auth blocker:
  - `imp settings` is now usable enough in both entry forms:
    - standalone `imp settings`
    - shell-local `:settings` inside `imp chat`
  - the CLI-first shell can now host local command surfaces without requiring provider auth up front, which is an important product property for future setup/config/personality flows too
  - this moves the CLI parity work out of pure proving/unblocking mode and into the next real ergonomics layer

  Recommended next implementation target from this checkpoint:
  - `imp personality` / `:personality`, preserving builder/source mode and global vs project scope as already captured in the mana decomposition

  ---
  2026-04-09T08:43:36.457268+00:00
  Checkpoint after landing the first `imp personality` slice:
  - CLI-first parity now covers two substantial control surfaces in concrete code:
    - `imp settings` / `:settings`
    - `imp personality` / `:personality`
  - both are terminal-native, shell-accessible, and no longer depend on fullscreen TUI ownership
  - the shell can now host local control/setup-oriented commands without requiring provider auth up front in `--no-tools` mode

  Recommended next implementation target from this checkpoint:
  - `imp setup` / `:setup` to complete the first trio of rich CLI control/setup flows (settings, personality, setup)

  ---
  2026-04-09T08:44:26.564557+00:00
  Conversation-time externalization requested by user. Durable checkpoint after landing the first `imp personality` slice:
  - CLI-first parity now covers two substantial terminal-native control surfaces in real code:
    - `imp settings` / `:settings`
    - `imp personality` / `:personality`
  - Both are reachable as top-level CLI subcommands and as shell-local commands inside `imp chat`.
  - Both no longer depend on fullscreen TUI ownership.
  - The shell can now host local control/setup-oriented commands in `--no-tools` mode without requiring provider auth up front.

  Next recommended implementation target from this checkpoint:
  - `imp setup` / `:setup`
  - rationale: this completes the first trio of rich CLI control/setup flows (settings, personality, setup) and continues the CLI-first migration without needing to revisit the shell/view boundary first.

  ---
  2026-04-09T09:09:12.955202+00:00
  Checkpoint update after starting the first `imp setup` slice:
  - the setup wizard work is underway in `imp-cli`, but verification is blocked by a new unrelated workspace compile drift in `mana-cli`
  - as before, the blocker has been externalized immediately rather than left implicit in chat
  - do not treat the `imp setup` direction as failing; clear the `mana-cli` compile blocker first, then resume setup verification and smoke testing

  ---
  2026-04-09T09:10:09.721933+00:00
  Conversation-time externalization requested explicitly by user before continuing. Durable checkpoint after starting the first `imp setup` slice:
  - `imp setup` / `:setup` work is now underway in `imp-cli` as the next CLI-first parity surface after settings and personality.
  - Current intended wizard shape in code mirrors the earlier TUI welcome/setup structure in a terminal-native way:
    1. provider selection with configured vs needs-auth status
    2. provider auth path (OAuth login or manual API key depending on provider)
    3. model selection for the chosen provider
    4. thinking-level selection
    5. optional web-search provider selection and API-key capture
    6. config save + summary
  - Verification of this setup slice is currently blocked by unrelated compile drift in `mana-cli`, not by a known design problem in the `imp setup` direction.
  - Immediate execution consequence: clear the `mana-cli` blocker first, then resume `imp setup` verification and smoke testing before broadening further.

  ---
  2026-04-09T09:31:23.081273+00:00
  Checkpoint update after clearing the first `mana-cli` blocker during `imp setup` verification:
  - the `mana-cli` tuple-shape drift is effectively resolved enough for `cargo check -p mana-cli` to pass
  - verification is still not complete because a new unrelated upstream compile drift in `imp-core` surfaced immediately afterward (`VerifyGroupRun` match exhaustiveness)
  - as before, this blocker has been externalized immediately rather than conflated with the `imp setup` work itself
  - do not treat the `imp setup` direction as blocked on product design; this remains a verification-unblocker chain caused by workspace drift

  ---
  2026-04-09T15:05:00.700386+00:00
  Conversation-time externalization requested by user after closing 50.4. Durable adapter-boundary decomposition from `docs/rebuild/imp-tui-adapter-boundary.md`: `imp-tui` should own fullscreen presentation only — rendering/layout, panes/sidebar behavior, overlays, selection, keyboard/mouse navigation, focused current-session transcript/composer presentation, and focused tool-output inspection. Shared imp runtime/session surfaces should own runtime bootstrap, model/provider/auth/config resolution, agent run lifecycle, event feeds, session load/append/persist semantics, and shared config/auth/personality/setup state. Browse-heavy cross-session inspection (sessions, branches/tree, logs, checkpoints, agent activity) should bias toward `imp view`, while routine authoring/control should bias toward the CLI shell. Product rule to preserve: no important settings/personality/auth/setup capability should depend on fullscreen ownership; the TUI may remain the richest editor/inspector for those surfaces, but only as an adapter over shared state.

  ---
  2026-04-09T15:05:49.507320+00:00
  Adapter-boundary delta from completed 50.4, externalized for downstream work: treat `imp-cli` / shared runtime as upstream and `imp-tui` as downstream fullscreen presentation. Preserve the current handoff shape (`run_interactive()` -> `InteractiveRunner`) as an adapter seam, not evidence of TUI architectural ownership. TUI-local scope is now intentionally narrow: rendering/layout, panes/sidebar, overlays, selection, keyboard/mouse navigation, focused current-session transcript/composer presentation, status/widget presentation, and focused tool-output inspection. Shared runtime/session/config/auth/personality/setup surfaces stay upstream and should be consumed by the TUI rather than redefined there. Cross-surface consequence to preserve: browse-heavy cross-session inspection (sessions, tree/branches, logs, checkpoints, agent activity) should bias toward `imp view`; routine authoring and control should bias toward the CLI shell; no important settings/auth/personality/setup capability should depend on fullscreen ownership.

  ---
  2026-04-09T15:06:37.236801+00:00
  Externalized the durable adapter-boundary delta from completed unit 50.4 onto the stable root parent so follow-on work can inherit it even while direct lookup for some child IDs is inconsistent. Added three parent-level decisions: (1) preserve `imp-cli`/runtime upstream of `imp-tui`, with `run_interactive()` remaining an adapter handoff and `InteractiveRunner` staying thin; (2) narrow `imp-tui` ownership to presentation state only — rendering, transcript/composer presentation, panes/sidebar, selection, overlays, keyboard/mouse navigation, status/widget display, and focused current-session tool-output inspection; (3) bias browse-heavy cross-session responsibilities toward `imp view` rather than fullscreen ownership. Also created two explicit root follow-up jobs from the seam analysis: 50.13 for planning how to peel shared runtime/session/config/auth/personality ownership out of `imp-tui/src/app.rs`, and 50.14 for specifying the shared UI-request/runtime-event seam that TUI, CLI, and view surfaces should all consume.

  ---
  2026-04-09T15:10:46.980625+00:00
  Externalized the CLI-native affordance sequence from docs/rebuild/imp-cli-affordance-sequence.md into root mana as concrete follow-on units with explicit ordering/dependency shape. Created 50.17 stable human vs machine output contracts (depends on shared UI/runtime seam 50.14); 50.18 CLI-first session browsing/search and session_search surface (depends on existing viewer split 50.11); 50.20 checkpoint productization (depends on 50.18 browse/search surface); 50.22 visible planning workflow and plan artifact (depends on 50.14 and 50.17); 50.24 approval policy surface linked to durable mana gating (depends on 50.14 plus rebuild review/approval policy work 45.6 and fact 52); 50.26 detached/background local execution model (depends on 50.17, 50.18, 50.20, 50.22, 50.24). This captures the durable order as output -> browse/search -> checkpoints -> planning -> approval -> detached/background, with runtime-heavy items separated from mostly surface/productization items.

  ---
  2026-04-09T18:04:29.592659+00:00
  Conversation-time externalization per user request. Verified `docs/rebuild/imp-view-submodes-and-entrypoints.md` and recorded the durable viewer decomposition on the root CLI-first parent so downstream work can inherit it even if child lookup is inconsistent. Grounded mapping to preserve: `session_picker.rs` -> `imp view sessions`; `tree.rs` -> `imp view tree`; `sidebar.rs` stream/split inspector -> `imp view logs`; checkpoints stay in the same focused list/detail viewer family. Product-surface rule: keep `imp view` entrypoints explicit and task-shaped; share runtime/layout internals if helpful, but keep the shell/view boundary visible and avoid drifting back into a single fullscreen browser/app.
labels:
- architecture
- imp
- cli
- tui
- ux
verify: cd /Users/asher/imp && test -f docs/rebuild/imp-cli-first-surface.md && rg -q 'CLI-first' docs/rebuild/imp-cli-first-surface.md && rg -q 'TUI as adapter' docs/rebuild/imp-cli-first-surface.md && rg -q 'run_print_mode' docs/rebuild/imp-cli-first-surface.md && rg -q 'run_interactive' docs/rebuild/imp-cli-first-surface.md
kind: job
decisions:
- 'Decision: adopt a CLI-first imp command grammar centered on explicit surfaces. Canonical families are `imp chat` for human authoring, `imp prompt` for one-shot prompt/print, `imp run <unit-id>` for the machine-facing mana-worker runtime, `imp view` for browsing/inspection, and `imp tui` for explicit fullscreen presentation. Plain `imp` should eventually equal `imp chat` after a staged compatibility migration rather than remaining fullscreen by default or becoming a heuristic launcher.'
- 'Decision for CLI/viewer surface: keep `imp view` operator-facing entrypoints focused and explicit (`sessions`, `tree`, `logs`, `checkpoints`) instead of exposing one viewer shell with internal tabs as the primary product contract. Reasoning: strongest CLI legibility, direct shell handoff mapping, and best protection against drift back into a second monolithic fullscreen authoring interface. Internal runtime/layout reuse remains encouraged.'
- 'Decision captured from completed unit 50.8 (`docs/rebuild/imp-simplified-tui-role.md`): after the CLI-first shift, `imp tui` should be product-framed as an explicit composer/control-panel adapter over shared imp state, not as the primary viewer or default product identity. Stable cross-surface split: CLI shell owns default authoring and routine control; `imp view` owns browse-heavy inspection (sessions, tree/branches, logs, checkpoints, agent activity); `imp tui` preserves focused fullscreen composition plus sidebar/tool inspection and optional rich editing of shared settings/personality/auth/setup state.'
- 'Decision from the simplified-TUI role spec: preserve three distinct operator surfaces rather than one overloaded fullscreen app. `imp chat` owns default authoring and routine control; `imp view` owns browse-heavy inspection (sessions, tree/branches, logs, checkpoints, agent activity); `imp tui` owns optional fullscreen focused composition, sidebar/tool inspection, and rich control-panel editing over shared config/auth/personality/setup/session state. Settings/personality/auth/setup remain valuable in the TUI but are not TUI-owned capabilities; they must remain available through shared CLI-first flows and shared underlying state.'
- Durable decision externalized from closed unit 50.4 (`docs/rebuild/imp-tui-adapter-boundary.md`).
- 'Model `imp-tui` as a fullscreen presentation adapter over shared runtime/session/event/config/auth/personality/setup surfaces. Preserve the current `run_interactive()` -> `InteractiveRunner` handoff as an adapter seam, but do not treat it as evidence that the TUI owns product architecture. TUI-local ownership stays narrow: rendering/layout, panes/sidebar, overlays, selection, keyboard/mouse navigation, focused current-session transcript/composer presentation, status/widget presentation, and focused tool-output inspection. Canonical runtime bootstrap, agent construction, session semantics, and browse-heavy cross-session inspection remain upstream/shared, with routine authoring/control biased toward the CLI shell and browse-heavy inspection biased toward `imp view`.'
- 'Adapter-boundary decision from `docs/rebuild/imp-tui-adapter-boundary.md`: preserve the current boundary direction during migration. `imp-cli` remains the upstream command/runtime dispatcher; `run_interactive()` remains an adapter handoff into `imp-tui`; `InteractiveRunner` stays thin and presentation-entry oriented rather than growing into a runtime owner.'
- 'Adapter-boundary decision from `docs/rebuild/imp-tui-adapter-boundary.md`: treat rendering, transcript/composer presentation, panes/sidebar, selection, overlays, keyboard/mouse navigation, status/widget display, and focused current-session tool-output inspection as TUI-local concerns. Treat runtime/tool/session/model/config/auth/personality/setup semantics as shared surfaces that `imp-tui` consumes rather than defines in `app.rs`.'
- 'Adapter-boundary decision from `docs/rebuild/imp-tui-adapter-boundary.md`: sessions, tree/branches, logs, checkpoints, and agent activity should bias toward `imp view` rather than the default fullscreen authoring surface. `imp-tui` may share components or present focused current-session inspection, but browse-heavy cross-session ownership should not depend on fullscreen mode.'
- Externalizing CLI-affordance sequencing from completed unit 50.5 onto the stable root parent so follow-on work inherits the order directly from mana, not only from the doc.
- '`imp view` should stay a family of explicit focused entrypoints at the product surface — `imp view sessions`, `imp view tree`, `imp view logs`, and `imp view checkpoints` — with bare `imp view` defaulting to `sessions` for now. Shared viewer runtime/components are fine under the hood, but the operator-facing contract should not collapse back into one monolithic viewer shell or blur with the authoring shell.'
---

Goal: decide and document whether imp should become explicitly CLI-first, with the TUI treated as an optional presentation layer over the same runtime and command semantics.

Current repo facts already inspected:
- `imp/crates/imp-cli/src/main.rs` owns multiple non-TUI execution paths today: `run_print_mode()`, `run_headless_mode()`, `run_rpc_mode()`, and `run_interactive()`.
- `run_interactive()` is currently a thin adapter that creates `imp_tui::interactive::InteractiveRunner`; the main TUI entrypoint lives in `imp/crates/imp-tui/src/interactive.rs` and `app.rs`.
- `imp/README.md` already exposes CLI-first-feeling surfaces such as `imp -p`, `imp run <unit-id>`, `imp -c`, login/secrets/config commands, and usage export.
- `imp/IMP_REVIEW.md` recommends making planning visible in both TUI and CLI, adding approval policy, and tightening detached/headless flows.
- `imp/imp_rebuild_strategy.md` says the runtime/protocol refactor should come before deeper TUI cleanup, and that the TUI should consume cleaner runtime/state boundaries rather than drive them.

Questions to answer:
1. Should `imp` be product-framed as a CLI tool first, with TUI optional, rather than as a TUI app with CLI side paths?
2. Should plain `imp` remain the current full-screen interactive mode, or should `imp tui` become explicit while the default path becomes a simpler line-oriented CLI/REPL/chat mode?
3. What command grammar should become canonical (`imp ask`, `imp run`, `imp chat`, `imp sessions`, `imp checkpoints`, etc.)?
4. What runtime/event/model/session code should be extracted so CLI, headless, RPC, and TUI all share the same behavior instead of each owning custom startup/run loops?
5. What output contracts should be stable for human CLI use versus machine/event-stream use?

Deliverable:
- Write `docs/rebuild/imp-cli-first-surface.md` grounding the recommendation in current code and proposing a migration path.
- Recommendation should be opinionated, name tradeoffs, and sequence the first implementation slice.

Do not:
- redesign the visual TUI in this unit
- pretend the current repo is already CLI-first if the code still duplicates runtime logic
- blur mana/imp ownership while discussing operator surface
- skip grounding in the currently inspected files
