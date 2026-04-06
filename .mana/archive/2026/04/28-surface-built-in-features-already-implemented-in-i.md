---
id: '28'
title: Surface built-in features already implemented in imp
slug: surface-built-in-features-already-implemented-in-i
status: closed
priority: 1
created_at: '2026-03-26T02:57:24.094581Z'
updated_at: '2026-04-06T21:21:32.859337Z'
notes: |-
  ---
  2026-03-29T22:12:17.281699+00:00
  Backlog review note: builder code already registers MemoryTool, SessionSearchTool, and MultiEditTool. This unit is likely close to done once docs and README wording are verified.
labels:
- feature
- ux
- imp-core
- docs
closed_at: '2026-04-06T21:21:32.859337Z'
close_reason: Completed and verified locally against the unit verify gate.
verify: cd /Users/asher/tower/imp && rg 'tools\.register\(Arc::new\(MemoryTool\)\);' crates/imp-core/src/builder.rs && rg 'tools\.register\(Arc::new\(SessionSearchTool\)\);' crates/imp-core/src/builder.rs && rg 'tools\.register\(Arc::new\(MultiEditTool\)\);' crates/imp-core/src/builder.rs && rg 'persistent memory' README.md && rg 'session search|search past conversations' README.md && cargo check -p imp-core
fail_first: true
checkpoint: '789d75300ed36164587d597c43acee53f83b22e5'
verify_hash: e4c680d82d3ff31603a75815525b2878e570c0e8b7d4784672d164abcf0e3e0e
claimed_by: imp
claimed_at: '2026-04-06T21:20:33.471798Z'
is_archived: true
history:
- attempt: 1
  started_at: '2026-04-06T21:21:32.539576Z'
  finished_at: '2026-04-06T21:21:32.812219Z'
  duration_secs: 0.272
  result: pass
  exit_code: 0
outputs:
  text: |-
    tools.register(Arc::new(MemoryTool));
        tools.register(Arc::new(SessionSearchTool));
        tools.register(Arc::new(MultiEditTool));
    **Tools** — File I/O, shell execution, code search (grep, find, AST scan), web search, diff preview/apply, user prompts, mana unit management, persistent memory, session search across past conversations, and built-in multi-edit for coordinated file changes. Readonly tools run in parallel. Prefer native tools over shell wrappers when available; for mana operations, use the built-in `mana` tool instead of `bash` for equivalent actions.
    **Tools** — File I/O, shell execution, code search (grep, find, AST scan), web search, diff preview/apply, user prompts, mana unit management, persistent memory, session search across past conversations, and built-in multi-edit for coordinated file changes. Readonly tools run in parallel. Prefer native tools over shell wrappers when available; for mana operations, use the built-in `mana` tool instead of `bash` for equivalent actions.
kind: epic
attempt_log:
- num: 1
  outcome: success
  notes: Completed and verified locally against the unit verify gate.
  agent: imp
  started_at: '2026-04-06T21:20:33.471798Z'
  finished_at: '2026-04-06T21:21:32.859337Z'
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
