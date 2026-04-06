---
id: '21'
title: 'imp efficiency: smarter tool output truncation'
slug: imp-efficiency-smarter-tool-output-truncation
status: in_progress
priority: 2
created_at: '2026-03-23T00:00:21.665478Z'
updated_at: '2026-04-06T21:30:50.666617Z'
notes: |2

  ## Attempt 1 — 2026-03-24T06:26:56Z
  Exit code: 1

  ```

  ```
verify: cd /Users/asher/tower && grep -q 'DEFAULT_LIMIT.*50\|of.*matches\|of.*results' imp/crates/imp-core/src/tools/grep.rs
checkpoint: '55c295c8901f6c58954da8e2bae9bbb1c578e7f8'
verify_hash: '55ad6a7f846ba231c25ba58d6f6c5c10b2a7a0e13d71a340161f3109ae693c58'
attempts: 1
claimed_by: imp
claimed_at: '2026-04-06T21:30:50.666617Z'
history:
- attempt: 1
  started_at: '2026-03-24T06:26:56.676229Z'
  finished_at: '2026-03-24T06:26:56.730668Z'
  duration_secs: 0.054
  result: fail
  exit_code: 1
kind: job
attempt_log:
- num: 1
  outcome: abandoned
  agent: imp
  started_at: '2026-04-06T21:30:50.666617Z'
---

## Problem
Tool outputs use fixed-size truncation (2000 lines, 50KB). This is context-blind — grep returning 100 matches when the model probably needs 10, scan dumping entire file structures, web read returning full pages. Every extra token burns context window and money.

## Design
1. grep: Default limit from 100 to 50 for line search. For block search, already 10.
2. scan: When extracting a single file, trim to just the requested file's output
3. web read: Consider more aggressive default truncation for large pages
4. All tools: Add a note about total results when truncating ("50 of 342 matches shown")

## Files
- `imp/crates/imp-core/src/tools/grep.rs` — adjust defaults
- `imp/crates/imp-core/src/tools/scan/mod.rs` — trim output
- `imp/crates/imp-core/src/tools/web/mod.rs` — review truncation

## Acceptance
- grep default limit reduced
- Truncation messages show total count
- Tests pass
