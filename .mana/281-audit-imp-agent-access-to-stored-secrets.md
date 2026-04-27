---
id: '281'
title: Audit imp agent access to stored secrets
slug: audit-imp-agent-access-to-stored-secrets
status: closed
priority: 2
created_at: '2026-04-27T19:45:22.731073Z'
updated_at: '2026-04-27T19:51:58.515685Z'
acceptance: A clear diagnosis exists for direct vs worker secret access, with exact code references, and either a minimal verified fix or follow-up implementation units. No actual secret values are exposed in logs, files, or chat.
notes: |-
  ---
  2026-04-27T19:47:26.915992+00:00
  Initial diagnosis from local code inspection:
  - Provider/model auth for normal sessions likely works in both direct and spawned workers: `ImpSession::create` loads `AuthStore` via `options.auth_path.or_else(storage::existing_global_auth_path).unwrap_or_else(storage::global_auth_path)`, and both `spawn` paths create `ImpSession`s with auth_path None.
  - The problematic path is Lua extension tools such as `openrouter_secret_run`: the Lua tool calls `imp.secret_fields("openrouter")` and then `imp.exec(..., { env = { OPENROUTER_API_KEY = key } })`.
  - `LuaCapabilityPolicy::default()` sets `allow_shell_exec=false` and `allow_secrets=false`; user configs inspected this session have `[lua]` empty or absent, so `imp.secret_fields()` and `imp.exec()` are disabled by default.
  - Even if those capabilities are enabled, `imp.exec` in `crates/imp-lua/src/bridge.rs` currently only consumes `opts.cwd`; it ignores `opts.env`, so `OPENROUTER_API_KEY` from `openrouter_secret_run.lua` is never injected.
  - `imp.secret`/`imp.secret_fields` hardcode `Config::user_config_dir().join("auth.json")` instead of using `storage::existing_global_auth_path`, which can miss legacy `~/.config/imp/auth.json` metadata in edge cases.
  - No secret values were read or printed.

  ---
  2026-04-27T19:49:50.390553+00:00
  Implementing the localized fix now: add scoped env support to Lua `imp.exec`, make Lua secret lookup use canonical/legacy-aware auth path resolution, and add targeted tests. Will not print or inspect secret values.

  ---
  2026-04-27T19:51:58.515681+00:00
  Implemented localized fix:
  - `crates/imp-lua/src/bridge.rs`: `imp.exec` now applies string key/value pairs from `opts.env` via `Command::env`, enabling scoped child-process secret injection for tools like `openrouter_secret_run`.
  - `crates/imp-lua/src/bridge.rs`: `imp.secret` and `imp.secret_fields` now load auth metadata via `storage::existing_global_auth_path().unwrap_or_else(storage::global_auth_path)` instead of hardcoded `Config::user_config_dir().join("auth.json")`.
  - `crates/imp-lua/src/lib.rs`: added regression test proving scoped env reaches the child process command path.
  Verified with `cargo test -p imp-lua imp_exec_ -- --nocapture`, `cargo test -p imp-lua imp_secret_ -- --nocapture`, and `cargo fmt --check`.
labels:
- imp
- secrets
- runtime
verify: rg -n "secret|OPENROUTER_API_KEY|openrouter_secret_run|spawn|worker" . && cargo test -q secret
kind: task
---

Goal: Determine why imp agents/workers may have trouble using secrets stored inside imp, and produce a concrete fix plan.

Current concern: Agents invoked through imp may not be receiving or resolving stored secrets reliably, especially in spawned/worker contexts. This affects tools like benchmark/openrouter-secret-backed execution where secrets should be exposed only to the intended command/tool and never printed.

Steps:
1. Inspect the current secret storage and lookup code paths in imp, including config loading, secret redaction, tool invocation, spawned worker/session construction, and environment propagation.
2. Compare direct user-session tool execution against spawned agent/worker execution. Identify whether workers inherit the secret store path/config, API key accessors, or only a scrubbed runtime context.
3. Check existing secret-aware tools, especially `openrouter_secret_run`, for assumptions about where secrets live and how they are injected.
4. Produce findings with exact files/functions and a minimal implementation plan. If the issue is confirmed and localized, implement the smallest safe fix; otherwise create follow-up units.

Files likely relevant:
- imp CLI/runtime session construction code (inspect)
- tool registration/invocation code (inspect)
- config/secret-loading modules (inspect)
- spawn/worker boundary code (inspect)

In scope:
- Secret lookup availability in direct sessions vs spawned agents/workers.
- Safe one-command secret injection paths.
- Redaction boundaries and prevention of secret leakage in logs/tool output.

Out of scope:
- Adding a new external secret manager.
- Printing, exporting, or committing actual secret values.
- Broad architecture rewrite unless required by evidence.

Verify gate: Run the narrowest relevant tests or a safe smoke command that proves a worker can access an imp-stored secret through an intended secret-aware path without exposing the value; if no safe smoke exists, add or document one.
