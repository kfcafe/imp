---
id: '268'
title: Smoke test direct imp run OpenAI auth resolution
slug: smoke-test-direct-imp-run-openai-auth-resolution
status: open
priority: 2
created_at: '2026-04-23T05:16:31.014092Z'
updated_at: '2026-04-23T05:16:31.014092Z'
labels:
- smoke
- auth
- openai
- worker-runtime
verify: test -n smoke-only
kind: job
---

Temporary verification unit for 28.1.5.1. Run headless with `imp run` / `imp mana <unit-id>` using `--provider openai -m gpt-5.4-nano --no-tools --max-turns 1 --mode json --defer-verify`. The worker should emit a single short response containing `SMOKE_OK` and must not fail with `Incorrect API key provided: ''` or any empty-string auth path. Prompt intent: reply with exactly `SMOKE_OK` and stop.
