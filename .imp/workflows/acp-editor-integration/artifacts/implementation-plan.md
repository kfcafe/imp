# ACP implementation plan

## Architectural decision

Implement ACP as a sibling adapter to the existing JSONL RPC mode in `imp-cli`, with small reusable helpers where the existing RPC code can be shared safely.

Recommended module split:

- `crates/imp-cli/src/acp/mod.rs` — server loop and session table.
- `crates/imp-cli/src/acp/protocol.rs` — JSON-RPC envelope and ACP subset types.
- `crates/imp-cli/src/acp/codec.rs` — newline-delimited stdio reader/writer and response correlation.
- `crates/imp-cli/src/acp/events.rs` — `AgentEvent`/tool/UI to ACP mapping.
- `crates/imp-cli/src/acp/session.rs` — ACP session state and imp `SessionManager` mapping.
- Optionally extract shared agent construction from current `create_rpc_agent` into a function that accepts cwd, config, history, and UI.

Keep `imp-core` changes minimal. Add imp-core helpers only if needed for clean session lookup/persistence or event classification.

## Dependency choice

Default plan: start with local typed protocol types using `serde` and `serde_json`, because imp already uses JSONL/stdout plumbing and the first ACP subset is small.

Revisit using the official `agent-client-protocol` Rust crate if:

- local schema drift becomes a risk,
- Zed compatibility tests reveal subtle shape mismatches,
- the crate provides a lightweight server abstraction without pulling large dependencies or forcing incompatible runtime choices.

Adding the SDK is approval-worthy if it is a significant new dependency.

## Phase 1 — protocol types and tests

Files likely touched:

- `crates/imp-cli/src/acp/protocol.rs`
- `crates/imp-cli/src/acp/codec.rs`
- `crates/imp-cli/src/acp/mod.rs`
- `crates/imp-cli/src/lib.rs` for module/command wiring later

Implement:

- JSON-RPC request/response/notification envelopes.
- ID support for number/string/null where JSON-RPC allows.
- Parse dispatch for `initialize`, `session/new`, `session/load`, `session/resume`, `session/prompt`, `session/cancel`, `session/close`.
- Response helpers for result/error.
- ACP content block subset: text, resource, resource_link, unknown.
- ACP update subset: agent/user message chunks, tool_call, tool_call_update.
- Permission request/result subset.

Tests:

- Parse representative docs examples.
- Serialize initialize response.
- Reject malformed JSON and invalid params with correct JSON-RPC errors.
- Unknown method returns method-not-found.

## Phase 2 — CLI entrypoint and stdio server

Files likely touched:

- `crates/imp-cli/src/lib.rs`
- `crates/imp-cli/src/acp/mod.rs`
- README/docs later

Implement:

- Add `Commands::Acp` or equivalent `imp acp` subcommand.
- Ensure no startup logs/timing are written to stdout in ACP mode.
- Create server state with client capabilities, session table, stdout writer, pending outbound requests.
- Implement initialize handshake.
- Implement method ordering checks: methods before initialize get a useful error.

Verification:

- Tiny shell/Python smoke sends initialize and checks one JSON response.
- Unit/integration test for quiet stdout.

## Phase 3 — session creation and durable mapping

Files likely touched:

- `crates/imp-cli/src/acp/session.rs`
- maybe `crates/imp-core/src/session.rs` for lookup helper

Implement:

- Validate absolute cwd.
- Resolve config per ACP cwd.
- Create `SessionManager::new` and return imp session UUID.
- Load/resume by session ID if implemented and advertised.
- Seed in-memory history from active branch messages.

Tests:

- `session/new` returns stable ID and creates session file.
- Relative cwd rejected.
- Load/resume unknown session gives useful error.
- If advertised, load replays history before response.

## Phase 4 — agent turn wiring

Files likely touched:

- `crates/imp-cli/src/acp/mod.rs`
- `crates/imp-cli/src/acp/events.rs`
- shared extraction near current `create_rpc_agent` / `spawn_rpc_agent`

