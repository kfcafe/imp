---
id: '33'
title: 'chat view: replace duplicated animation logic with activity_label()'
slug: chat-view-replace-duplicated-animation-logic-with
status: open
priority: 2
created_at: '2026-03-27T08:03:57.381159Z'
updated_at: '2026-03-27T08:03:57.381159Z'
labels:
- animation
- cleanup
verify: cd /Users/asher/tower/imp && test $(grep -c 'spinner_frame' crates/imp-tui/src/views/chat.rs) -eq 0 && test $(grep -c 'waiting_badge' crates/imp-tui/src/views/chat.rs) -eq 0 && grep -q 'activity_label' crates/imp-tui/src/views/chat.rs && cargo check -p imp-tui 2>&1
fail_first: true
kind: epic
---

## Problem

`build_chat_lines` in `crates/imp-tui/src/views/chat.rs` (around line 530-560) has a manual `match activity_state { WaitingForResponse => match animation_level { ... }, Thinking => match animation_level { ... } }` block that duplicates logic from `animation::activity_label()`.

This duplication has already diverged: `activity_label` with `ActivitySurface::Chat` returns empty for `WaitingForResponse` in `Minimal` mode, but the inline code shows `"⠁ waiting"`. The centralized function should be the single source of truth.

## Fix

1. Replace the ~25-line inline match block (the one inside `if msg.is_streaming && msg.content.trim().is_empty()`) with a single call to `crate::animation::activity_label(activity_state, tick, animation_level, ActivitySurface::Chat)`.
2. **Also update `activity_label` in `animation.rs`**: the `Chat` surface currently returns empty for `WaitingForResponse` and `Thinking` in `Minimal` mode. It should show the waiting/thinking labels in Chat too — the chat inline indicator is useful when the message area is empty. Adjust the `Minimal` branch for `WaitingForResponse` and `Thinking` so Chat surface gets a label (matching what the inline code currently does).
3. After the fix, `chat.rs` should have zero direct calls to `spinner_frame` or `waiting_badge` — all animation text goes through `activity_label`.

## Key files
- `crates/imp-tui/src/views/chat.rs` — `build_chat_lines`, the streaming-empty-content block
- `crates/imp-tui/src/animation.rs` — `activity_label`, `ActivitySurface::Chat` branches
