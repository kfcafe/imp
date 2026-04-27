---
id: '267'
title: Fix native imp worker OpenAI route failure when spawned runs omit instructions
slug: fix-native-imp-worker-openai-route-failure-when-sp
status: open
priority: 1
created_at: '2026-04-21T04:02:53.561523Z'
updated_at: '2026-04-21T04:02:53.561523Z'
labels:
- bug
- imp
- providers
- worker
- openai
verify: cd /Users/asher/tower && cargo test -p imp-core should_use_openai_chatgpt_route -- --nocapture && cargo test -p imp-llm openai_empty_instructions_omitted -- --nocapture && cargo check -p imp-core -p imp-llm
kind: job
---

Investigate and fix the native imp worker/provider path so spawned unit runs do not fail on the OpenAI Responses route with `HTTP 400 Bad Request: {"detail":"Instructions are required"}` when no explicit `system_prompt` is passed. Repro came from using the native `imp` tool in unit mode; the same work succeeded when forced onto `provider=openai-codex` with a non-empty `system_prompt`. Inspect `imp/crates/imp-core/src/tools/imp.rs`, `imp/crates/imp-core/src/mana_worker.rs`, `imp/crates/imp-core/src/imp_session.rs`, and `imp/crates/imp-llm/src/providers/openai.rs` / `openai_codex.rs` to determine why worker runs can reach the OpenAI route with empty instructions and whether the fix belongs in system-prompt assembly, provider routing, or request building. Keep the fix small, add a regression test if practical, and verify with focused imp-core/imp-llm tests.
