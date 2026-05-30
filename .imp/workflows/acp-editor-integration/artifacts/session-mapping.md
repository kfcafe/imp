# ACP to imp session mapping

## Goals

- ACP sessions should feel stable in editors.
- imp should keep using its durable JSONL session storage rather than inventing ACP-only history.
- ACP session IDs should be predictable enough for load/resume and not leak filesystem paths.
- The adapter must honor ACP `cwd` for each session.

## Proposed mapping

Use imp's persisted session UUID as the ACP `sessionId`.

- `session/new` creates `SessionManager::new(&cwd, &global_sessions_dir())` and returns `session.session_id()`.
- The in-process ACP session table stores:
  - ACP/imp session ID
  - session file path
  - current cwd
  - current branch messages/history
  - active prompt join handle/command sender/cancel state, if any
  - pending client requests for permission/UI
  - client capabilities snapshot relevant to the session
- `session/load` finds the persisted session file by UUID, opens it with `SessionManager::open`, replays branch messages to the client via `session/update`, and loads the same messages into the in-process history.
- `session/resume` finds and opens the persisted session but does not replay updates.
- `session/close` removes only the in-process table entry. It cancels active work if present. It does not delete the JSONL file.

## Finding sessions by ID

`SessionManager::open(path)` takes a path, while ACP load/resume provides a session ID. Implementation should add or locally implement a lookup helper:

1. Validate session ID is a UUID-ish filename stem or a known active session ID.
2. Resolve `global_sessions_dir().join(format!("{session_id}.jsonl"))`.
3. Reject path traversal and non-file paths.
4. Open with `SessionManager::open`.
5. Verify the opened `session_id()` matches the requested ID.

Do not accept arbitrary paths as ACP session IDs.

## History persistence during prompt turns

Current `run_rpc_mode` only tracks `Vec<Message>` in memory. ACP should persist messages.

Preferred implementation:

- On `session/prompt`, append the converted user prompt to the `SessionManager` before or at turn start.
- Run the agent with `agent.messages` seeded from the active branch messages before the new user message, or adjust to avoid double-adding because `Agent::run(prompt)` itself pushes `Message::user(&prompt)`.
- After the turn finishes, persist newly added assistant/tool/context messages from the agent's final `messages` that are not already in the session.
- Use existing `SessionManager::append_user_message` / `append_assistant_message` where possible; if tool/result message persistence matters, inspect existing TUI persistence behavior and match it.

Open implementation detail: because `Agent::run(prompt)` appends the user prompt internally, the simplest safe first implementation may seed the agent with pre-prompt branch messages, call `run(prompt)`, then diff `agent.messages` against the seeded length and append the new messages in order to the `SessionManager`. This avoids duplicate user entries.

## Loading and replaying

For `session/load`, replay the active branch history in chronological order:

- User messages -> `session/update` with `user_message_chunk` and text content.
- Assistant messages -> `session/update` with `agent_message_chunk` and text content.
- Tool/result messages -> best-effort tool updates if they can be reconstructed, otherwise `_meta.imp` history entries or omitted from first replay with a documented limitation.

The replay must complete before sending the `session/load` response.

## CWD and workspace context

- ACP `cwd` must be absolute and must become `Agent.cwd` and the base for config resolution: `Config::resolve(global_root, Some(&cwd))`.
- The adapter should not use process cwd except as fallback before `session/new` or for logging.
- If client sends additional workspace roots through current/future ACP params, record them in session metadata or prompt context only after checking schema support. First version should at minimum honor primary `cwd`.

## Concurrent sessions and active turns

Design target:

- Multiple sessions may exist in the in-process table.
- Only one active prompt per session.
- Cross-session concurrent prompts can be deferred until the first usable version is stable. If the server serializes all active turns globally, return clear `busy` errors or queue intentionally.

Recommended first implementation:

- Support multiple session records.
- Allow one active turn globally at first only if simpler, but make the error explicit: `imp ACP currently supports one active prompt at a time; wait for the current prompt to finish or cancel it.`
- Better: allow one active turn per session by giving each session its own agent join handle and event forwarding task.

## Resume behavior

Advertise `sessionCapabilities.resume` only after:

- Load by session ID works.
- Agent history is seeded from the persisted branch.
- A follow-up `session/prompt` after `session/resume` continues the conversation coherently.

If these are not all true in the initial implementation, do not advertise resume and return unsupported if called.

## Cleanup

- On stdin close/process shutdown, cancel active turns and let the process exit.
- Persist completed turns before removing session state.
- Do not delete session files through ACP close.
- Pending permission requests should be dropped/cancelled when a turn is cancelled or session closes.
