# ACP live editor integration — blocked for future runtime work

## Status

`imp acp` should remain a truthful scaffold/stub for now.

The workflow is intentionally **blocked**, not abandoned. The remaining acceptance criteria describe the future live ACP editor adapter, but they depend on deeper imp runtime/host seams rather than ACP-specific shims.

## What is safe to ship now

The ACP scaffold may remain useful for protocol exploration and future editor-client testing if it stays honest about its behavior:

- JSON-RPC stdio server entrypoint exists.
- `initialize` returns conservative capabilities.
- `session/new` creates a durable imp session file.
- prompt content block parsing exists for text/resources.
- scaffold prompt handling is visibly marked as stubbed/scaffold behavior.
- event/protocol mapping helpers and tests preserve the target shapes for future implementation.

Do **not** advertise live prompt execution, live permission approval, or active cancellation until they work end to end.

## Why this is blocked instead of done

The remaining work belongs in shared imp runtime architecture:

1. host-neutral agent construction shared by RPC/headless/ACP;
2. active run/prompt lifecycle with structured state;
3. host `UserInterface` bridge for approvals and selections;
4. JSON-RPC client response routing for host-mediated permission requests;
5. canonical active cancellation through `AgentCommand`/cancel token;
6. consistent policy-denial event visibility;
7. smoke client and docs that validate/describe actual shipped capabilities.

Trying to finish ACP by shimming around these seams would duplicate runtime behavior and risk weakening policy/cancellation/session semantics.

## Resume criteria

Resume this workflow when at least one of these is true:

- imp has a host-neutral agent construction helper extracted from RPC/headless code;
- imp has a reusable active-run controller that exposes event stream, cancellation, and final status;
- `UserInterface` approval flows have a host adapter contract suitable for non-TUI clients;
- there is explicit product priority to build ACP as a live editor adapter.

## Collaboration and prototype requirement

This workflow is not designed to be completed autonomously from the blocked state. Future live ACP work must proceed through collaborative prototype review before production implementation.

Required before resuming implementation:

- choose which shared runtime seam to prototype first with the project maintainer;
- record the chosen prototype and rejection criteria in `prototype-decision.md`;
- review the prototype evidence with the maintainer;
- get explicit approval before promoting prototype behavior into the production ACP adapter.

The workflow now includes proposed prototypes for host-neutral agent construction, active-run control, ACP permission bridging, and protocol smoke testing. These are decision tools, not automatic build steps.

## Future execution order

When resumed collaboratively, implement in this order:

1. Extract/shared host-neutral agent construction.
2. Introduce ACP active prompt state and event forwarding.
3. Add ACP UI permission bridge and response routing.
4. Wire active cancellation.
5. Persist completed live turns without duplicates.
6. Add protocol smoke client.
7. Update docs and initialize capabilities.
8. Run final verification and close workflow.

## Current closeout decision

Keep the workflow and artifacts as the future implementation contract. Do not mark live-adapter acceptance criteria complete while ACP remains a stub.
