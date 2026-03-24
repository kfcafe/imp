---
id: '24'
title: TUI UX overhaul — information density, summaries, interactivity, polish
slug: tui-ux-overhaul-information-density-summaries-inte
status: open
priority: 1
created_at: '2026-03-24T03:16:12.302973Z'
updated_at: '2026-03-24T03:16:12.302973Z'
labels:
- tui
- ux
verify: 'cd /Users/asher/tower && cargo test -p imp-tui 2>&1 | tail -1 | grep -q ''test result: ok'''
fail_first: true
---

Comprehensive UX improvement for imp's TUI (imp-tui crate).

## Problems
1. Tool calls and prose interleaved in one stream — hard to tell what happened
2. No progress indication beyond a spinner — no elapsed time, no tool count
3. Tool output is binary (all expanded or all collapsed) — no per-tool control
4. No summary after agent finishes — user reconstructs what changed from tool list
5. Editor has no placeholder text or keybinding hints for new users
6. Context window tracking is naive (cumulative, not actual conversation size)
7. No approval/confirmation flow for destructive operations

## Architecture
- imp-tui crate: /Users/asher/tower/imp/crates/imp-tui/src/
- Main app: app.rs (~2000 lines, handles events, rendering, state)
- Views: views/ (chat.rs, editor.rs, status.rs, tools.rs, etc.)
- Theme: theme.rs
- Keybindings: keybindings.rs
- Agent events come via AgentEvent enum from imp-core

## Phases
1. Foundation: Turn activity tracker (enables progress + summary)
2. Streaming: Progress indicator, per-tool expand, auto-expand errors
3. Summary: Turn-end summary with file change tracking
4. Visual: Better separation of activity vs content
5. Interactivity: Approval gates
6. Polish: Editor hints, context fix, keybinding discoverability
