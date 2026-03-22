---
id: '16'
title: 'imp-core hardening: production-ready agent engine'
slug: imp-core-hardening-production-ready-agent-engine
status: open
priority: 0
created_at: '2026-03-22T10:27:16.725150Z'
updated_at: '2026-03-22T10:27:16.725150Z'
verify: cd /Users/asher/tower && cargo test -p imp-core 2>&1 | tail -5
---

Production hardening for imp-core. The core engine is implemented and passing 216 tests. This unit covers the wiring and resilience features from the Elixir plan that make it production-ready.

Children cover: config→agent wiring, tool argument validation, LLM retry, loop detection, file-not-found suggestions, auto-resume after compaction, file read tracking, file version history.

See imp_core_plan.md for full spec.
