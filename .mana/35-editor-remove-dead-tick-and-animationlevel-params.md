---
id: '35'
title: 'editor: remove dead tick and animation_level params'
slug: editor-remove-dead-tick-and-animationlevel-params
status: open
priority: 2
created_at: '2026-03-27T08:03:57.381192Z'
updated_at: '2026-03-27T08:03:57.381192Z'
labels:
- animation
- cleanup
verify: 'cd /Users/asher/tower/imp && ! grep -q ''animation_level: AnimationLevel'' crates/imp-tui/src/views/editor.rs && ! grep -q ''tick: u64'' crates/imp-tui/src/views/editor.rs && cargo check -p imp-tui 2>&1'
fail_first: true
kind: epic
---

## Problem

`EditorView` in `crates/imp-tui/src/views/editor.rs` has `tick: u64` and `animation_level: AnimationLevel` fields with builder methods (`.tick()`, `.animation_level()`), but the `render()` method never reads them. They're dead code plumbed through for nothing.

## Fix

1. Remove the `tick` and `animation_level` fields from the `EditorView` struct.
2. Remove the `.tick()` and `.animation_level()` builder methods.
3. Remove the `use imp_core::config::AnimationLevel;` import from editor.rs if it becomes unused.
4. Update the call site in `app.rs` `render()` method — remove the `.tick(self.tick)` and `.animation_level(self.config.ui.animations)` calls on the `EditorView` builder.

## Key files
- `crates/imp-tui/src/views/editor.rs` — struct, builder methods
- `crates/imp-tui/src/app.rs` — the `render()` method where `EditorView` is constructed
