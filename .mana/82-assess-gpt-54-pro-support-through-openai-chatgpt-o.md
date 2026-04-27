---
id: '82'
title: Assess GPT-5.4-pro support through OpenAI ChatGPT OAuth in imp
slug: assess-gpt-54-pro-support-through-openai-chatgpt-o
status: open
priority: 2
created_at: '2026-04-13T05:12:03.222257Z'
updated_at: '2026-04-13T05:22:05.970738Z'
notes: |-
  ---
  2026-04-13T05:12:31.827045+00:00
  Conversation-time plan externalized from chat before further work.

  Visible delta captured:
  - Created root epic 82 to track GPT-5.4-pro via OpenAI/ChatGPT OAuth in imp.
  - Created child 82.1 for the entitlement/backend probe against the existing `openai-codex` path.
  - Created child 82.2 for the conditional implementation slice, explicitly blocked on 82.1.

  Key decision captured:
  - Treat this as probe-first work. Do not add product-visible support for `gpt-5.4-pro` unless the existing ChatGPT OAuth/Codex backend accepts that model for a real subscribed account.
  - If probe fails, record the limitation as backend/entitlement, not an imp architecture blocker.

  Relevant inspected files this plan is grounded in:
  - `imp/crates/imp-llm/src/oauth/chatgpt.rs`
  - `imp/crates/imp-llm/src/providers/openai_codex.rs`
  - `imp/crates/imp-llm/src/model.rs`
  - `imp/crates/imp-cli/src/main.rs`
  - `imp/crates/imp-tui/src/app.rs`

  ---
  2026-04-13T05:22:05.970720+00:00
  User-provided signal (not independently verified in this session): GPT-5.4-pro is not exposed in the Codex CLI app. Given imp's current OpenAI OAuth path is the same general ChatGPT/Codex backend surface (`openai-codex` via `chatgpt.com/backend-api/codex/responses`), this makes support through the existing OAuth route unlikely.

  Working conclusion for now:
  - Do not spend implementation time adding product-visible GPT-5.4-pro support on the OAuth path without contradictory live evidence.
  - Keep 82.1 as an optional confirming probe rather than an assumed-next step.
  - Treat the current default position as 'likely unsupported on current OAuth/Codex surface' rather than an imp code limitation.
labels:
- imp
- openai
- oauth
- provider
- model
- investigation
kind: epic
---

Goal: determine whether imp can safely expose a `gpt-5.4-pro` model when authentication comes from OpenAI/ChatGPT OAuth rather than a normal OpenAI API key.

Current repo state inspected in this session:
- `imp/crates/imp-llm/src/oauth/chatgpt.rs` already implements ChatGPT/OpenAI OAuth login.
- `imp/crates/imp-llm/src/providers/openai_codex.rs` sends OAuth-backed requests to `https://chatgpt.com/backend-api/codex/responses` under provider id `openai-codex`.
- `imp/crates/imp-cli/src/main.rs` and `imp/crates/imp-tui/src/app.rs` already prefer `openai-codex` when only ChatGPT OAuth is available for OpenAI-family models.
- `imp/crates/imp-llm/src/model.rs` already includes built-in OpenAI models `gpt-5.4`, `gpt-5.4-mini`, `gpt-5.4-nano`, `gpt-5.3-chat-latest`, `gpt-5.3-codex`, and `gpt-5.3-codex-spark`, and `builtin_openai_codex_models()` retags the OpenAI list onto provider `openai-codex`.

Unknown: whether OpenAI's ChatGPT OAuth/Codex backend actually accepts `gpt-5.4-pro` for a subscribed account. That entitlement/backend question must be answered before exposing the model in imp.

Planned decomposition:
1. Probe the existing OAuth-backed `openai-codex` path with `gpt-5.4-pro` using a real logged-in account.
2. If the backend accepts the model, add the model metadata and aliases/tests in imp.
3. If the backend rejects the model, record the rejection as an entitlement/backend limitation and keep the product surface unchanged.

Do not implement speculative private-backend support beyond the already used OAuth/Codex path.
