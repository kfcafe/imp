# ACP session strategy

## Session ownership

Each ACP session should own:

- ACP/imp session id (same UUID returned by `SessionManager::session_id`).
- Absolute cwd from `session/new` / load / resume.
- Resolved `Config` for that cwd.
- A `SessionManager` opened or created for the session file.
- Active prompt state, if any.

Do not use process cwd after `session/new` except as a pre-session fallback for config/auth paths that are explicitly global.

## Creating sessions

`session/new`:

1. Validate `cwd` is absolute.
2. Reject or clearly warn on non-empty `mcpServers` while unsupported.
3. Create `SessionManager::new(&cwd, global_sessions_dir())`.
4. Resolve `Config::resolve(global_root, Some(&cwd))` and store it in session state.
5. Return `session.session_id()`.

## Prompt persistence without duplicates

`Agent::run(prompt)` appends the user prompt internally. Therefore:

1. Before running, read `history = session.get_active_messages()`.
2. Set `seeded_message_len = history.len()`.
3. Build agent with `agent.messages = history`.
4. Call `agent.run(prompt)`.
5. After completion, iterate `agent.messages[seeded_message_len..]` and append each new message to `SessionManager` in order.

Persistence rules:

- `Message::User` -> append as `SessionEntry::Message` with a new UUID.
- `Message::Assistant` -> prefer `append_assistant_turn_with_model_meta` when model metadata is available so usage records remain canonical.
- `Message::ToolResult` -> `append_tool_result_message`.

This approach persists exactly what the agent runtime saw and avoids duplicating the prompt.

## Load/resume

Do not advertise load/resume until implemented.

When implemented:

- `session/load`:
  - safe-resolve session ID to `global_sessions_dir/<id>.jsonl`;
  - open with `SessionManager::open`;
  - verify opened id matches requested id;
  - replay `get_active_messages()` via `message_to_session_updates` before responding;
  - store session in active table.
- `session/resume`:
  - same safe open/verify/store path;
  - do not replay history;
  - respond `{}`.

## Concurrency

Minimum usable behavior:

- multiple sessions can exist;
- one active prompt per session;
- concurrent prompt for same session returns a clear busy error.

Cross-session parallelism is allowed only if the writer and config/session state are safe. If not implemented, return a global busy error and document it.

## Close

`session/close`:

- if active, send cancel and wait or mark closing;
- remove session from in-process table;
- do not delete the persisted session file;
- respond `{}`.

## Tests

Needed tests:

- `session/new` persists cwd in header.
- prompt persistence appends one user and one assistant message without duplicate user prompt.
- unknown/path-traversal session IDs fail.
- load replay produces user/assistant updates before response if load is advertised.
- resume seeds history without replay if resume is advertised.
- capabilities match implemented load/resume behavior.
