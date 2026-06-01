# ACP live wiring plan

## Target shape

The ACP server must become an async runtime adapter rather than a synchronous request mapper.

Core state:

```rust
struct AcpServer {
    version: String,
    initialized: bool,
    client: ClientState,
    sessions: HashMap<String, AcpSession>,
    outbound: AcpWriter,
    pending_client_requests: PendingClientRequests,
    next_request_id: AtomicU64,
}

struct AcpSession {
    id: String,
    cwd: PathBuf,
    config: Config,
    registry: ModelRegistry,
    session: SessionManager,
    active: Option<ActivePrompt>,
}

struct ActivePrompt {
    prompt_request_id: JsonRpcId,
    command_tx: mpsc::Sender<AgentCommand>,
    join: JoinHandle<(Agent, imp_core::Result<()>)>,
    seeded_message_len: usize,
    final_status: Arc<Mutex<Option<RunFinalStatus>>>,
}
```

The exact representation can vary, but the adapter needs these responsibilities.

## Server loop

Replace `handle_line -> Vec<JsonRpcResponse>` as the live path with async dispatch:

1. Read stdin lines.
2. Parse JSON-RPC.
3. For normal request methods, call async handler.
4. For `session/prompt`, start an active prompt task and do not write the request response until the task completes.
5. For notifications such as `session/cancel`, mutate active state and send `AgentCommand::Cancel`.
6. For client responses, route by id into `pending_client_requests`.
7. Allow event-forwarding tasks and permission requests to write outbound messages through a serialized writer channel.

Recommended writer design:

- one `mpsc::Sender<serde_json::Value>` for all outbound ACP messages;
- a single writer task serializes each value to stdout with a trailing newline;
- request handlers send response values through the writer instead of returning them when async/live behavior is needed.

This keeps stdout ordering deterministic enough and prevents concurrent writes.

## Agent construction reuse

Extract from `create_rpc_agent` into a host-neutral helper, for example:

```rust
fn create_host_agent(
    cli: &Cli,
    cwd: &Path,
    config: &Config,
    registry: &ModelRegistry,
    history: Vec<Message>,
    ui: Arc<dyn UserInterface>,
) -> Result<(Agent, AgentHandle), Box<dyn std::error::Error>>
```

`create_rpc_agent` becomes a thin wrapper that builds `Arc<dyn UserInterface>` from `RpcUi` and calls this helper.

ACP calls the same helper with `Arc<AcpUi>`. This preserves:

- provider/model/auth resolution;
- API key override behavior;
- thinking/system prompt override;
- Lua extension loading;
- builder/runtime defaults.

## Prompt execution flow

For `session/prompt`:

1. Validate initialized, known session id, no active prompt for that session.
2. Convert ACP `ContentBlock[]` to an imp prompt string.
3. Seed history from `session.session.get_active_messages()`.
4. Record `seeded_message_len = history.len()`.
5. Build agent with host-neutral helper and `AcpUi`.
6. Spawn event forwarding task using `AgentHandle.event_rx`:
   - map each `AgentEvent` to zero or more `session/update` notifications;
   - record `AgentEnd.status`, usage, and cost for prompt result metadata;
   - optionally track if cancellation/status occurs.
7. Spawn or await `agent.run(prompt)`.
8. After join completes, persist new messages by diffing `agent.messages[seeded_message_len..]`.
9. Return ACP `session/prompt` response with stopReason and `_meta.imp` usage/status/cost.
10. Clear active prompt state.

## Stop reason mapping

- `Ok(())` + final status complete/success-ish -> `end_turn`.
- `Err(Error::Cancelled)` or final `RunFinalStatus::Cancelled` -> `cancelled`.
- context too long / max tokens if distinguishable -> `max_tokens`.
- final failed/blocked/needs-user-input -> `refusal` initially, with `_meta.imp.status` containing the precise imp status.

Do not return raw cancellation errors as JSON-RPC errors; ACP requires a `cancelled` stop reason for prompt cancellation.

## Event forwarding

Use existing `agent_event_to_session_updates` as the first mapper, then improve:

- `MessageDelta::TextDelta` -> `agent_message_chunk`.
- `MessageDelta::ThinkingDelta` can be `agent_message_chunk` or a custom `_meta` update; avoid leaking hidden reasoning if not intended.
- `MessageDelta::ToolCall` -> `tool_call` pending.
- `ToolExecutionStart` -> `tool_call_update` in_progress.
- `ToolOutputDelta` -> `tool_call_update` content.
- `ToolExecutionEnd` -> `tool_call_update` completed/failed.
- `PolicyChecked` with denial -> tie to tool if possible; otherwise visible status/warning update.
- `AgentEnd` -> not a `session/update`; use it to form the prompt response.

## Error behavior

JSON-RPC errors should be reserved for request-level failures before/around a prompt:

- invalid params, unknown session, concurrent prompt, unsupported content, unsupported MCP.
- provider/auth/model setup failure before spawning the agent should be a JSON-RPC error with remediation.

Runtime failures after prompt execution begins should usually:

- send visible `session/update` error/warning;
- complete the prompt response with an appropriate stop reason and `_meta.imp.error`;
- use JSON-RPC prompt error only for unrecoverable protocol/server failures where no turn semantics apply.

## Capability truthfulness

Until implemented:

- keep `loadSession: false`;
- keep `sessionCapabilities.resume: None`;
- keep `sessionCapabilities.close: {}` if close cancels/removes active state;
- keep `promptCapabilities.embeddedContext: true` for embedded text resources;
- do not advertise image/audio/MCP/auth/logout.

After load/resume implementation and tests pass, advertise those capabilities.
