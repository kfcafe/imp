---
id: '21'
title: 'imp efficiency: smarter tool output truncation'
slug: imp-efficiency-smarter-tool-output-truncation
status: open
priority: 2
created_at: '2026-03-23T00:00:21.665478Z'
updated_at: '2026-03-23T00:00:21.665478Z'
verify: cd /Users/asher/tower && grep -q 'DEFAULT_LIMIT.*50\|of.*matches\|of.*results' imp/crates/imp-core/src/tools/grep.rs
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
