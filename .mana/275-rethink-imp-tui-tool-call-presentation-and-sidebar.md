---
id: '275'
title: Rethink imp TUI tool-call presentation and sidebar interaction model
slug: rethink-imp-tui-tool-call-presentation-and-sidebar
status: in_progress
priority: 2
created_at: '2026-04-27T05:19:53.262110Z'
updated_at: '2026-04-27T05:30:02.981598Z'
notes: |-
  ---
  2026-04-27T05:24:43.313002+00:00
  Initial code reconnaissance: imp TUI already has many pieces for the proposed model. Relevant files: crates/imp-tui/src/views/chat.rs renders DisplayMessage and already supports DisplayAssistantBlock::ToolCall, ChatToolDisplay, tool_focus, tool_line_indices, and build_click_map; crates/imp-tui/src/views/sidebar.rs renders both a tool stream/list and a detail pane from DisplayToolCall; crates/imp-tui/src/views/tools.rs owns DisplayToolCall summaries/header styling; crates/imp-tui/src/app.rs owns selection/focus/caches with Pane::{Chat,SidebarList,SidebarDetail}; crates/imp-core/src/config.rs has UiConfig.sidebar_style and ChatToolDisplay::{Interleaved,Summary,Hidden}. Design direction should likely reuse/adjust existing inline-tool rendering and repurpose sidebar default toward selected-tool detail/inspector rather than inventing a parallel model.

  ---
  2026-04-27T05:26:02.901488+00:00
  Implementation constraint discovered: inline tool calls are already first-class in the chat renderer (`DisplayAssistantBlock::ToolCall`, `ChatToolDisplay::Interleaved/Summary/Hidden`). The current default conflict is mainly sidebar behavior: `SidebarStyle::Stream` is default and renders a second chronological tool-output feed. Product direction should change default sidebar role to inspector/detail for the selected inline tool call, keep inline chat at Summary by default, and keep old stream/split behavior as config-compatible options during migration. Need implement carefully because app mouse handling and focus currently map split/sidebar list assumptions.

  ---
  2026-04-27T05:30:02.981580+00:00
  Implemented first pass of the selected design in the existing TUI architecture: added SidebarStyle::Inspector as new default; changed ChatToolDisplay default to Summary so tool calls stay inline as compact transcript cards; sidebar inspector renders selected tool detail using existing detail renderer; stream and split styles remain available in settings. Mouse/selection model: clicking an inline tool card focuses that tool and opens the sidebar; inspector area scroll/click routes to detail, not list. Errors no longer expand full output inline in default Summary mode; details are shown in selected inspector instead. Also updated settings cycling and labels for inspector style.
labels:
- imp
- tui
- ux
- tools
- design
verify: test -n "design-only"
kind: epic
---

Explore a TUI UX direction where tool calls appear inline in the main transcript stream as compact, clickable/selectable blocks, while the sidebar becomes a contextual inspector that shows the selected tool call's full input/output/details. Current idea from product discussion: replace always-on sidebar tool-call/output feed with inline tool call markers in the primary conversation, preserving chronological flow and reducing split-attention. Open design questions: how much output appears inline by default; keyboard navigation and focus model; how errors/progress/streaming states are represented; whether sidebar selection is sticky or follows cursor; how this affects transcript persistence/export and accessibility in terminal UI.
