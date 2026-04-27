---
id: '273'
title: Diagnose and harden Kimi Code OAuth model routing
slug: diagnose-and-harden-kimi-code-oauth-model-routing
status: open
priority: 2
created_at: '2026-04-27T04:38:38.847861Z'
updated_at: '2026-04-27T05:27:46.973563Z'
acceptance: A user who completes `imp login kimi` can select and use an OAuth-backed Kimi Code model without accidentally routing to Moonshot API-key auth. If `kimi` remains Moonshot-only, the UI/CLI must make that distinction actionable. Verification should include targeted auth/model routing tests and `cargo check -p imp-cli -p imp-llm -p imp-tui`.
notes: |-
  ---
  2026-04-27T04:53:38.370164+00:00
  Implemented first pass of option 1 auth-aware routing. In `crates/imp-core/src/imp_session.rs`, runtime model resolution now uses an `auth_preferred_oauth_route` helper: existing OpenAI→openai-codex fallback remains, and Kimi/Moonshot aliases now route to `kimi-code`/`kimi-for-coding` when Kimi Code OAuth exists and no Moonshot API key/API-key override is present. If Moonshot API key exists, `kimi` remains Moonshot (`kimi-k2.6`). In `crates/imp-tui/src/app.rs`, model picker now includes `kimi-for-coding` when Kimi Code OAuth exists and no Moonshot API key exists. Added targeted tests for both routing and picker cases. Verification passed: `cargo fmt --package imp-core --package imp-tui`; `cargo test -p imp-core resolve_runtime_connection -- --nocapture`; `cargo test -p imp-tui filtered_model_options -- --nocapture`; `cargo check -p imp-cli -p imp-llm -p imp-tui`. Still not proven: live Kimi Code HTTP endpoint compatibility (`https://api.kimi.com/coding/v1/chat/completions`) against real OAuth token.

  ---
  2026-04-27T04:56:38.817037+00:00
  User asked to live-test whether Kimi OAuth works. Next step: inspect available stored auth without exposing token values, then run a minimal Kimi Code/OAuth request through imp if credentials exist. If no usable Kimi OAuth credential exists locally, report exact command/user action needed to complete login.

  ---
  2026-04-27T04:59:48.569794+00:00
  Live Kimi OAuth test results: local `~/.imp/auth.json` contains `kimi-code` OAuth credentials and `~/.kimi/credentials/kimi-code.json` exists. Direct HTTP POST to `https://api.kimi.com/coding/v1/chat/completions` with Kimi Code bearer token + Kimi CLI headers returned HTTP 200 for both non-streaming and streaming requests using model `kimi-for-coding`. Non-streaming body for prompt `What is 2+2? Answer with the number only.` returned assistant `content:"4"` plus `reasoning_content`, usage 20/33/53. Streaming returned SSE chunks with `reasoning_content` first and HTTP 200. However, running through current `imp-cli` print mode with `--provider kimi-code --model kimi-for-coding` completes with zero token usage and no visible text; this likely means imp's streaming parser/agent path is not surfacing final non-empty content for Kimi Code, or max-token/forced reasoning behavior causes reasoning-only stream with no visible text in the print-mode output. Next implementation should add a Kimi Code provider/streaming regression using mocked SSE where reasoning-only chunks are followed by content, and inspect whether the live stream eventually emits content with sufficient max tokens and whether `MessageEnd` content is dropped from print mode when no TextDelta is emitted.

  ---
  2026-04-27T05:15:30.306208+00:00
  User clarified expected model naming should feel like `kimi2.6`/K2.6 rather than exposing `kimi-for-coding` as the primary name. Continue fixing the live bug: likely stream parser issue because Kimi Code SSE lines are `data:{...}` without a space, while imp's OpenAI-compatible parser only accepts `data: `. Also verify whether Kimi Code coding endpoint accepts Moonshot model IDs such as `kimi-k2.6`; if not, keep API wire model as `kimi-for-coding` but hide it behind user-facing aliases/labels.

  ---
  2026-04-27T05:18:21.261864+00:00
  Fixed live Kimi OAuth streaming bug. Root cause confirmed: Kimi Code SSE uses `data:{...}` without a space, while imp parsed only `data: `; therefore the stream had HTTP 200 but no parsed chunks, zero usage, and no text. Updated OpenAI-compatible SSE parsing to accept both `data: {...}` and `data:{...}`. Also renamed/aliased the OAuth-facing model to `kimi-k2.6-code`: Kimi Code endpoint accepts request model ids like `kimi-k2.6`/`kimi2.6` and returns model `kimi-for-coding`, so imp should not expose `kimi-for-coding` as the primary user-facing name. Added `kimi-k2.6-code` model metadata, aliases `kimi-code`, `kimi2.6`, and `kimi-k2.6-code`, and route fallback now chooses `kimi-k2.6-code`; wire request maps `kimi-k2.6-code` to API model id `kimi-k2.6`. Live verify now prints response through imp-cli with `--provider kimi-code --model kimi-k2.6-code`; output included answer `4` and token usage. Note: Kimi also streamed reasoning text, which imp currently prints to stderr in print mode, so visible output contains answer plus reasoning when stderr is merged; stdout answer is present. Verification passed: cargo test -p imp-llm openai_compat; cargo test -p imp-core resolve_runtime_connection; cargo test -p imp-tui filtered_model_options; cargo check -p imp-cli -p imp-llm -p imp-tui.

  ---
  2026-04-27T05:26:45.764730+00:00
  User asked to make the OAuth-facing Kimi model id simply `kimi2.6` instead of `kimi-k2.6-code`. Implement by renaming the user-facing `kimi-code` model metadata id to `kimi2.6`, mapping it to wire API model id `kimi-k2.6`, and updating auth fallback/picker/tests accordingly. Keep `kimi-for-coding` only as a legacy/backend alias/model if needed for compatibility, not the primary route.

  ---
  2026-04-27T05:27:46.973556+00:00
  Renamed OAuth-facing model from `kimi-k2.6-code` to `kimi2.6` per user request. Updated metadata, aliases, auth-aware fallback, picker, and wire mapping (`kimi2.6` sends API model id `kimi-k2.6`). Targeted `imp-llm` model/openai_compat tests passed after rename. Broader check/run is currently blocked by unrelated pre-existing dirty TUI/TypeScript extension compile errors in `crates/imp-tui/src/app.rs`, `crates/imp-tui/src/views/settings.rs`, and `crates/imp-core/src/typescript_extensions.rs`; these are outside Kimi files and were present/introduced by unrelated work. Earlier live run before rename proved Kimi OAuth streaming bug fixed; after rename, compile of imp-cli is blocked by unrelated TUI errors because imp-cli depends on imp-tui.
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
