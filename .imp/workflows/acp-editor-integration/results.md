# ACP editor integration results

Status: scaffold implemented, verified with concerns.

## Implemented

- Added `imp acp` CLI subcommand in `crates/imp-cli/src/lib.rs`.
- Added initial ACP adapter module under `crates/imp-cli/src/acp/`:
  - JSON-RPC/ACP protocol subset types and parsing/serialization.
  - stdio server loop with newline-delimited JSON-RPC.
  - `initialize` handshake for protocol version 1.
  - `session/new` with absolute cwd validation and durable imp `SessionManager` creation.
  - safe session-id-to-session-file lookup helper.
  - scaffolded `session/prompt` completion response shape.
  - scaffolded `session/cancel` state handling.
  - initial `AgentEvent`/message/content-to-ACP `session/update` mapping helpers.
- Added `docs/acp.md` and linked it from `README.md` and `docs/index.md`.
- Recorded smoke-demo evidence in `.imp/workflows/acp-editor-integration/artifacts/smoke-client-demo.md`.

## Evidence inspected

Planning/context artifacts were written from inspected ACP docs and imp source:

- `.imp/workflows/acp-editor-integration/artifacts/context.md`
- `.imp/workflows/acp-editor-integration/artifacts/acp-contract.md`
- `.imp/workflows/acp-editor-integration/artifacts/session-mapping.md`
- `.imp/workflows/acp-editor-integration/artifacts/implementation-plan.md`
- `.imp/workflows/acp-editor-integration/artifacts/cli-surface-decision.md`

## Verification

Passed:

```sh
cargo fmt --check
cargo check -p imp-cli
cargo test -p imp-cli acp -- --nocapture
```

Smoke command passed and returned valid ACP JSON-RPC initialize/session responses:

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{},"clientInfo":{"name":"smoke"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/Users/asher/imp","mcpServers":[]}}' \
  | cargo run -q -p imp-cli -- acp
```

Observed response shape:

- `protocolVersion: 1`
- `agentInfo.name: "imp"`
- durable UUID `sessionId`

## Concerns / remaining work

This is not yet a complete usable daily editor integration.

Important remaining work:

- Wire `session/prompt` to real `Agent::run` execution.
- Stream provider text/tool progress to the ACP client during active turns.
- Implement `session/request_permission` for imp UI/tool approvals.
- Map policy denials to high-quality ACP tool/status updates.
- Implement real cancellation of active live turns, not just scaffold cancellation state.
- Implement and advertise `session/load`/`session/resume` only after replay/resume semantics are complete.
- Reduce or intentionally allow dead-code warnings as scaffold helpers become live-wired.

Docs explicitly mark the current adapter as scaffold-level and list unsupported behavior.
