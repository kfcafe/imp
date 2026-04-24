---
id: '269'
title: Add ChatGPT 5.5 / GPT-5.5 model support
slug: add-chatgpt-55-gpt-55-model-support
status: open
priority: 2
created_at: '2026-04-24T00:32:42.803762Z'
updated_at: '2026-04-24T03:03:02.981652Z'
acceptance: Users can select GPT-5.5 / ChatGPT 5.5 model IDs through the built-in model registry and alias resolution, and focused tests covering alias/meta resolution pass.
notes: |-
  ---
  2026-04-24T02:02:10.881373+00:00
  Implemented first-class OAuth-path model support for GPT-5.5 in crates/imp-llm/src/model.rs without changing plain OpenAI API builtins. Added a ChatGPT/Codex-only built-in model entry in builtin_openai_codex_models(), plus aliases gpt5.5 / gpt-5.5 / chatgpt5.5 / chatgpt-5.5 -> gpt-5.5. This preserves the existing behavior where provider-less resolution still treats gpt-5.5 as an OpenAI-style custom model, but the ChatGPT/Codex route now explicitly recognizes it for OAuth fallback and picker/runtime metadata. Verified with: cargo test -p imp-llm model::tests ; cargo test -p imp-core imp_session::tests::resolve_runtime_connection_prefers_openai_chatgpt_route_when_oauth_exists ; cargo test -p imp-cli resolve_model_prefers_chatgpt_provider_when_only_oauth_is_available

  ---
  2026-04-24T02:40:25.912679+00:00
  Operator rollout / usage note: to start using GPT-5.5 on the ChatGPT/Codex OAuth path after this change, rebuild the installed imp CLI from source (`cargo install --path crates/imp-cli --force` from imp/), then verify with `imp --version`, refresh OAuth if needed via `imp login openai`, and run `imp --model gpt-5.5`. A full local-state wipe is not required for model support; only recommend backing up/removing `~/.config/imp/` when the user explicitly wants a clean reinstall of auth/config/session state.

  ---
  2026-04-24T02:58:10.564955+00:00
  New issue found after install verification: GPT-5.5 does not appear in the TUI model picker even though OAuth-path runtime support works. Root cause from inspected code: the picker uses App::filtered_models(), which draws from model_registry.list(); the registry intentionally does not include gpt-5.5 as a built-in model to avoid advertising unsupported plain-API details, so the picker never sees the ChatGPT/Codex-only model. Plan: patch TUI picker filtering to append ChatGPT/Codex-only built-in models that are not present in the normal registry when ChatGPT/OpenAI OAuth credentials exist, and add a focused test.

  ---
  2026-04-24T03:03:02.981638+00:00
  Fixed the TUI picker gap in crates/imp-tui/src/app.rs. Added shared filtered_model_options() logic plus a helper that appends ChatGPT/Codex-only built-in models (currently including gpt-5.5) when ChatGPT/OpenAI OAuth exists and no plain OpenAI API key is configured. This keeps the picker/settings aligned with the same OAuth-fallback intent used by runtime selection, while avoiding advertising the OAuth-only model when the normal OpenAI API path is active. Added focused tests covering both inclusion and hiding behavior, plus kept the existing cycle-model integration check passing. Verified with: cargo test -p imp-tui filtered_model_options_includes_chatgpt_oauth_only_models -- --nocapture ; cargo test -p imp-tui filtered_model_options_hides_chatgpt_oauth_only_models_when_openai_api_key_exists -- --nocapture ; cargo test -p imp-tui tui_integration_model_switch_via_cycle -- --nocapture ; cargo test -p imp-cli resolve_model_prefers_chatgpt_provider_when_only_oauth_is_available -- --nocapture
verify: cargo test -p imp-llm model::tests
kind: job
paths:
- /Users/asher/tower/imp/crates/imp-llm/src/model.rs
- /Users/asher/tower/imp/crates/imp-cli/src/lib.rs
---

Add first-class model registry support for the newly released ChatGPT 5.5 / GPT-5.5 models in imp. Inspect existing OpenAI model metadata, alias resolution, and tests in crates/imp-llm/src/model.rs plus any CLI/runtime assumptions. Add the new model IDs and aliases in a minimal way that preserves existing behavior and lets users select the new model by explicit name or obvious alias. Keep scope to model resolution and surfaced metadata only; do not change provider auth flows unless required by inspected code.
