---
id: '50'
title: '[temp] Smoke test fresh-built Codex/OpenAI headless worker path'
slug: temp-smoke-test-fresh-built-codexopenai-headless-w
status: open
priority: 3
created_at: '2026-04-16T03:05:55.700095Z'
updated_at: '2026-04-16T03:05:55.700095Z'
acceptance: A fresh cargo-built headless `imp` run can load this unit and emit `SMOKE_OK` without changing repo files.
labels:
- temp
- smoke-test
- imp-run
verify: test -n smoke-ok
kind: job
---

Temporary smoke-test unit for validating the direct headless `imp` worker path after the runtime/auth refactors.

Steps:
1. Read this unit.
2. Reply with exactly `SMOKE_OK`.
3. Do not use tools.
4. Do not edit files.
5. Stop immediately.

In scope:
- proving the worker starts and can send a minimal response

Out of scope:
- any repo changes
- any follow-up work
