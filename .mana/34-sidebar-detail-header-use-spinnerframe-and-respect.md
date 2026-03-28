---
id: '34'
title: 'sidebar detail header: use spinner_frame() and respect AnimationLevel'
slug: sidebar-detail-header-use-spinnerframe-and-respect
status: open
priority: 2
created_at: '2026-03-27T08:04:22.260666Z'
updated_at: '2026-03-27T08:04:22.260666Z'
labels:
- animation
- cleanup
verify: cd /Users/asher/tower/imp && ! grep -q 'const SPINNER' crates/imp-tui/src/views/sidebar.rs && grep -q 'spinner_frame' crates/imp-tui/src/views/sidebar.rs && grep -q 'AnimationLevel' crates/imp-tui/src/views/sidebar.rs && cargo check -p imp-tui 2>&1 | tail -1 | grep -q 'could not compile' && exit 1 || cargo check -p imp-tui 2>&1
fail_first: true
kind: epic
---

## Problem

`render_detail_header` in `crates/imp-tui/src/views/sidebar.rs` (around line 490) has two issues:

1. **Ignores `AnimationLevel`** — it shows a spinner even when `animation_level = None`. Every other spinner in the app respects this setting.
2. **Hardcoded spinner with wrong speed** — uses its own `const SPINNER` array with `tick/2`, while the standard `spinner_frame()` in `animation.rs` uses `tick/3`. This makes the detail header spin 50% faster than every other spinner in the UI.

## Fix

1. Add an `animation_level: AnimationLevel` parameter to `render_detail_header`.
2. Replace the hardcoded `SPINNER` array + `tick/2` lookup with a call to `crate::animation::spinner_frame(tick)`.
3. When `animation_level == AnimationLevel::None`, show a static `"•"` instead of a spinner (matching the pattern in `DisplayToolCall::header_line_animated_focused`).
4. Thread the `animation_level` from the `UiConfig` through the call site in `render_detail` → `render_detail_header`.

## Key files
- `crates/imp-tui/src/views/sidebar.rs` — `render_detail_header` and its caller `render_detail`
- `crates/imp-tui/src/animation.rs` — `spinner_frame()` for reference
