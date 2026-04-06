---
id: '30'
title: Render compact widget/status surfaces already supported by imp UI abstractions
slug: render-compact-widgetstatus-surfaces-already-suppo
status: closed
priority: 1
created_at: '2026-03-26T03:27:21.781799Z'
updated_at: '2026-04-06T21:30:20.394744Z'
labels:
- feature
- ux
- imp-tui
closed_at: '2026-04-06T21:30:20.394744Z'
close_reason: 'Already complete in repo: widget state storage, SetWidget handling, render_widget_tray, and verification all pass. Closing stale-open unit after confirming the TUI widget surface exists.'
verify: cd /Users/asher/tower/imp && rg '_ => \{\} // SetWidget, Custom — not yet handled' crates/imp-tui/src/app.rs && exit 1; rg 'UiRequest::SetWidget' crates/imp-tui/src/app.rs && rg 'render_widget' crates/imp-tui/src/app.rs crates/imp-tui/src/views && cargo check -p imp-tui
fail_first: true
is_archived: true
history:
- attempt: 1
  started_at: '2026-04-06T21:30:20.110645Z'
  finished_at: '2026-04-06T21:30:20.381060Z'
  duration_secs: 0.27
  result: pass
  exit_code: 0
outputs:
  text: |-
    UiRequest::SetWidget { key, content } => {
            app.handle_ui_request(crate::tui_interface::UiRequest::SetWidget {
            app.handle_ui_request(crate::tui_interface::UiRequest::SetWidget {
    crates/imp-tui/src/app.rs:    fn render_widget_tray(&self, frame: &mut Frame, area: Rect) {
    crates/imp-tui/src/app.rs:        frame.render_widget(widget, area);
    crates/imp-tui/src/app.rs:        frame.render_widget(Clear, area);
    crates/imp-tui/src/app.rs:        frame.render_widget(top_bar, top_bar_area);
    crates/imp-tui/src/app.rs:            self.render_widget_tray(frame, widget_area);
    crates/imp-tui/src/app.rs:        frame.render_widget(chat, chat_area);
    crates/imp-tui/src/app.rs:                    frame.render_widget(view, sidebar_area);
    crates/imp-tui/src/app.rs:                    frame.render_widget(view, sidebar_area);
    crates/imp-tui/src/app.rs:            frame.render_widget(AskBar::new(state, &self.theme), editor_area);
    crates/imp-tui/src/app.rs:            frame.render_widget(editor, editor_area);
    crates/imp-tui/src/app.rs:        frame.render_widget(
    crates/imp-tui/src/app.rs:                frame.render_widget(view, overlay_area);
    crates/imp-tui/src/app.rs:                frame.render_widget(view, palette_area);
    crates/imp-tui/src/app.rs:                frame.render_widget(view, finder_area);
    crates/imp-tui/src/app.rs:                frame.render_widget(view, overlay_area);
    crates/imp-tui/src/app.rs:                frame.render_widget(view, overlay_area);
    crates/imp-tui/src/app.rs:                frame.render_widget(view, tree_area);
    crates/imp-tui/src/app.rs:                frame.render_widget(view, overlay_area);
    crates/imp-tui/src/app.rs:                frame.render_widget(view, overlay_area);
    crates/imp-tui/src/app.rs:                frame.render_widget(view, overlay_area);
    crates/imp-tui/src/app.rs:                frame.render_widget(view, overlay_area);
kind: epic
---

Finish the existing TUI plumbing for imp’s UI abstraction so compact runtime surfaces can actually appear in the interface. The core runtime already supports `set_status`, `set_widget`, and `custom`-style UI requests, but the TUI currently only handles status items and drops richer widget/custom requests. This unit should make the TUI able to render compact, non-intrusive widget content so future UX work has a real display surface.

Do the following:

1. Wire `SetWidget` requests through the TUI app.
   - Update the TUI request handling so `UiRequest::SetWidget` is no longer ignored.
   - Store active widget content in `App` state keyed by widget name.
   - Support clearing/removing widgets when content is `None`.

2. Render widgets in a compact, predictable place.
   - Add a small widget tray in the normal TUI layout.
   - Render widget content in a compact area near the top bar or just above the editor, without overwhelming the transcript.
   - Keep the main chat area dominant.
   - Widget content should feel like lightweight system state, not a second conversation stream.

3. Support the existing widget content shapes.
   - Render `WidgetContent::Lines(Vec<String>)` cleanly.
   - If `WidgetContent::Component(...)` is already easy to support, render a minimal useful fallback.
   - If full component rendering is too large for this unit, render a safe textual fallback instead of dropping it silently.

4. Preserve existing UX quality.
   - Do not break chat scrolling, sidebar rendering, ask overlay, or editor behavior.
   - Ensure the interface still behaves correctly when there are zero widgets, one widget, or multiple widgets.
   - Keep the visual treatment subtle and consistent with the current TUI style.

5. Make the feature testable.
   - Add targeted tests where practical for widget state storage/removal, layout/render behavior, or rendering helpers.
   - At minimum, make it easy to verify that widget requests no longer disappear.

6. Keep scope tight.
   - Do not implement checkpoint UX in this unit.
   - Do not implement planning UX in this unit.
   - Do not implement approval policy UX in this unit.
   - Do not add new core UI abstractions.
   - This unit is about rendering the existing abstraction, not inventing new product behavior.

Desired outcome: imp’s TUI has a working compact widget surface so runtime features can show lightweight status blocks without spamming the transcript. This creates the display foundation for future checkpoint, mana-plan, approval, or recall UX.
