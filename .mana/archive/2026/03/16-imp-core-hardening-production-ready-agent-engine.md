---
id: '16'
title: 'imp-core hardening: production-ready agent engine'
slug: imp-core-hardening-production-ready-agent-engine
status: closed
priority: 0
created_at: '2026-03-22T10:27:16.725150Z'
updated_at: '2026-03-24T06:26:51.120931Z'
closed_at: '2026-03-24T06:26:51.120931Z'
verify: cd /Users/asher/tower && cargo test -p imp-core 2>&1 | tail -5
is_archived: true
history:
- attempt: 1
  started_at: '2026-03-24T06:26:11.484362Z'
  finished_at: '2026-03-24T06:26:51.082327Z'
  duration_secs: 39.597
  result: pass
  exit_code: 0
outputs:
  text: |-
    running 0 tests

    test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
---

Production hardening for imp-core. The core engine is implemented and passing 216 tests. This unit covers the wiring and resilience features from the Elixir plan that make it production-ready.

Children cover: config→agent wiring, tool argument validation, LLM retry, loop detection, file-not-found suggestions, auto-resume after compaction, file read tracking, file version history.

See imp_core_plan.md for full spec.
