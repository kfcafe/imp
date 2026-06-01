# ACP deep integration blockers

The remaining ACP work should not be treated as a thin shim over the current scaffold. The blockers require host/runtime integration seams that are shared with RPC/headless execution and core tool approval behavior.

## Why the current scaffold should stay a truthful stub

`imp acp` currently validates basic JSON-RPC/ACP shapes, creates durable imp session files, maps ACP content blocks into text, emits scaffold session updates, and has tests for protocol/event mapping helpers. That is useful as a compatibility scaffold, but it is not yet a live editor adapter.

Until the runtime seams below are implemented, ACP should avoid advertising behavior it cannot provide. In particular, prompts that do not run a real agent should remain visibly marked as scaffold/stub behavior, and load/resume/permission/cancel capabilities should only be advertised when the server actually supports them end to end.

## Required deeper integration work

### 1. Host-neutral agent construction

ACP needs the same runtime setup as JSONL RPC/headless mode:

- config resolution for the session cwd;
- model/provider/auth resolution;
- runtime API key override behavior where applicable;
- Lua extension tool loading;
- system prompt overrides and role/config handling;
- `UserInterface` injection.

This should be extracted from the existing RPC path into a reusable host-neutral helper instead of duplicating provider/auth/tool setup inside `crates/imp-cli/src/acp`.

### 2. Async active prompt lifecycle

ACP `session/prompt` needs to become an active turn state machine:

- reject concurrent prompts for the same session with a clear busy error;
- build an agent from persisted session history;
- spawn/drive `Agent::run(prompt)`;
- forward `AgentHandle.event_rx` into ACP `session/update` notifications;
- persist new user/assistant/tool messages after completion without duplicating the prompt;
- complete the original `session/prompt` response only when the turn ends.

The current synchronous request handler is enough for the scaffold, but active cancellation and streaming require explicit active prompt state.

### 3. ACP `UserInterface` bridge

Tool approvals and policy-sensitive confirmations flow through imp's `UserInterface` abstraction. ACP needs an `AcpUi` implementation that can:

- map approval-like `confirm` / `select_with_context` calls to `session/request_permission`;
- allocate JSON-RPC request ids;
- route client responses back to pending UI requests;
- handle allow, reject, timeout, and cancellation outcomes;
- emit visible ACP updates for unsupported generic input/custom UI requests.

This must preserve imp policy semantics; ACP should display and route decisions, not bypass approval checks.

### 4. Client response routing

The stdio server loop currently ignores inbound JSON-RPC responses. Live permissions require a pending request table keyed by JSON-RPC id, plus response delivery to the `AcpUi` request waiters. This is also where cancellation should settle pending permission requests.

### 5. Active cancellation

`session/cancel` currently only marks a flag that affects the next prompt. Live cancellation requires:

- storing the active prompt's `AgentCommand` sender and/or cancel token;
- sending `AgentCommand::Cancel` when `session/cancel` arrives;
- cancelling pending ACP UI permission requests;
- allowing late runtime/tool events to drain safely;
- returning `stopReason: cancelled` for the pending `session/prompt` response.

### 6. Policy denial visibility

Core policy remains enforced by imp. ACP needs event mapping that turns policy/tool denial outcomes into visible `session/update` messages, ideally tied to the relevant tool call when possible. If the denial is not tied to a tool call, it should still be shown as an agent/status update.

### 7. Smoke client and capability truthfulness

After the above is implemented, add a small local protocol smoke client that exercises:

- `initialize`;
- `session/new`;
- live `session/prompt` streaming;
- one permission or cancellation path;
- `session/close`.

Docs and initialize capabilities must match the behavior that actually ships.

## Workflow implication

The current workflow should treat the scaffold as intentionally stubbed until these deeper runtime seams are built. The remaining live-adapter acceptance criteria should stay in the workflow as blocked/future build work, not be marked complete by shimming scaffold responses.
