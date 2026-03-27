---
id: '27'
title: Native mana tool overhaul — background runs, lightweight UI, non-blocking follow-ups
slug: native-mana-tool-overhaul-background-runs-lightwei
status: open
priority: 1
created_at: '2026-03-25T17:08:04.108096Z'
updated_at: '2026-03-25T17:08:04.108096Z'
acceptance: imp has a clear native path for mana orchestration work; mana activity can surface in a compact UI area; background mana work does not prevent later user messages from reaching the agent; prompt/tool guidance makes the native path the default over bash; work is split into focused child units.
labels:
- feature
- mana
- imp-core
- imp-tui
paths:
- crates/imp-core/src/tools/mana.rs
- crates/imp-core/src/agent.rs
- crates/imp-core/src/ui.rs
- crates/imp-cli/src/main.rs
- crates/imp-tui/src/app.rs
feature: true
---

Improve imp's native mana tool so agents use it instead of shelling out through bash for mana work, and so it behaves more like Pi's mana tool. Split this into orchestration actions/run state, compact UI, non-blocking follow-ups, and guidance to prefer the native mana tool over bash when equivalent actions exist.
