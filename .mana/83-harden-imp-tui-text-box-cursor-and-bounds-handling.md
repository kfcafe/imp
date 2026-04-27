---
id: '83'
title: Harden imp-tui text-box cursor and bounds handling against panic-prone state
slug: harden-imp-tui-text-box-cursor-and-bounds-handling
status: in_progress
priority: 2
created_at: '2026-04-13T05:54:35.138359Z'
updated_at: '2026-04-13T06:32:14.045790Z'
notes: |-
  ---
  2026-04-13T06:03:33.288860+00:00
  Implemented a focused imp-tui text-box hardening pass. In `imp/crates/imp-tui/src/views/editor.rs`, added `clamp_cursor_to_boundary(...)`, normalized editor cursors before any insert/delete/slice/render math, and made `cursor_screen_position(...)` return safely for zero-sized/tiny rects using saturating bounds. In `imp/crates/imp-tui/src/views/ask_bar.rs`, normalized stale option cursors and UTF-8 input cursors, removed direct option indexing in tab-to-edit/confirm paths, kept `editor_cursor` synchronized after typed/backspace edits, and hardened cursor position math for tiny rects. In `imp/crates/imp-tui/src/app.rs`, guarded ask-tab selection with `.get(...)` instead of direct indexing. Also fixed unrelated compile drift so narrow verification could run: `imp/crates/imp-lua/src/sandbox.rs` and `imp/crates/imp-lua/src/lib.rs` now initialize the newer `ToolContext.turn_mana_review` field, and `imp/crates/imp-tui/src/app.rs` now matches/constructs `AgentEvent::TurnEnd { mana_review, .. }` correctly. Added focused regression tests for stale/out-of-range cursors, invalid UTF-8 boundaries, and tiny render areas in both editor and ask-bar modules.

  ---
  2026-04-13T06:19:44.389938+00:00
  User requested a broader/full audit after the first editor+ask-bar hardening pass. Expanding this unit from the initial shared editor/ask overlay patch into a full imp-tui text-entry audit: inspect all TUI text-input surfaces and related handlers for panic-prone cursor/index/slice math, direct indexing, unwrap/expect in user-triggerable input paths, tiny-rect rendering issues, and invalid UTF-8 offset assumptions; patch concrete issues and verify with focused imp-tui checks.

  ---
  2026-04-13T06:32:14.045756+00:00
  Expanded the initial editor/ask hardening pass into a broader imp-tui text-entry audit and verified the full imp-tui lib suite cleanly. Additional findings/fixes beyond the first pass: (1) `views/settings.rs` now clamps stale `selected` field indices via `normalized_selected()` and guards empty `model_options`/`theme_options` before modulo/`len()-1` math in cycle_forward/cycle_backward; added tests `current_field_clamps_stale_selection` and `cycle_model_is_safe_with_empty_model_options`. (2) `views/personality.rs` now clamps stale builder-field selection via `normalized_selected()` and has a regression test for out-of-range selection. (3) `views/welcome.rs` had the riskiest direct indexing in live flow state: step/provider/model/web-provider selection now uses normalized helpers; `selected_provider*`/`selected_web_provider` return `Option`; empty provider lists fail gracefully instead of panicking; render paths show fallback messages; `finish_welcome` in `app.rs` now tolerates missing selected provider id with an anthropic fallback. Added welcome tests for stale indices and empty provider lists. (4) Existing earlier fixes remain: editor/ask cursor normalization, UTF-8 boundary clamping, stale option index guarding, and tiny-rect cursor positioning. Verification: targeted tests for new regressions passed, then full `cargo test -p imp-tui --lib -- --nocapture` passed with 184 tests, 0 failed.
labels:
- imp
- tui
- hardening
verify: cd /Users/asher/tower && cargo test -p imp-tui cursor_screen_position_tracks_soft_wraps -- --nocapture
verify_timeout: 120
kind: job
---

Goal: continue the imp-tui text-box hardening pass by making the shared editor and ask overlay robust against stale cursor positions, invalid UTF-8 byte offsets, empty/stale option cursors, and tiny render areas. Current state: `imp/crates/imp-tui/src/views/editor.rs` and `imp/crates/imp-tui/src/views/ask_bar.rs` assume cursor/index state is valid in several places (`String::insert`, slicing with `[..cursor]`, direct option indexing, and `area.width - 2` math). Those invariants usually hold in happy paths but can panic if state becomes stale or terminal geometry gets too small. Steps: 1) inspect editor and ask-bar cursor/index helpers and direct indexing call sites in `imp/crates/imp-tui/src/views/editor.rs`, `imp/crates/imp-tui/src/views/ask_bar.rs`, and `imp/crates/imp-tui/src/app.rs`; 2) add cursor normalization helpers that clamp to string length and valid char boundaries before any insert/delete/slice/render math; 3) make ask option selection paths use guarded access rather than direct indexing and normalize stale option cursors; 4) harden cursor-screen-position math to use saturating bounds for very small rectangles; 5) add focused regression tests covering invalid cursor offsets, stale option cursors, and tiny render areas; 6) run the smallest relevant imp-tui test/check command and record any unrelated compile blockers precisely if they remain. In scope: editor/ask overlay robustness for imp-tui text entry. Out of scope: unrelated TUI redesign or broad runtime refactors.
