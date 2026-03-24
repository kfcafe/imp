---
id: '20'
title: 'imp efficiency: parallelize grep block search with rayon'
slug: imp-efficiency-parallelize-grep-block-search-with
status: closed
priority: 2
created_at: '2026-03-23T00:00:08.283281Z'
updated_at: '2026-03-24T06:26:56.662686Z'
closed_at: '2026-03-24T06:26:56.662686Z'
verify: 'cd /Users/asher/tower && cargo test -p imp-core "tools::grep" -- --test-threads=1 2>&1 | grep -q ''test result: ok'''
is_archived: true
history:
- attempt: 1
  started_at: '2026-03-24T06:26:56.050429Z'
  finished_at: '2026-03-24T06:26:56.639666Z'
  duration_secs: 0.589
  result: pass
  exit_code: 0
---

## Problem
Block search (grep with blocks=true) walks files sequentially. Benchmark shows 500ms-1.4s for imp-core/src. The ignore crate supports parallel walking, and file processing is CPU-bound (tree-sitter parsing) — perfect for parallelism.

## Design
- Use `ignore::WalkBuilder::build_parallel()` or `rayon` for parallel file processing
- Each file: read → match → parse → extract blocks independently
- Collect results and sort at the end
- Benchmark target: 2-4x speedup on multi-core machines

## Files
- `imp/crates/imp-core/src/tools/grep.rs` — `walk_files` + `execute_block_search`
- `imp/crates/imp-core/Cargo.toml` — add rayon dep if needed

## Acceptance
- Benchmark shows measurable speedup on block search
- All grep tests pass
- Line search unaffected
