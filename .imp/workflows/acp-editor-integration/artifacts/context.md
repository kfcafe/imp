# ACP editor integration context

## ACP protocol evidence

Sources inspected:

- https://agentclientprotocol.com/llms.txt — documentation index and current feature/RFD surface.
- https://agentclientprotocol.com/protocol/overview.md — communication model and baseline methods.
- https://agentclientprotocol.com/protocol/transports.md — stdio transport rules.
- https://agentclientprotocol.com/protocol/initialization.md — initialize request/response, protocol version, capabilities, implementation info.
- https://agentclientprotocol.com/protocol/session-setup.md — `session/new`, `session/load`, `session/resume`, `session/close`, cwd/session-id rules.
- https://agentclientprotocol.com/protocol/prompt-turn.md — `session/prompt`, `session/update`, cancellation, stop reasons.
- https://agentclientprotocol.com/protocol/tool-calls.md — tool call/update payloads and `session/request_permission`.
- https://agentclientprotocol.com/get-started/clients.md — ACP-compatible clients/editors list.
- https://agentclientprotocol.com/libraries/rust.md — official Rust SDK information.
- Zed external agents docs and JetBrains ACP docs were checked for client expectations.

Important protocol facts for imp:

- ACP is JSON-RPC 2.0. Current stable examples use `protocolVersion: 1`.
- The preferred/current transport is stdio: the client launches the agent as a subprocess, writes newline-delimited JSON-RPC messages to stdin, and reads newline-delimited JSON-RPC messages from stdout. stdout must contain only valid ACP messages; logs belong on stderr.
- Baseline client-to-agent methods: `initialize`, `authenticate` if required, `session/new`, `session/prompt`; optional/current session methods include `session/load`, `session/resume`, `session/close`, `session/set_mode`, `logout`, and `session/list` per stabilized announcements.
- Baseline agent-to-client method: `session/request_permission`. Optional client methods include `fs/read_text_file`, `fs/write_text_file`, and terminal methods. Agent-to-client notifications include `session/update` for text chunks, tool calls, plans, commands, and modes.
- `session/new` and load/resume requests carry an absolute `cwd`. ACP says cwd must be used for the session regardless of agent subprocess cwd and should bound file operations.
- `session/prompt` carries `ContentBlock[]`. Baseline support must include text and resource links; richer image/audio/embedded resource content is capability-gated.
- Prompt completion returns a `stopReason` such as `end_turn`, `cancelled`, `max_tokens`, `max_turn_requests`, or `refusal`. Cancellation is a `session/cancel` notification and must resolve the pending prompt request with `stopReason: cancelled`, not an opaque error.
- Tool UX is first-class: agents should send `tool_call` and `tool_call_update` session updates with stable tool call IDs, human titles, kind (`read`, `edit`, `execute`, etc.), status, content, locations, and raw input/output where useful.
- Permission UX is first-class: agents may call `session/request_permission` with option IDs/kinds and must handle selected/cancelled outcomes.

## Existing imp RPC and runtime seams

Source inspected:

- `crates/imp-cli/src/lib.rs#run_rpc_mode` (`2793-2897`): current JSONL RPC worker loop resolves cwd/config/model registry, spawns a stdout JSONL writer, creates `RpcUi`, reads stdin commands, tracks in-memory history, queues follow-ups while an agent turn is active, and stores the finished agent messages as next-turn history.
- `crates/imp-cli/src/lib.rs#parse_rpc_command` (`3133-3146`): current RPC input commands are simple JSON objects with `type: prompt|cancel|steer|followup` plus content.
- `crates/imp-cli/src/lib.rs#process_rpc_command` (`2900-2965`): prompt/followup spawn or queue agent turns, cancel sends `AgentCommand::Cancel`, steer sends `AgentCommand::Steer` only when active.
- `crates/imp-cli/src/lib.rs#create_rpc_agent` (`2995-3058`): builds an `Agent` with config/model/auth, Lua tool loader, optional system prompt, `RpcUi`, and prior `history`.
- `crates/imp-cli/src/lib.rs#forward_rpc_events` (`3175-3190`): converts `AgentEvent` into legacy JSON plus runtime event/state snapshots and sends over stdout.
- `crates/imp-cli/src/lib.rs#RpcUi` and `UserInterface for RpcUi` (`2643+`, `2670-2805`): maps imp UI operations to ad hoc `ui_request` messages with pending response IDs/timeouts.
- `crates/imp-cli/src/lib.rs#rpc_agent_event_legacy_json` (`3190+`): existing event mapping covers agent start/end, turn start/end/assessment, message start/delta/end, tool execution start/end, tool output deltas, warnings, timing, workflow/provenance/policy/evidence events, and errors.

