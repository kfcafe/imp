---
id: '52'
title: Fix sticky error formatting after hook failures and ask-tool interactions
slug: fix-sticky-error-formatting-after-hook-failures-an
status: open
priority: 1
created_at: '2026-04-16T03:41:04.568426Z'
updated_at: '2026-04-16T04:32:58.369895Z'
notes: |-
  ---
  2026-04-16T03:49:05.257990+00:00
  Implemented the sticky error-formatting fix across imp-core and imp-tui. Root causes were twofold: (1) non-blocking hook failures were emitted as AgentEvent::Error, which made transient warnings render as error messages; changed these to AgentEvent::Warning and routed UI Notify levels in the TUI so Warning/Info become non-error chat messages. (2) TUI agent event handling updated only self.messages.last_mut(), so if an ask flow or notification appended a system/error message after the streaming assistant message, later tool deltas/results could miss the actual in-flight assistant/tool state and leave stale error-looking UI. Added latest_streaming_message_mut() and find_tool_call_mut() helpers and switched MessageDelta, ToolExecutionStart, ToolOutputDelta, ToolExecutionEnd, AgentEnd, and error teardown to target the actual streaming assistant/tool by id instead of the latest chat message. Added focused tests warning_notify_uses_system_role_not_error_role and tool_updates_target_streaming_assistant_not_latest_message. Verified with targeted imp-tui tests and cargo check -p imp-tui.

  ---
  2026-04-16T04:12:52.121085+00:00
  User asked for another pass so warnings are more visually distinct, not just no longer errors. Planned refinement: add a dedicated Warning display role in the TUI instead of reusing System, render warning chat lines with theme.warning_style() and a visible 'Warning:' prefix, and update any narrow tests/export paths that currently collapse warnings into system text.

  ---
  2026-04-16T04:14:22.471991+00:00
  Did the visual-distinction pass. Added a dedicated MessageRole::Warning in crates/imp-tui/src/views/chat.rs instead of collapsing warnings into System. Warnings now render with theme.warning_style() and a visible 'Warning:' prefix in chat, export as '*Warning:*', and the existing notify-level routing now lands warnings in this role. Also updated the copy-last-message helper to treat Warning as a copyable surfaced result alongside Assistant/Error. Added chat rendering regression test warning_messages_render_with_prefix and updated warning_notify_uses_system_role_not_error_role to expect MessageRole::Warning. Re-ran targeted tests plus cargo check -p imp-tui.

  ---
  2026-04-16T04:32:58.369890+00:00
  Follow-up from user trying to install locally: cargo check -p imp-cli exposed a compile blocker from the new AgentEvent::Warning variant not yet handled in imp-cli JSON/RPC event serialization. Need a small compatibility patch in crates/imp-cli/src/lib.rs before local install can succeed. Also user hit the virtual-manifest install error from running cargo install at the workspace root; the correct package manifest is crates/imp-cli/Cargo.toml or use cargo build -p imp-cli then copy target/debug/imp.
labels:
- bugfix
- tui
- ui-state
- hooks
- ask
verify: cd /Users/asher/imp && cargo test -p imp-tui error -- --nocapture && cargo check -p imp-tui
kind: job
---

Goal: fix imp UI state so tool/text output does not remain styled as an error after a non-blocking hook failure or ask-tool interaction resolves.

Current state:
- User reports that after a hook-related error resolves, subsequent text output still renders with error formatting.
- User also sees the same sticky error-state behavior when agents use the ask tool.
- Need to inspect TUI/app rendering and event/state transitions for tool results, prompts, follow-ups, and hook background error reporting.

Steps:
1. Inspect imp-core/imp-tui code paths that surface non-blocking hook failures and ask tool interactions to the UI.
2. Identify where error styling state is stored and why it is not cleared on subsequent normal output.
3. Implement the smallest reversible fix so error styling reflects the current message/output state rather than leaking from prior events.
4. Add focused regression coverage if there is an existing test seam; otherwise add the narrowest feasible test around the affected state transition.
5. Verify with targeted tests/builds.

Files:
- crates/imp-core/src/agent.rs (read — hook failure event emission if relevant)
- crates/imp-core/src/ui.rs (read/modify if shared UI status state is involved)
- crates/imp-tui/src/app.rs (likely modify — rendered state + styling reset)
- crates/imp-tui/src/views/* (read/modify if styling lives there)
- crates/imp-core/src/tools/ask.rs (read — ask event shape)

In scope:
- Sticky error formatting after resolved hook failure
- Sticky error formatting around ask tool flows

Out of scope:
- Redesigning overall notification UX
- Broad restyling unrelated surfaces

Do not:
- Guess about repo behavior without inspecting the exact state transitions
- Claim success without verifying the reset path
