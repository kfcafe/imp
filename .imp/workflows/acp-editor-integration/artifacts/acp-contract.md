# ACP adapter contract for imp

## Scope and target

This contract targets ACP protocol version 1 over the stable stdio transport. The goal is a usable editor integration with truthful capabilities and strong quality-of-life behavior, not a custom editor extension.

The first production entrypoint should be:

```sh
imp acp [--provider <provider>] [--model <model>] [--thinking <level>] [--system-prompt <text>] [--api-key <key>] [--verbose]
```

`imp rpc` remains the existing imp-native JSONL worker protocol. ACP is a separate wire protocol and must not share stdout messages with `imp rpc`.

## Transport

- Local stdio subprocess only.
- Input: one UTF-8 JSON-RPC 2.0 request/notification/response per line.
- Output: one UTF-8 JSON-RPC 2.0 request/notification/response per line.
- stdout must contain only valid ACP JSON-RPC envelopes.
- Logs, startup timing, warnings not represented as ACP messages, and debug output go to stderr or a log file.
- Server should keep running until stdin closes, fatal initialization failure, or client exits.

## Lifecycle

1. Client launches `imp acp`.
2. Client calls `initialize`.
3. Optional auth check occurs through normal imp credential resolution. The first version should return actionable JSON-RPC errors for missing credentials rather than implementing ACP `authenticate`, unless a design pass decides to expose existing `imp login` through ACP.
4. Client calls `session/new`, or `session/load`/`session/resume` if advertised.
5. Client calls `session/prompt` for each user turn.
6. Agent sends `session/update` notifications during the turn.
7. Agent may call client `session/request_permission` when imp UI/policy requires confirmation/selection.
8. Client may send `session/cancel` notification for active work.
9. Optional `session/close` can cancel active work and release server-side session state.

## Initialize response

Initial advertised capabilities should be conservative:

```json
{
  "protocolVersion": 1,
  "agentCapabilities": {
    "loadSession": true,
    "promptCapabilities": {
      "embeddedContext": true
    },
    "sessionCapabilities": {
      "resume": {},
      "close": {}
    }
  },
  "agentInfo": {
    "name": "imp",
    "title": "imp",
    "version": "0.3.0"
  },
  "authMethods": []
}
```

Capability notes:

- `loadSession`: advertise only if ACP session IDs map to persisted imp sessions and history replay is implemented.
- `sessionCapabilities.resume`: advertise only if imp can restore history without replaying it.
- `sessionCapabilities.close`: straightforward if the server tracks active sessions and can cancel active turns.
- `promptCapabilities.embeddedContext`: advertise if `resource` content can be converted into imp prompt text/file-context blocks. Text support is baseline and need not be advertised.
- Do not advertise image/audio until imp prompt ingestion preserves those content types.
- Do not advertise `mcpCapabilities` until imp can connect to ACP-provided MCP servers.
- Do not advertise auth/logout until ACP auth is deliberately implemented.

If implementation starts with fewer capabilities, the initialize response must be reduced rather than faking support.

## Supported client-to-agent messages

### `initialize`

- Validate JSON-RPC and params shape.
- Negotiate protocol version: support v1. If the client proposes a later major version, respond with `protocolVersion: 1` and let the client decide whether to continue.
- Store client capabilities and client info in connection state.

### `session/new`

Params consumed:

- `cwd`: required absolute path. Must become the imp session cwd regardless of process cwd.
- `mcpServers`: accepted syntactically but unsupported unless MCP client support is added. If non-empty and unsupported, prefer a clear JSON-RPC error or warning depending on ACP conformance/client tolerance. Do not advertise MCP support.

Result:

```json
{ "sessionId": "<imp-session-uuid>" }
```

### `session/load`

If advertised:

- Open the persisted imp session by ACP/imp session ID.
- Set cwd from params after validating absolute path.
- Replay conversation history as `session/update` notifications in chronological branch order.
- Respond `null` after replay completes.

If not implemented, do not advertise `loadSession` and return method-not-found/unsupported if called.

### `session/resume`

If advertised:

- Open the persisted imp session by ID.
- Load messages into server memory without replay.
- Respond `{}`.

### `session/prompt`

- Requires initialized connection and known session ID.
- Reject concurrent prompts for the same session with a clear JSON-RPC error, unless queuing is explicitly implemented. ACP clients can send a follow-up after the prior prompt response.
- Convert `ContentBlock[]` to an imp user prompt:
  - `text` blocks concatenate as user text.
  - `resource` blocks with text become structured file/resource context in the prompt, with URI/path labels.
  - `resource_link` blocks become a short reference line unless fetched through a supported client fs method later.
  - unsupported content types return an actionable unsupported-content error unless safely ignorable.
