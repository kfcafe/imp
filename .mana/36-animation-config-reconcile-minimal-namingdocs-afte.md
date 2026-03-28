---
id: '36'
title: 'animation config: reconcile Minimal naming/docs after removing full option'
slug: animation-config-reconcile-minimal-namingdocs-afte
status: open
priority: 3
created_at: '2026-03-27T08:07:33.902437Z'
updated_at: '2026-03-27T08:07:33.902437Z'
labels:
- animation
- cleanup
- config
verify: cd /Users/asher/tower/imp && ! grep -q '#\[serde(alias = "full")\]' crates/imp-core/src/config.rs && ! grep -q 'Restrained motion with concise state-specific labels' crates/imp-core/src/config.rs && cargo check -p imp-core && cargo check -p imp-tui
fail_first: true
kind: epic
---

## Problem

After removing the old `full` option, the animation config still has leftover semantics that are confusing:

- `AnimationLevel::Minimal` is the default and the richest remaining mode, but its docs say `Restrained motion with concise state-specific labels`.
- `imp-core/src/config.rs` still has `#[serde(alias = "full")]` on `AnimationLevel::Minimal`.
- Settings UI still surfaces the label as `"minimal"`, which may be fine, but the comments/docs should match actual behavior.

This is not a functional bug, but it leaves the animation settings model inconsistent and makes future cleanup harder.

## Fix

Tighten the naming/docs so the configuration matches reality after `full` was removed.

1. Audit `AnimationLevel` docs/comments in `crates/imp-core/src/config.rs`.
2. Remove the stale `#[serde(alias = "full")]` if backward-compatibility with old config values is no longer desired.
3. If the enum variant name stays `Minimal`, update the surrounding docs/comments/settings help text so it clearly describes the actual behavior.
4. If the better fix is to rename the enum variant, do the minimal safe refactor and update call sites/settings labels accordingly.

Prefer the smallest change that leaves the config model internally consistent.

## Key files
- `crates/imp-core/src/config.rs`
- `crates/imp-tui/src/views/settings.rs`
- any user-facing animation settings/help text
