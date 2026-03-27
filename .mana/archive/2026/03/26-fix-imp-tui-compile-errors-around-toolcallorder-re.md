---
id: '26'
title: Fix imp-tui compile errors around tool_call_order references
slug: fix-imp-tui-compile-errors-around-toolcallorder-re
status: closed
priority: 1
created_at: '2026-03-25T03:54:16.084321Z'
updated_at: '2026-03-25T04:02:00.459163Z'
labels:
- bug
- compile
- imp-tui
closed_at: '2026-03-25T04:02:00.459163Z'
verify: cargo check -p imp-tui
fail_first: true
is_archived: true
history:
- attempt: 1
  started_at: '2026-03-25T04:02:00.180249Z'
  finished_at: '2026-03-25T04:02:00.439021Z'
  duration_secs: 0.258
  result: pass
  exit_code: 0
---

`cargo check -p imp-tui` currently fails because `imp/crates/imp-tui/src/app.rs` references `imp_core::config::ToolCallOrder` and `config.ui.tool_call_order`, but the current `UiConfig` does not expose that type/field. Restore compilation by aligning `imp-tui` with the current config surface area. Scope is only the compile break; do not continue the OpenAI model work here. Verify with a scoped check so `imp-tui` builds cleanly again.
