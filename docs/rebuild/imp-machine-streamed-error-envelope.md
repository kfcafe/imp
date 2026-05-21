# Machine-facing streamed-error envelope recommendation

Unit `248.18.3` evaluated whether imp JSON and RPC streams should add a stable structured error envelope for streamed/provider failures.

## Current shapes

Inspected `crates/imp-cli/src/lib.rs` and related runtime-event conversion:

- Headless JSON legacy agent errors: `{"type":"error","error":"..."}`.
- Headless JSON legacy stream errors: `{"type":"stream_error","error":"..."}`.
- RPC legacy agent errors: `{"type":"error","error":"..."}`.
- RPC legacy stream errors: `{"type":"stream_error","error":"..."}`.
- RPC and runtime-json paths already attach `runtime_event` and `runtime_state` alongside the legacy payload.
- `AgentEvent::to_runtime_event` currently maps stream/agent errors into `RuntimeEventKind::Error { message }`.
- `imp-llm::StreamEvent::Error` is still a flat string, while `imp-core` stream diagnostics now classify important cases in the message text, e.g. partial-output failure and missing terminal completion event.

## Decision

Yes, streamed errors should gain a structured machine-facing envelope, but not by breaking existing flat `error` fields.

The next implementation should add an additive `error_info` object to machine-facing JSON/RPC events while preserving the current top-level `error` string for compatibility.

## Recommended envelope

```json
{
  "type": "stream_error",
  "error": "Provider stream failed after partial output: connection reset",
  "error_info": {
    "kind": "provider_stream_error",
    "display_message": "Provider stream failed after partial output: connection reset",
    "raw_message": "Provider stream failed after partial output: connection reset",
    "retryable": false,
    "stream_phase": "after_partial_output"
  }
}
```

Fields:

- `kind`: stable classification.
- `display_message`: concise normalized message safe for human display.
- `raw_message`: original surfaced message string.
- `retryable`: whether automatic retry is appropriate from the machine contract perspective.
- `stream_phase`: where the failure occurred.

## Initial classifications

Use conservative categories first:

- `provider_declared_error`
  - Provider sent an explicit `StreamEvent::Error` before meaningful assistant/tool output.
  - `retryable` may be true if retry policy says the provider error is retryable.
  - `stream_phase`: `before_output`.
- `provider_stream_error`
  - Provider or transport failed after partial meaningful stream output.
  - `retryable`: false, because replaying after partial output risks duplicate effects or confusing transcript state.
  - `stream_phase`: `after_partial_output`.
- `premature_stream_end`
  - Stream ended without a terminal `MessageEnd`.
  - `retryable`: false if any meaningful content/tool call already streamed; otherwise future policy may decide.
  - `stream_phase`: `missing_message_end`.
- `transport_error`
  - Lower-level stream transport failure before meaningful output.
  - `retryable`: based on retry policy.
  - `stream_phase`: `before_output`.
- `unknown`
  - Fallback when classification is unavailable.
  - `retryable`: null/unknown or false depending on serializer constraints.

## Compatibility rule

Do not remove or rename existing machine-facing fields in this slice:

- keep top-level `error` string;
- keep current event `type` values (`error`, `stream_error`, `protocol_error`);
- add `error_info` only when classification is available;
- if added to RPC, add the same object to headless JSON for equivalent event meanings;
- keep `runtime_event` additive, not a replacement for legacy fields until a versioned protocol migration exists.

## Where to implement

Prefer a small shared helper in `imp-core` near `error_display` or runtime event types, e.g. `MachineErrorInfo`, so CLI JSON and RPC serializers do not duplicate classification logic.

Likely implementation surface:

- add `MachineErrorInfo` or equivalent in `imp-core`;
- classify from the already surfaced error string plus optional stream context;
- have `legacy_json_event_value`, `stream_event_to_json`, `rpc_agent_event_legacy_json`, and `rpc_stream_event_to_json` include `error_info` consistently;
- optionally enrich `RuntimeEventKind::Error` later, but avoid making that a prerequisite if the additive legacy payload is the smallest safe slice.

## Tests to add if implemented

Focused tests in `imp-cli` should assert:

1. headless JSON stream error keeps `error` and adds `error_info.kind`;
2. RPC stream error keeps the same top-level `error` and same `error_info` payload;
3. partial-output wording maps to `provider_stream_error` + `after_partial_output` + `retryable=false`;
4. missing terminal completion maps to `premature_stream_end` + `missing_message_end`;
5. protocol errors keep their existing shape unless explicitly brought under the same envelope later.

## Non-goals

- No human rendering changes.
- No immediate JSON schema version bump.
- No removal of flat error fields.
- No broad RPC envelope redesign.
- No provider API error object redesign in `imp-llm`.

## Recommendation summary

Add structured machine error metadata as an additive compatibility field. Preserve legacy flat strings until a separate versioned machine-output migration removes them.
