---
id: '282'
title: Design native scoped secret injection for command tools
slug: design-native-scoped-secret-injection-for-command
status: open
priority: 2
created_at: '2026-04-27T20:04:18.688197Z'
updated_at: '2026-04-27T20:04:18.688197Z'
acceptance: Design covers API, policy, redaction, and migration path for replacing Lua secret-backed command wrappers with native scoped secret injection.
labels:
- imp
- secrets
- runtime
- design
kind: job
---

Goal: Design and implement a native, auditable way for imp tools/agents to inject stored secrets into command environments without exposing raw secret values to the agent or broad Lua runtime.

Current state: `openrouter_secret_run` works via Lua after enabling `allow_shell_exec` and `allow_secrets`, and `imp.exec` now supports `opts.env`. This is functional but broad: a Lua tool with both capabilities can read secrets and execute arbitrary shell commands. We want a narrower native boundary.

Proposed direction to evaluate:
1. Add a native command execution interface that accepts secret bindings like `{ env: "OPENROUTER_API_KEY", provider: "openrouter", field: "api_key" }`.
2. The runtime resolves the secret internally from `AuthStore` and injects it into the child process environment.
3. The tool call/result never includes the secret value; logs/details should include only provider, field name, and env var name.
4. Output redaction should mask any injected secret if the child command accidentally prints it.
5. Policy should gate which providers/fields can be injected and whether arbitrary command execution is allowed.

Scope boundaries:
- Do not print, serialize, or expose raw secret values to the model.
- Do not require global `allow_secrets=true` for this path.
- Keep existing Lua secret APIs as legacy/advanced capability, not the preferred route.

Acceptance: A short design exists with API shape, policy, redaction behavior, and migration path for `openrouter_secret_run`; if straightforward, implementation units are created with verify gates.
