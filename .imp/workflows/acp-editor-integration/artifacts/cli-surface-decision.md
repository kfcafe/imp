# ACP CLI surface and module boundary decision

## Decision

Add a first-class ACP subcommand:

```sh
imp acp [global imp options]
```

Examples:

```sh
imp --provider anthropic --model claude-sonnet-4 acp
imp --thinking medium acp
imp --system-prompt "Use concise explanations" acp
```

The subcommand starts a local ACP stdio server. It reads newline-delimited JSON-RPC messages from stdin and writes newline-delimited ACP JSON-RPC messages to stdout.

## Why a subcommand instead of `--mode acp`

- `--mode rpc` is already an imp-native JSONL worker protocol with simple `{type: ...}` commands. ACP is JSON-RPC with a session lifecycle, request/response IDs, server-to-client methods, and different stdout constraints.
- A subcommand is easier for editors to configure and users to discover: the launch command is simply `imp acp`.
- ACP-specific help text can live on the command without making `imp --help` mode semantics more confusing.
- Future ACP-only options can be added locally without impacting TUI/print/RPC behavior.

## Global options preserved

The ACP command should reuse existing top-level options where meaningful:

- `--provider`
- `--model`
- `--thinking`
- `--api-key`
- `--system-prompt`
- `--autonomy`
- `--allow-tool` / `--deny-tool`
- `--allow-write` / `--deny-write`
- `--no-tools`
- `--max-turns`
- `--max-tokens`
- `--verbose` only if it logs to stderr, never stdout

Session-selection flags such as `--continue`, `--resume`, `--session`, and `--no-session` should not drive ACP sessions directly in the first version; ACP has its own `session/new`, `session/load`, and `session/resume` lifecycle. If accepted by clap globally, document that ACP session methods win.

## Module boundary

Initial implementation should live in `imp-cli`, not a new crate:

```text
crates/imp-cli/src/acp/
  mod.rs        server loop, dispatch, connection state
  protocol.rs   JSON-RPC and ACP subset serde types
  codec.rs      newline-delimited stdio reader/writer
  events.rs     AgentEvent/UserInterface/tool mapping
  session.rs    ACP session table and SessionManager bridge
```

Justification:

- ACP is currently a process/CLI adapter concern.
- `imp-core` already exposes the runtime primitives needed: `Agent`, `AgentHandle`, `AgentCommand`, `AgentEvent`, `UserInterface`, and `SessionManager`.
- Keeping protocol code in `imp-cli` avoids making editor wire formats part of the core runtime API before the shape is proven.
- The boundary is still easy to extract later into `imp-core` or a dedicated `imp-acp` crate if another host needs in-process ACP.

## Reuse plan

Reuse, do not duplicate:

- Agent construction logic from `create_rpc_agent`, likely by extracting a host-neutral helper that accepts `cwd`, `Config`, history, and `Arc<dyn UserInterface>`.
- Event streaming structure from `forward_rpc_events`, but with an ACP-specific mapper instead of legacy RPC JSON.
- `RpcUi` design pattern for pending UI requests, but implement a separate `AcpUi` because ACP uses outbound JSON-RPC `session/request_permission` and client responses, not ad hoc `ui_request` objects.
- `SessionManager` durable JSONL storage for ACP session IDs and history.

Do not reuse:

- `parse_rpc_command` / `RpcInputCommand`, because ACP dispatch is JSON-RPC method-based.
- legacy RPC stdout messages, because ACP stdout must be valid ACP JSON-RPC only.
- in-memory-only `run_rpc_mode` history as the ACP source of truth.

## Review notes

This boundary keeps first implementation focused and reversible. The largest risk is that durable session persistence and agent construction are currently split across TUI/RPC paths; extraction should be small and behavior-preserving. If the official Rust ACP SDK is adopted later, it should replace or wrap `protocol.rs`/`codec.rs` without changing imp-core runtime behavior.
