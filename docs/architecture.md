# Architecture

imp is a Rust workspace split by runtime responsibility.

## Crates

```text
imp-cli   CLI entrypoint, TUI launch, one-shot/headless/RPC modes
imp-core  agent loop, tools, sessions, workflows, policy, SDK
imp-llm   providers, streaming, auth, model metadata
imp-lua   Lua extension runtime
imp-tui   terminal UI
```

## Runtime flow

A typical run resolves configuration, auth, model selection, session state, tool registry, policy, context, and hooks before starting the agent loop.

The agent loop streams model output, executes structured tool calls, records observations, applies policy checks, and emits runtime/session events.

## Surfaces

Primary user/integration surfaces:

- TUI
- one-shot prompt mode
- JSONL RPC mode
- preview Rust SDK

CLI chat exists as compatibility code, but it is not the primary documented interactive surface.

## Tool registry

Native tools are registered in `imp-core`. Lua extensions can add tools through the extension bridge. Role and runtime policy can filter or deny tools before or during execution.

## Sessions

Sessions are durable JSONL records. They are used by the TUI, one-shot/headless runs, viewers, usage reports, evidence commands, and recovery flows.

## Workflows

Workflow schema and validation live in `imp-core/src/workflow`. The model-facing workflow tool lives in `imp-core/src/tools/workflow.rs`. Workflows are currently local and file-backed.

## Providers

Provider integration lives under `imp-llm`. The runtime treats providers through shared model/streaming abstractions while preserving provider-specific auth and model metadata.

## Extensions

`imp-lua` loads Lua files, sets up the host API, registers commands/tools/hooks, and applies capability policy.

## Planned surfaces

- workflow API
- MCP
- `.imp/agents`
- ACP/editor adapters
- hosted sync/team collaboration
