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

  ---
  2026-03-29T22:12:28.796441+00:00
  Backlog review note: builder code already registers MemoryTool, SessionSearchTool, and MultiEditTool. This unit is likely close to done once docs and README wording are verified.

  ---
  2026-03-29T22:12:28.944473+00:00
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

Align imp's default native tool surface with capabilities that already exist in code so runtime behavior, docs, and future UX work start from the same baseline. Limit scope to default registration and public surfaced contract for implemented built-ins, especially memory, session_search, and multi_edit. Do not add new UX flows, LSP, checkpoints, or planning UI in this unit. Files in scope: crates/imp-core/src/builder.rs and README.md.
