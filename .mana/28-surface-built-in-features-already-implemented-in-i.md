---
id: '28'
title: Surface built-in features already implemented in imp
slug: surface-built-in-features-already-implemented-in-i
status: open
priority: 1
created_at: '2026-03-26T02:57:24.094581Z'
updated_at: '2026-03-29T22:12:28.944480Z'
notes: |-
  ---
  2026-03-29T22:12:17.281699+00:00
  Backlog review note: builder code already registers MemoryTool, SessionSearchTool, and MultiEditTool. This unit is likely close to done once docs and README wording are verified.
labels:
- feature
- ux
- imp-core
- docs
verify: cd /Users/asher/tower/imp && rg 'tools\.register\(Arc::new\(MemoryTool\)\);' crates/imp-core/src/builder.rs && rg 'tools\.register\(Arc::new\(SessionSearchTool\)\);' crates/imp-core/src/builder.rs && rg 'tools\.register\(Arc::new\(MultiEditTool\)\);' crates/imp-core/src/builder.rs && rg 'persistent memory' README.md && rg 'session search|search past conversations' README.md && cargo check -p imp-core
fail_first: true
kind: epic
---

## Current State
Several built-in imp capabilities already exist in code, but the default surfaced contract is not fully aligned with that reality. In particular, memory, session search, and multi-edit should be treated as part of the stock built-in experience so docs and future UX work match the actual runtime.

## Task
Align the default built-in tool surface and public docs with what imp already implements.

Do the following:
1. confirm the relevant built-in tools are registered in the default builder path
2. make README/runtime-facing copy treat these as first-class built-ins, not hidden extras
3. keep the scope tightly focused on the default surfaced contract for implemented built-ins

## Files to Modify
- `crates/imp-core/src/builder.rs`
- `README.md`

## Important Built-ins to Surface
- `memory`
- `session_search`
- `multi_edit`

## Scope Boundaries
- Do **not** add new backend capability here
- Do **not** add TUI-only discoverability work here; that belongs in `29`
- Do **not** add checkpoint or planning UX here

## Edge Cases
- documentation should describe what is truly available by default
- builder registration and README wording should not drift apart
- avoid promising behavior that is project-local or extension-only

## How to Verify
Run: `cd /Users/asher/tower/imp && rg "tools\.register\(Arc::new\(MemoryTool\)\);" crates/imp-core/src/builder.rs && rg "tools\.register\(Arc::new\(SessionSearchTool\)\);" crates/imp-core/src/builder.rs && rg "tools\.register\(Arc::new\(MultiEditTool\)\);" crates/imp-core/src/builder.rs && rg "persistent memory" README.md && rg "session search|search past conversations" README.md && cargo check -p imp-core`

## Done When
- the default native tool surface matches implemented built-ins
- docs and runtime expectations start from the same baseline
