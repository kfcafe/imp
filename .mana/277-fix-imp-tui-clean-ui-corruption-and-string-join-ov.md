---
id: '277'
title: Fix imp TUI clean UI corruption and string join overflow panic
slug: fix-imp-tui-clean-ui-corruption-and-string-join-ov
status: in_progress
priority: 1
created_at: '2026-04-27T16:59:23.538366Z'
updated_at: '2026-04-27T17:14:59.289711Z'
acceptance: Imp TUI no longer panics with `String join would overflow memory bounds` from normal rendering/input state and the clean UI text layout avoids corrupted overlapping output for wrapped content.
notes: |-
  ---
  2026-04-27T17:01:14.089173+00:00
  Investigated screenshot regression. Recent sidebar inspector change renders full pretty-printed tool input from DisplayToolCall.details. For large tool args (e.g. screenshot/read content) this can produce enormous lines/line counts and ratatui render paths can panic with `String join would overflow memory bounds` plus corrupt clean UI wrapping. Implemented bounded inspector input preview in crates/imp-tui/src/views/sidebar.rs: max 80 input lines, max 12k chars, per-line display-width cap before wrapping, with truncation marker. Added targeted regression test for huge args.

  ---
  2026-04-27T17:11:12.941378+00:00
  Verification update: `cargo fmt -p imp-tui --check` initially failed only because sidebar.rs needed rustfmt wrapping; ran `cargo fmt -p imp-tui`. Focused verification now passes: `cargo test -p imp-tui inspector -- --nocapture` and `cargo check -p imp-tui`. Broader `cargo test -p imp-tui --lib` still has one unrelated existing failure in `tui_integration_slash_memory_add_and_show` asserting memory output contains `Added`; this is outside the inspector/input-rendering regression scope.

  ---
  2026-04-27T17:14:59.289709+00:00
  Adjusted fix per user direction: replaced the raw-JSON-with-cap approach with structured per-tool input summaries. Inspector now shows useful fields by tool (shell command/workdir/timeout, read path/offset/limit, edit path/edit count, write path/content size, scan/mana/ask/web/spawn key fields, generic field summaries) and only uses small scalar truncation as a backstop. This avoids feeding giant raw JSON into the clean UI while preserving useful context. Targeted inspector tests pass.
labels:
- bug
- tui
- ui-regression
verify: cargo test -p imp-tui --lib || cargo test -p imp --lib
kind: job
---

Investigate and fix the imp TUI regression shown in screenshot: clean UI text becomes horizontally corrupted/overlapped during long chat/tool output, then panics with `String join would overflow memory bounds` near the prompt/input area. Scope: inspect recent TUI rendering/input wrapping paths, identify unbounded string join or layout width issue, make the smallest safe fix, and add/adjust targeted tests if feasible. Verify with the narrowest relevant Rust test/check for TUI/core rendering plus `cargo check` for affected crate if no narrower command exists.
