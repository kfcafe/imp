# ACP permission and cancellation strategy

## AcpUi

Implement `AcpUi` as `UserInterface` for ACP sessions.

State:

- session id;
- outbound writer sender;
- pending client request map: JSON-RPC id -> oneshot sender;
- request id allocator;
- cancellation token/notification for prompt-level cancellation.

## Mapping UserInterface methods

Baseline support:

- `notify`: send visible `session/update` agent/status text or stderr? Prefer `session/update` if associated with a session.
- `confirm`: send ACP `session/request_permission` with `allow-once` and `reject-once` options.
- `select_with_context`: if options are approval-like, send `session/request_permission` with one option per selection. For arbitrary selection, either return `None` with a clear update or use a documented `_imp/select` extension later.
- `multi_select_with_context`: first pass may fall back to select or return `None` unless a clear ACP-compatible mapping exists.
- `input_with_context` and `custom`: ACP baseline does not define generic input. Return `None` and send a visible limitation update unless an approved extension is added.
- `set_status`/`set_widget`: map to lightweight `session/update` status text or ignore if purely decorative.

## `session/request_permission` shape

Outbound request:

```json
{
  "jsonrpc": "2.0",
  "id": "imp-acp-1",
  "method": "session/request_permission",
  "params": {
    "sessionId": "...",
    "toolCall": { "toolCallId": "...", "title": "...", "kind": "execute", "status": "pending" },
    "options": [
      { "optionId": "allow-once", "name": "Allow once", "kind": "allow_once" },
      { "optionId": "reject-once", "name": "Reject", "kind": "reject_once" }
    ]
  }
}
```

Response handling:

- `outcome.cancelled` -> `None`.
- selected `allow-*` -> true / selected allow index.
- selected `reject-*` -> false / selected reject index.
- unknown option -> `None` and visible warning.

## Client response routing

The server loop must handle `JsonRpcMessage::Response` by looking up the id in `pending_client_requests` and completing the oneshot.

Timeout behavior:

- use a reasonable timeout similar to `RpcUi` (currently 60s) or no timeout while prompt is active;
- on timeout, return `None` and emit a visible warning.

## Cancellation

`session/cancel` while active:

1. Find active prompt for session.
2. Send `AgentCommand::Cancel` through active `command_tx`.
3. Mark pending permission requests cancelled and complete their oneshots with a cancelled outcome where possible.
4. Event forwarding may still send late tool updates until prompt completes.
5. Prompt response must be `{ "stopReason": "cancelled" }`, not JSON-RPC error.

## Policy denials

Policy must remain enforced by imp core. ACP adapter should only display outcomes.

Mapping:

- If denial is tied to a tool call, send `tool_call_update` with status `failed` and text explaining denial.
- If not tied to a tool call, send visible `agent_message_chunk` or a custom status update.
- Include `_meta.imp.policy` later if useful, but keep user-facing text clear.

## Tests

Needed tests:

- confirm allow -> true.
- confirm reject -> false.
- request permission cancelled -> None.
- client response routes by JSON-RPC id.
- active prompt cancellation sends AgentCommand::Cancel and returns cancelled stop reason.
- policy denial event becomes a visible ACP update.
