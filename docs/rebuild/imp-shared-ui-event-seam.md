# Shared imp UI Request and Runtime Event Seam

imp has several user-facing surfaces: `imp chat`, `imp view`, RPC/machine clients, and `imp-tui`. They should share the same presentation-neutral request and runtime-event vocabulary instead of each surface inventing its own interaction semantics.

## Grounding in current code

- `crates/imp-core/src/ui.rs` already owns the canonical operator-request abstraction through `UserInterface`.
- `crates/imp-tui/src/tui_interface.rs` adapts `UserInterface` calls into fullscreen-local `UiRequest` messages.
- `crates/imp-tui/src/app.rs` consumes `UiRequest` and `StreamEvent`/runtime signals to update the TUI.
- `crates/imp-cli/src/lib.rs` hosts CLI shell, print/headless, view, and RPC flows, and currently serializes `StreamEvent` for machine clients in surface-local code.

This means the shared seam already exists conceptually: `imp-core` describes what the runtime needs from a presentation, while each surface decides how to render or transport it.

## Contract owners

`imp-core` owns the canonical semantic categories:

- operator requests;
- runtime/model/tool progress events;
- session and turn lifecycle facts;
- status/widget update intent;
- cancellation and completion outcomes.

Surfaces own presentation and transport:

- TUI layout, focus, keybindings, modal state, scrollback, colors, and widgets;
- CLI prompt text, terminal fallback behavior, and print formatting;
- `imp view` navigation, browsing, filtering, and read-only inspection UX;
- RPC framing, JSON envelope shape, request ids, and transport errors.

## Shared UI request categories

The shared request contract should cover the categories already represented by `UserInterface` and `UiRequest`:

1. **Notification**
   - Current grounding: `UserInterface::notify`, `UiRequest::Notify`.
   - Shared fields: message, level, optional scope/source.
   - Surface behavior: TUI toast/log entry, CLI stderr/stdout line, RPC event.
2. **Confirmation**
   - Current grounding: `UserInterface::confirm`, `UiRequest::Confirm`.
   - Shared fields: title, message, default/cancel semantics, optional consequence.
   - Surface behavior: modal, inline prompt, machine request/response.
3. **Single select**
   - Current grounding: `select_with_context`, `UiRequest::Select`.
   - Shared fields: title, context, options with labels/descriptions.
   - Surface behavior: list picker, numbered prompt, RPC choice request.
4. **Multi select**
   - Current grounding: `multi_select_with_context`, `UiRequest::MultiSelect`.
   - Shared fields: title, context, options, selection constraints if needed later.
   - Surface behavior: checkbox list, comma-separated CLI choices, machine response.
5. **Input**
   - Current grounding: `input_with_context`, `UiRequest::Input`.
   - Shared fields: title, context, placeholder, optional validation hint.
   - Surface behavior: text field, terminal prompt, RPC input request.
6. **Status update**
   - Current grounding: `set_status`, `UiRequest::SetStatus`.
   - Shared fields: stable key, optional text/value, source.
   - Surface behavior: TUI footer/status panel, CLI sparse status lines, RPC event.
7. **Widget/component update**
   - Current grounding: `set_widget`, `custom`, `WidgetContent`, `ComponentSpec`, `UiRequest::SetWidget`, `UiRequest::Custom`.
   - Shared fields: key/component spec/content and optional reply channel.
   - Surface behavior: TUI structured widget, CLI degraded text rendering, RPC structured payload.

## Shared runtime-event categories

Runtime events should stay presentation-neutral and be consumable by TUI, CLI, view, and RPC:

1. **Model stream events**
   - Grounding: `StreamEvent::MessageStart`, `TextDelta`, `ThinkingDelta`, `ToolCall`, `MessageEnd`, `Error`.
   - Shared meaning: model output/progress, not a surface rendering instruction.
2. **Tool/runtime progress**
   - Grounding: existing TUI runtime signals and tool-call rendering, workflow stream handling, and CLI JSON stream serializers.
   - Shared meaning: tool started/progressed/finished, with tool name, call id, status, and bounded display summary.
3. **Session lifecycle**
   - Shared meaning: session loaded, turn started, turn completed, interrupted, cancelled, or failed.
   - Surface behavior differs, but the lifecycle vocabulary should not.
4. **Policy and approval events**
   - Shared meaning: approval needed, denied, accepted, policy blocked, sandbox degraded.
   - Presentation handles operator confirmation or machine handoff.
5. **Status/widget changes**
   - Shared meaning: runtime wants a named status or widget updated.
   - Presentation chooses location, timing, and persistence.

## Mapping current types into the seam

`UiRequest` in `crates/imp-tui/src/tui_interface.rs` should be treated as a TUI-local transport for the shared `UserInterface` categories, not as the canonical fullscreen-only contract. The current variants map directly:

- `Notify` → shared notification request;
- `Confirm` → shared confirmation request;
- `Select`/`MultiSelect` → shared choice requests;
- `Input` → shared text input request;
- `SetStatus` → shared status update;
- `SetWidget`/`Custom` → shared component/widget update.

`AgentEvent`/`StreamEvent`-style runtime events should similarly be defined once by the runtime and then adapted outward:

- TUI consumes them into transcript rows, progress panels, footer state, and notifications.
- CLI consumes them into human terminal output.
- `imp view` consumes stored events/artifacts into browsing and inspection UI.
- RPC consumes them into stable JSON events for machine clients.

## What remains TUI-local

The TUI should keep ownership of:

- focus and mode state;
- keyboard bindings;
- modal layout and selection mechanics;
- scrollback and viewport calculations;
- render caches;
- colors, styling, truncation, and panel placement;
- fullscreen-specific batching/debouncing;
- local pending-reply bookkeeping for oneshot channels.

These details must not leak into shared request/event semantics.

## What remains CLI/view/RPC-local

- CLI owns prompt wording, stdout/stderr choices, non-interactive fallback messages, and terminal capabilities.
- `imp view` owns browse/navigation state, filters, timeline projection, and artifact inspection UX.
- RPC owns request ids, transport framing, protocol versioning, and JSON compatibility guarantees.

They should all adapt from the same semantic request/event categories.

## Design decision

The shared seam belongs in `imp-core`. `imp-tui` should consume it through an adapter boundary rather than define the product-wide request/event ontology. Machine surfaces should serialize the same semantic seam rather than duplicate new surface-local schemas.

## Follow-up implementation direction

1. Keep `UserInterface` as the current canonical request abstraction.
2. Promote any missing runtime lifecycle categories into shared imp-core event types before adding more TUI-only runtime signals.
3. Deduplicate CLI/RPC `StreamEvent` serializers around the shared event vocabulary.
4. Keep presentation state local to each surface.