Implement:

- Convert ACP prompt content to imp prompt text.
- Build agent using existing builder path with ACP UI implementation.
- Spawn `agent.run(prompt)` and forward events.
- Hold prompt request open until run completes.
- Return stop reason and optional `_meta.imp` usage/cost/status.
- Persist new messages into `SessionManager` after completion.

Tests:

- With fake/mock provider if existing tests support it, verify prompt stream order.
- Event mapper unit tests using synthetic `AgentEvent`s.
- Completion maps success/failure/cancel correctly.

## Phase 5 — permissions, policy, cancellation, follow-up

Files likely touched:

- `crates/imp-cli/src/acp/events.rs`
- `crates/imp-cli/src/acp/mod.rs`
- `crates/imp-cli/src/acp/session.rs`

Implement:

- `AcpUi` implementing `UserInterface`.
- Map confirm/select approval-like requests to `session/request_permission` outbound JSON-RPC requests.
- Track pending outbound request IDs and route client responses.
- Map cancelled permission outcomes to `None`.
- Implement `session/cancel` -> `AgentCommand::Cancel`/cancel token.
- Ensure cancelled prompt response is `stopReason: cancelled` rather than raw task error.
- Allow next `session/prompt` after completion as normal continuation.

Tests:

- UI confirm approval and denial.
- Policy denial event is visible/mapped.
- Cancel active prompt and then follow-up prompt works.
- Pending permission request cancellation does not hang.

## Phase 6 — polish, docs, smoke client

Files likely touched:

- `docs/acp.md`
- `README.md`
- `.imp/workflows/acp-editor-integration/artifacts/smoke-client-demo.md`
- optional `scripts/acp-smoke-client.py` or test fixture

Implement/docs:

- Launch command.
- Supported capabilities and limitations.
- Zed/JetBrains/manual client configuration examples.
- Troubleshooting for auth, model/provider, workspace cwd, no stdout logs, and unsupported MCP/content.
- Smoke client that exercises initialize, session/new, prompt, cancellation or permission path, close.

## Verification commands

Minimum final checks:

```sh
cargo fmt --check
cargo check -p imp-cli
cargo test -p imp-cli acp
cargo test -p imp-core session   # if session helpers changed
python3 scripts/acp-smoke-client.py --imp target/debug/imp --cwd "$PWD"
```

Adjust commands to actual test names and whether a script is checked in.

## Main risks

- Persisting exact agent message history without duplicating user prompts requires care because `Agent::run(prompt)` appends the user prompt internally.
- Current RPC event mapping is imp-native; ACP clients need less internal noise and better tool-call lifecycle semantics.
- ACP permission is specific to tool authorization; imp `UserInterface` also supports generic input/custom components that may not map cleanly.
- Missing auth/model setup inside an editor can be confusing unless errors are very explicit.
- MCP server params are part of ACP session setup; unsupported handling must be protocol/client friendly.
- Concurrent sessions may expose assumptions in the current RPC single-history loop.


## Chosen integration boundary update

After inspecting the existing clap surface, use an explicit `imp acp` subcommand rather than overloading `--mode rpc`.

Rationale:

- ACP is a public editor protocol with its own lifecycle; it should be discoverable as a first-class command.
- Existing `--mode rpc` remains imp-native JSONL and should not gain incompatible JSON-RPC behavior.
- Existing global provider/model/thinking/system-prompt/api-key/autonomy/tool policy flags can still apply because clap global options live on `Cli`.
- A subcommand leaves room for ACP-specific flags later (`--log-file`, `--strict-client`, `--no-load-session`) without confusing print/TUI modes.

Boundary:

- Keep the adapter in `imp-cli` initially under `src/acp/`.
- Extract a shared `create_agent_for_host` helper only if doing so reduces duplication with `create_rpc_agent`; do not move ACP protocol types into `imp-core` until another crate needs to host ACP.
- Add small `imp-core` session helpers only for durable lookup/replay if required.