- Run the imp agent turn using existing `Agent::run` machinery.
- Stream mapped `AgentEvent`s as `session/update` notifications.
- Complete the JSON-RPC request with `{ "stopReason": "end_turn" }`, `{ "stopReason": "cancelled" }`, or another mapped stop reason.

### `session/cancel`

- Notification only.
- Requires session ID.
- Sends `AgentCommand::Cancel` to active turn and/or sets the active cancel token.
- Ensures the pending `session/prompt` request resolves with `stopReason: "cancelled"` when cancellation wins.
- Cancels/settles pending permission requests as cancelled when possible.

### `session/close`

- If advertised, cancel active work for the session and remove it from the in-process active session table.
- Do not delete persisted imp session files.
- Return `{}`.

## Agent-to-client messages

### `session/update`

All updates use:

```json
{
  "jsonrpc": "2.0",
  "method": "session/update",
  "params": {
    "sessionId": "...",
    "update": { "sessionUpdate": "..." }
  }
}
```

Event mapping:

| imp event | ACP update | Notes |
|---|---|---|
| `AgentEvent::MessageDelta` text | `agent_message_chunk` | Preserve chunk order. |
| `AgentEvent::MessageStart/End` | usually no-op or `_meta` status | Avoid duplicate text if deltas are emitted. |
| `AgentEvent::ToolExecutionStart` | `tool_call` | `toolCallId` = imp call id; title from tool name/args; kind mapped by tool category; status `pending` or `in_progress`. |
| `AgentEvent::ToolOutputDelta` | `tool_call_update` | status `in_progress`; content text chunk. |
| `AgentEvent::ToolExecutionEnd` | `tool_call_update` | status `completed` or `failed`; include content text and raw output/details where useful. |
| `AgentEvent::Warning` | `agent_message_chunk` or custom `_meta` update | Prefer visible but non-scary text/status. |
| `AgentEvent::PolicyChecked` | `tool_call_update` or warning/status | Policy denials should be visible and tied to the relevant tool where possible. |
| `AgentEvent::Error` | prompt error or visible update | If terminal for the turn, use JSON-RPC error or stop reason; otherwise visible status. |
| workflow/evidence/verification/worktree events | tool/status updates with `_meta.imp` | Preserve useful progress without forcing clients to understand imp internals. |
| `AgentEvent::AgentEnd` | no update; prompt response stop reason | Usage/cost can go in `_meta.imp` on the prompt result if desired. |

Tool kind mapping:

- read/search/extract/scan/references/impact -> `read` or `search`
- edit/write -> `edit`
- bash/audit/test/build commands -> `execute`
- web -> `fetch`
- workflow/planning/internal assessment -> `think` or `other`
- unknown Lua tools -> `other`

### `session/request_permission`

Use for `UserInterface::confirm` and selection flows when the request gates a policy/tool decision.

- `confirm`: options `allow-once` and `reject-once`.
- `select`: map each `SelectOption` to an ACP permission option when it is approval-like; otherwise use a documented imp extension or custom component fallback.
- `input`/`custom`: ACP baseline does not define generic elicitation; use a documented unsupported/fallback path in first version, or an `_imp/input` custom method only after checking client support.

Permission outcomes:

- selected `allow-*` -> true/selected index.
- selected `reject-*` -> false/rejected index.
- cancelled -> `None` to imp UI, preserving current cancellation behavior.

## Errors

Use standard JSON-RPC errors with actionable messages. Recommended categories:

- parse error / invalid request / method not found: standard JSON-RPC codes.
- invalid params: standard `-32602` with field context.
- not initialized, unknown session, concurrent prompt, unsupported content, unsupported MCP servers: implementation-defined negative codes with stable messages.
- missing auth/provider/model setup: implementation-defined error with remediation, e.g. `Run imp login <provider> in a terminal, or set --api-key/IMP_* as documented.`

Do not expose Rust backtraces or raw provider secrets. Do include provider/model names and docs hints where useful.

## Unsupported in first usable version unless explicitly implemented

- HTTP/WebSocket ACP transport.
- MCP server connections supplied by the client.
- Image/audio prompt content.
- Client filesystem and terminal delegation (`fs/*`, `terminal/*`) unless imp tools are explicitly moved to those client methods.
- Generic structured elicitation beyond permission-like approval.
- ACP registry packaging.
- Multi-agent-process collaboration or remote service semantics.
