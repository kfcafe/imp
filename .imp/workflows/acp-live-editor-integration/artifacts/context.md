# ACP live editor integration context

## Prior workflow results reviewed

Source: `.imp/workflows/acp-editor-integration/results.md` and implementation artifacts.

The previous workflow intentionally shipped only a scaffold:

- `imp acp` subcommand exists.
- ACP JSON-RPC parsing/serialization and stdio loop exist.
- `initialize` and `session/new` work.
- `session/new` validates absolute `cwd` and creates a durable imp `SessionManager` file.
- `session/prompt` currently returns a scaffolded completion response instead of running the agent.
- `session/cancel` currently marks scaffold state only.
- Initial event/content mapping helpers exist.
- docs/acp.md truthfully says the adapter is scaffold-level.

Remaining concerns from prior closeout:

- Wire `session/prompt` to real `Agent::run`.
- Stream provider text/tool progress to ACP `session/update` notifications.
- Implement `session/request_permission` for imp UI/tool approvals.
- Map policy denials to useful ACP UX.
- Implement real active-turn cancellation.
- Implement or do not advertise `session/load` / `session/resume`.
- Clean up/deal with dead-code warnings from scaffold helpers.

## Scaffold code reviewed

Source: `crates/imp-cli/src/acp/mod.rs`, `protocol.rs`, `events.rs`.

Current `AcpServer` shape:

- Owns `version`, `initialized`, and `sessions: HashMap<String, AcpSession>`.
- `AcpSession` currently stores `cwd`, `path`, `cancelled`, and `prompt_count`.
- `run_stdio_server_with_io` reads newline-delimited stdin and writes JSON-RPC responses only. It currently only emits responses from `handle_line`; there is no outbound request/notification channel yet.
- `handle_line` parses request/notification/response. Responses from clients are ignored today.
- `handle_request` handles initialize, session/new, close, load/resume unsupported errors, and prompt scaffold.
- `handle_session_prompt` converts ACP prompt blocks to text but only returns `_meta.imp.status = stubbed`.
- `session_update_message` and `event_update_messages` already wrap `AgentEvent` mappings as ACP `session/update` JSON values, but are unused by live runtime.

Current protocol support:

- JSON-RPC id as number/string/null.
- Requests: initialize, session/new, session/load, session/resume, session/prompt, session/close.
- Notification: session/cancel.
- Content blocks: text, resource with embedded text, resource_link, unknown.
- Updates: agent/user message chunks, tool_call, tool_call_update.
- Prompt result stop reasons.
- Missing protocol pieces for live work: outbound `session/request_permission` request type/result parsing; possibly generic outbound JSON-RPC request IDs and response routing.

Current event mapping:

- Text deltas -> `agent_message_chunk`.
- stream tool calls -> `tool_call` pending.
- tool execution start/delta/end -> `tool_call_update`.
- warnings/errors -> visible `agent_message_chunk` text.
- history message replay helper exists through `message_to_session_updates`.
- Gaps: AgentEnd status/cost/usage to prompt result `_meta`, TurnEnd/MessageEnd handling, policy-denial specific mapping, terminal/diff content, paths/locations.

## Runtime seams reviewed

Sources:

- `crates/imp-cli/src/lib.rs#create_rpc_agent`
- `crates/imp-cli/src/lib.rs#spawn_rpc_agent`
- `crates/imp-cli/src/lib.rs#forward_rpc_events`
- `crates/imp-core/src/agent/run_loop.rs#run`
- `crates/imp-core/src/ui.rs#UserInterface`
- `crates/imp-core/src/agent/events.rs#AgentEvent`
- `crates/imp-core/src/session.rs` session helpers.

Reusable seams:

1. Agent construction
   - `create_rpc_agent` already performs provider/model/auth resolution, applies thinking/system prompt overrides, initializes Lua extensions, builds the agent, sets UI, and seeds `agent.messages` from history.
   - Best next step is to extract a host-neutral helper from this function that accepts `Arc<dyn UserInterface>` instead of `Arc<RpcUi>`. ACP can then reuse provider/auth behavior without duplicating it.

2. Agent spawning and cancellation
   - `spawn_rpc_agent` shows the shape: build agent, clone `handle.command_tx`, spawn event forwarding, spawn `agent.run(prompt)` and return the join handle.
   - `AgentHandle` exposes `event_rx`, `command_tx`, and cancel token.
   - `AgentCommand::Cancel` is the right active-turn control. `Agent::run` checks commands before turns and during streaming/tool execution and returns `Error::Cancelled` if cancelled.

3. Event forwarding
   - `forward_rpc_events` receives `AgentEvent` from `AgentHandle.event_rx` and serializes events to stdout writer state.
   - ACP needs a similar task/loop, but it must send `session/update` notifications and also capture terminal `AgentEnd` status for the prompt response.

4. User interaction
   - `UserInterface` supports notify, confirm, select, multi-select, input, status/widget/custom.
   - ACP baseline permission flow maps cleanly to approval-like `confirm`/`select` only. Generic input/custom need either a documented unsupported fallback or an ACP extension later.
   - Existing `RpcUi` is a useful pattern: it tracks request IDs and pending oneshot responses. ACP needs a variant that sends outbound JSON-RPC `session/request_permission` and routes inbound client responses.

5. Sessions
   - `SessionManager::new` creates the persistent JSONL session file/header.
   - `SessionManager::open` opens by path and is side-effect free.
   - `SessionManager::append` persists generic message entries and maintains parent/leaf.
   - `append_assistant_turn_with_model_meta` records assistant messages and canonical usage entries.
   - `append_tool_result_message` persists tool results.
   - `get_active_messages` returns branch messages suitable for seeding a new agent.

## Exact seams to reuse

- Extract or add `create_host_agent(cli, cwd, config, registry, history, ui)` in `imp-cli/src/lib.rs` or a small internal module, used by both RPC and ACP.
- Keep ACP protocol/session loop in `crates/imp-cli/src/acp`.
- Add an ACP stdout writer abstraction that can send both responses and notifications, not just return Vec<JsonRpcResponse> from sync handlers.
- Model active prompts explicitly in `AcpSession`:
  - session id
  - cwd
  - session path / `SessionManager`
  - history length before run
  - active command_tx/join handle or an active task record
  - pending prompt response id
  - final AgentEnd status/usage/cost
- Persist messages after the agent join returns by diffing final `agent.messages` against the seeded history length. Because `Agent::run(prompt)` appends the user prompt internally, do not pre-append the user prompt before running.

## Blockers / risks

- Current `AcpServer::handle_line` is synchronous and returns responses immediately. Live `session/prompt` must be async and hold the request open while events stream. This likely requires changing the server loop to dispatch async request handling or special-case prompt tasks.
- Current writer only writes responses returned by `handle_line`; ACP live mode needs outbound notifications and outbound client requests (`session/request_permission`) while a prompt is active.
- Client JSON-RPC responses are currently ignored. Permission support requires a pending response map keyed by JSON-RPC id.
- Tests need a fake/static provider path. Existing imp-cli tests include `StaticTestProvider` under `mana-ui` cfg in lib tests; we may need a non-feature-gated fake provider helper or test at the event/agent-construction seam.
- Dead-code warnings are expected until live wiring uses scaffold helpers; the workflow has a warning review gate.
- The worktree is already dirty with scaffold changes from the prior workflow. Preserve and build on those changes; do not reset.