Useful existing runtime types:

- `crates/imp-core/src/agent/mod.rs#AgentCommand` (`61-65`): `Cancel`, `Steer(String)`, `FollowUp(String)`.
- `crates/imp-core/src/agent/mod.rs#AgentHandle` (`158-162`): event receiver, command sender, cancel token.
- `crates/imp-core/src/agent/mod.rs#Agent` (`76-155`): contains model/tools/messages/cwd/mode/UI/session IDs/thread IDs/runtime state and command/event channels.
- `crates/imp-core/src/agent/events.rs#AgentEvent` (`143-225`): sufficient event surface for ACP message chunks, tool calls/updates, progress/status, policy, warnings, errors, and completion.
- `crates/imp-core/src/ui.rs#UserInterface` (`7-63`): sufficient abstraction for ACP `session/request_permission`-style confirm/select/input/custom requests.

Inference: the ACP adapter should reuse the existing JSONL RPC construction path and event stream but should not reuse the ad hoc wire protocol. ACP needs a typed JSON-RPC codec, session table, request/response correlation, and ACP-specific event mapper.

## Durable session model

Source inspected:

- `crates/imp-core/src/session.rs#SessionManager` (`176-182`): file-backed manager tracks entries, path, leaf ID, session name, and summary.
- `SessionManager::new` creates a new JSONL session file in a session directory with a UUID filename/header.
- `SessionManager::open` loads an existing JSONL session file side-effect free.
- `SessionManager::list`, `list_page`, and `continue_recent` support discovery/recent-session behavior.
- `SessionManager::append`, `append_user_message`, and `append_assistant_message` persist message entries and maintain branch leaf IDs.
- `SessionManager::branch_messages`/`get_branch` expose replayable branch history.
- `SessionManager::session_id` returns the header/session UUID; `path` exposes the backing file path.

Concern: current `run_rpc_mode` uses in-memory `Vec<Message>` history and does not obviously bind to `SessionManager`. For usable ACP daily use, `session/new` should create or associate a durable imp session, and prompt completion should persist message history. This may require extracting/borrowing session-handling behavior from TUI/headless paths rather than only wrapping JSONL RPC.

## Client/editor expectations

Sources inspected:

- ACP clients index lists Zed, JetBrains, VS Code via extension, Neovim plugins, Emacs `agent-shell.el`, Obsidian, Unity, desktop/web/mobile clients, and CLI/TUI clients.
- Zed docs confirm Zed launches external CLI agents through ACP and is UI-focused; billing/auth remains between user and the agent.
- JetBrains docs confirm custom ACP agents can be added manually or from registry.
- ACP registry has stabilized; registry support is useful polish but not required for first implementation.

Minimum client-facing behavior for a usable editor integration:

1. `imp acp` or equivalent must run as a quiet stdio subprocess. No non-JSON stdout.
2. `initialize` must return protocol version 1, truthful `agentInfo`, and only capabilities imp actually supports.
3. `session/new` must honor absolute `cwd` from the client rather than process cwd.
4. `session/prompt` must stream text and tool progress via `session/update`, and reply only after the turn finishes.
5. `session/cancel` must cancel the active turn and resolve the prompt with `cancelled`.
6. Permission prompts must be represented through `session/request_permission` instead of auto-approving policy-sensitive tool calls.
7. Errors for missing auth/model/provider/workspace must be actionable because editor users may not see TUI setup flows.
8. Logs/debugging should go to stderr and/or an explicit log file, never ACP stdout.

## Compatibility risks / open questions

- Whether to use the official Rust SDK (`agent-client-protocol`) or implement a local minimal typed protocol module. The SDK may reduce protocol drift and improve Zed compatibility, but adding a dependency needs review.
- Exact latest stable schema details should be pulled from the official schema/OpenAPI or SDK before coding; docs show protocol v1 and stabilized additions, but tests should pin exact wire shapes.
- ACP MCP server support is expected in `session/new`; imp currently has Lua tools/native tools and no obvious MCP client path in the inspected seams. First version should likely accept but explicitly ignore/reject unsupported MCP server configs, while advertising no MCP capabilities unless implemented.
- Durable ACP session mapping needs design: ACP session ID could equal imp session UUID, but current JSONL RPC does not persist sessions.
- Concurrent sessions: ACP supports multiple sessions per agent process conceptually. First usable version may serialize active turns per session or limit to one active turn at a time, but capabilities/errors must be clear.
- Request-permission mapping from generic `UserInterface` confirm/select/input to ACP permission options is straightforward for approvals, but arbitrary text input/custom components may need ACP extension/update behavior or a documented fallback.
