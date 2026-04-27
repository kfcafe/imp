---
id: '278'
title: Fix inspector mode interaction model
slug: fix-inspector-mode-interaction-model
status: open
priority: 2
created_at: '2026-04-27T17:21:39.007903Z'
updated_at: '2026-04-27T17:50:41.879696Z'
acceptance: Tool summaries are visible in chat by default for inspector mode; clicking a summary opens/focuses that tool in inspector; new tool calls do not steal focus from a manually selected older tool; streaming output auto-follows latest only when not manually pinned/scrolled; /settings clearly separates chat tool summaries, sidebar inspector mode, and output verbosity; targeted imp-tui/imp-core tests pass.
notes: |-
  ---
  2026-04-27T17:22:13.384083+00:00
  Inspected current inspector/sidebar implementation. Key findings: inspector style is a single detail pane using the full sidebar area; `selected_tool_call()` defaults to latest tool when no explicit focus; `focus_tool()` opens sidebar and resets detail scroll; `ToolExecutionStart` currently focuses the running tool and sets inspector/stream detail_scroll to `usize::MAX`; mouse clicks in inspector detail start text selection rather than choosing tools because there is no list pane. Relevant files: `crates/imp-tui/src/views/sidebar.rs`, `crates/imp-tui/src/app.rs`, config in `crates/imp-core/src/config.rs`. Need user clarification before implementing UX changes.

  ---
  2026-04-27T17:29:41.740908+00:00
  User clarified intended inspector behavior: inspector should combine latest/running details with selected tool details; new tools must not steal focus from a manually inspected older tool; streaming output should auto-scroll unless user scrolls; navigation should primarily happen by clicking tool summaries in chat. Root pain: tool call summaries were hidden due to Chat Tool Display setting; changing from hidden to summary/interleaved made behavior closer to desired. Need revisit settings terminology/organization for tool call vs output display and /settings UX.

  ---
  2026-04-27T17:37:20.027130+00:00
  User asked to externalize the durable plan/decomposition before continuing implementation. Root-scope epic should own cross-project/product direction; this project-local task can remain the immediate imp implementation thread.

  ---
  2026-04-27T17:48:34.792722+00:00
  Verification update: fixed config test expectation to match the new inspector invariant (inspector always keeps summary cards visible even if legacy hide/hidden settings are set). `cargo test -p imp-core inspector_sidebar_keeps_tool_calls_in_chat_summary -- --nocapture` passes. The combined imp-tui command `cargo test -p imp-tui inspector settings -- --nocapture` was invalid cargo syntax (`unexpected argument 'settings'`), so rerunning inspector and settings filters separately.

  ---
  2026-04-27T17:50:41.879687+00:00
  Implemented/verified pass-1 inspector behavior in current tree: inspector effective chat display keeps summaries visible even if legacy hide flag is set; settings apply primary inspector model (inspector layout, full output, summary chat display, hide false); settings no longer expose sidebar style/tool output/tool output lines/chat tool display fields; inspector focus now distinguishes pinned manual selection from auto-follow and new tool starts do not steal pinned focus. Verified with `cargo test -p imp-core config::tests::inspector_sidebar_keeps_tool_calls_in_chat_summary`, `cargo test -p imp-tui settings -- --nocapture`, and `cargo test -p imp-tui inspector -- --nocapture`. Note: repo has unrelated dirty files outside this scope.
labels:
- tui
- inspector
- ux
kind: task
decisions:
- 'Consider simplifying TUI tool-call UI around a single next-generation inspector mode as the default and potentially removing split/stream modes. Rationale: reduces settings confusion, keeps compact summaries in chat as navigation, frees sidebar as a general detail/action surface. Future sidebar direction may include editable/manipulable text that can apply changes back to codebase, but that is explicitly nontrivial and out of immediate scope.'
---

Fix inspector mode and tool-call display UX. Confirmed direction: inspector should show live/latest tool details when no manual selection, but preserve manual selection when user clicks a tool summary in chat; streaming output should auto-scroll unless user manually scrolls; navigation should primarily happen by clicking tool summaries in chat. Primary issue discovered by user: chat tool summaries can be hidden via Chat Tool Display/legacy hide setting, making inspector feel broken. Implementation should review and simplify the settings model for tool calls vs outputs and improve /settings organization/labels/help text before or alongside behavior changes.
