---
id: '273'
title: Diagnose and harden Kimi Code OAuth model routing
slug: diagnose-and-harden-kimi-code-oauth-model-routing
status: open
priority: 2
created_at: '2026-04-27T04:38:38.847861Z'
updated_at: '2026-04-27T04:47:35.130122Z'
acceptance: A user who completes `imp login kimi` can select and use an OAuth-backed Kimi Code model without accidentally routing to Moonshot API-key auth. If `kimi` remains Moonshot-only, the UI/CLI must make that distinction actionable. Verification should include targeted auth/model routing tests and `cargo check -p imp-cli -p imp-llm -p imp-tui`.
design: |-
  Durable decomposition for Kimi OAuth support:

  Decision: separate Moonshot/Kimi API-key support from Kimi Code OAuth support. Moonshot/Kimi public API models (`kimi`, `kimi-k2.6`, `kimi-thinking`, etc.) route through provider `moonshot` and use API keys. Kimi OAuth should route through provider `kimi-code` and its OAuth-backed model(s), not silently reuse Moonshot aliases unless routing logic intentionally chooses the OAuth route.

  Recommended UX direction: prefer ergonomic OAuth fallback for `kimi` only when it is safe and explicit: if the user has `kimi-code` OAuth credentials and lacks a Moonshot/Kimi API key, selecting/typing `kimi` may resolve to the Kimi Code OAuth model. If both auth paths exist, do not guess; preserve the configured canonical model/provider and label the choices clearly in the picker. At minimum, make `kimi-for-coding` visible and selectable after Kimi OAuth login.

  Decomposition:
  1. Diagnose actual Kimi Code HTTP contract.
     - Confirm whether `https://api.kimi.com/coding/v1/chat/completions` with OpenAI-compatible body and Kimi Code headers is accepted.
     - If not, compare against Kimi CLI traffic/docs and implement a provider-specific URL/body/header shim in `crates/imp-llm/src/providers/openai_compat.rs` or a dedicated provider only if the shim becomes ugly.
  2. Fix model/auth routing.
     - Ensure `imp login kimi` resulting credentials under `kimi-code` make at least one model visible/selectable in TUI and usable from CLI/session startup.
     - Decide whether `kimi` alias fallback to `kimi-for-coding` is implemented centrally in model/session resolution or only in picker UX.
  3. Harden login/import path.
     - Validate `try_import_kimi_cli_credentials` shape against real `~/.kimi/credentials/kimi-code.json`.
     - Preserve Kimi CLI `device_id` coupling when imported tokens require matching device headers.
     - Add actionable error messages when OAuth exists but the selected model routes to Moonshot API-key provider.
  4. Tests/verification.
     - Add tests that `kimi-code` OAuth credentials satisfy auth for `kimi-for-coding`.
     - Add tests that the model picker includes an OAuth-backed Kimi Code model when `kimi-code` OAuth exists.
     - Add a regression test for the confusing case: `imp login kimi` + selected `kimi` must either resolve to a usable OAuth route or emit a clear Moonshot API-key requirement.

  Scope boundaries:
  - Do not conflate Kimi Code OAuth with Moonshot/Kimi platform API keys.
  - Do not stage unrelated dirty YouTube/web-tool files.
  - Avoid broad provider refactors unless endpoint incompatibility proves generic OpenAI-compatible routing cannot support Kimi Code.
labels:
- kimi
- oauth
- bug
kind: epic
decisions:
- 'Adopt auth-aware model routing for Kimi and, by principle, all providers with both OAuth and API-key paths: when the user has OAuth credentials and lacks the corresponding API key, model aliases/common names should resolve to the OAuth-backed provider/model instead of failing on the API-key route. If both OAuth and API-key credentials exist, do not silently guess; preserve explicit configured provider/model and label picker choices clearly. This should become the general interaction model for OAuth vs API keys, not a Kimi-only special case.'
---

Investigate why Kimi Code OAuth login/use does not work reliably in imp. Current observed code state: imp has a Kimi Code OAuth device-flow implementation in crates/imp-llm/src/oauth/kimi_code.rs; CLI/TUI login can store credentials under provider id `kimi-code`; provider registry has `kimi-code` with base URL `https://api.kimi.com/coding`; OpenAI-compatible provider always posts to `{base_url}/v1/chat/completions`; model registry has a separate OAuth-backed model `kimi-for-coding` on provider `kimi-code`, while aliases like `kimi` and `kimi-k2.6` resolve to provider `moonshot` and require Moonshot/Kimi API keys. Likely failure modes: users run `imp login kimi` but then select alias/model `kimi`, which routes to `moonshot` instead of `kimi-code`; or the Kimi Code API path/header/body differs from the generic OpenAI-compatible path. Steps: reproduce auth/model routing after `imp login kimi`; verify whether selecting `kimi-for-coding` uses `kimi-code` and calls the correct URL; inspect/compare Kimi CLI HTTP endpoint if needed; add route-aware alias/picker behavior or clearer UX so OAuth login exposes a usable Kimi Code model; add tests for login-provider/model-provider mismatch and OAuth token refresh. Avoid touching unrelated YouTube/web-tool dirty files.
