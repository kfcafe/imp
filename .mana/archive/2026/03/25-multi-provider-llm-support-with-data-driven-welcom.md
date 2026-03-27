---
id: '25'
title: Multi-provider LLM support with data-driven welcome flow
slug: multi-provider-llm-support-with-data-driven-welcom
status: closed
priority: 2
created_at: '2026-03-24T16:56:28.551472Z'
updated_at: '2026-03-24T17:13:47.237738Z'
labels:
- feature
closed_at: '2026-03-24T17:13:47.237738Z'
close_reason: 'Auto-closed: all children completed'
verify: cargo check -p imp-llm && cargo check -p imp-tui && cargo check -p imp-cli
is_archived: true
---

Replace the hardcoded 3-provider system (Anthropic, OpenAI, Google) with a data-driven provider registry that supports ~10+ providers out of the box and allows users to enter API keys for any LLM.

Currently hardcoded in 5 places:
- `imp-llm/src/providers/mod.rs` — `create_provider()` match on 3 names
- `imp-llm/src/auth.rs` — `AuthStore::resolve()` maps 3 provider names to env vars
- `imp-llm/src/model.rs` — `builtin_models()` for 3 providers
- `imp-tui/src/views/welcome.rs` — `WelcomeProvider` 3-variant enum
- `imp-cli/src/main.rs` — `run_login()` only handles anthropic

New providers to add: DeepSeek, Groq, Together, Mistral, xAI/Grok, OpenRouter, Fireworks — all OpenAI Chat Completions compatible.

Reference: models.dev (https://models.dev/api.json) is an open-source provider/model registry used by OpenCode.
