---
id: '275'
title: Rethink imp TUI tool-call presentation and sidebar interaction model
slug: rethink-imp-tui-tool-call-presentation-and-sidebar
status: in_progress
priority: 2
created_at: '2026-04-27T05:19:53.262110Z'
updated_at: '2026-04-27T16:37:39.692596Z'
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

  ---
  2026-04-27T06:26:26.237141+00:00
  User reported first inspector pass is incomplete: inspector view opens but does not actually print/show the selected tool call, and tool output cannot be scrolled. Investigating render/selection/scroll wiring in `crates/imp-tui/src/app.rs` and `crates/imp-tui/src/views/sidebar.rs`; scope is bugfixing the existing inspector implementation, not redesigning it.

  ---
  2026-04-27T06:30:20.279221+00:00
  Fixed inspector bug report. Root causes: when inspector sidebar was open with no explicit `tool_focus`, detail rendering received `None` and showed only the placeholder instead of a tool; detail rendering also emitted only output lines, so the selected tool call header/args were absent. Changes: inspector detail now falls back to the latest tool call when no focus is set, selected inspector detail includes the focused tool header plus full output, and focusing any tool resets detail scroll for predictable scrolling from the top. Added focused regression tests in `crates/imp-tui/src/app.rs` and `crates/imp-tui/src/views/sidebar.rs`. Verification passed: `cargo test -p imp-tui inspector -- --nocapture`; `cargo check -p imp-tui`. Note: `cargo fmt --check` still reports pre-existing unrelated formatting diffs in `imp-lua` and the earlier Esc cancellation test, so I did not run global formatting.

  ---
  2026-04-27T06:36:05.652422+00:00
  Clarified user report: the broken behavior is not about a literal placeholder; inspector mode was effectively showing only LLM text after completion because tool call cards could disappear from chat when `chat_tool_display=interleaved` + inspector mode pushed details to sidebar, and the sidebar detail only showed transient tool output during execution. Added a config-level guard so inspector mode coerces interleaved chat tool display to summary (unless explicitly hidden), preserving persistent inline tool-call cards while inspector owns full output. Previous detail changes keep selected tool header/args and full output in inspector. Verification passed: `cargo test -p imp-core inspector_sidebar_keeps_tool_calls_in_chat_summary -- --nocapture`; `cargo test -p imp-tui inspector -- --nocapture`; `cargo check -p imp-tui -p imp-core`; `cargo fmt -p imp-core -p imp-tui --check`.

  ---
  2026-04-27T16:37:39.692590+00:00
  Continuing inspector bugfix from user report: inspector still is not printing tool calls in real use. Re-inspecting render/selection/cache path; likely issue remains in sidebar detail data source or selected-tool mapping rather than high-level design.
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
