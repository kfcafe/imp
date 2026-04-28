---
id: '291'
title: Rewrite imp README around product features, mana, and influences
slug: rewrite-imp-readme-around-product-features-mana-an
status: open
priority: 2
created_at: '2026-04-28T00:43:25.202204Z'
updated_at: '2026-04-28T00:44:12.297781Z'
acceptance: README.md is rewritten with a professional structure, feature focus, mana section, influences section, install/quickstart, and accurate current limitations.
notes: |2

  ## Attempt 1 — 2026-04-28T00:44:12Z
  Exit code: 127

  ```
  sh: README: command not found
  ```
labels:
- docs
- readme
verify: README exists and mentions mana, features, influences, install, quickstart, tools, sessions, modes, extensions, providers.
attempts: 1
history:
- attempt: 1
  started_at: '2026-04-28T00:44:12.238351Z'
  finished_at: '2026-04-28T00:44:12.297766Z'
  duration_secs: 0.059
  result: fail
  exit_code: 127
  output_snippet: 'sh: README: command not found'
kind: task
autonomy_disposition:
  kind: blocked
  blockers:
  - verify_failed
  review: unknown
  approval: unknown
  verify: failed
  visibility: unknown
  attempt_pressure: within_budget
  risk: unknown
  provenance: mixed
  continuation_budget: 2
---

Rewrite /Users/asher/imp/README.md into a professional product-oriented README. User wants emphasis on features, mana integration, and influences. Preserve accurate shipped/current-state claims from existing README: Rust-native coding agent, TUI/cockpit, CLI chat, tools, sessions, modes, providers, secure secrets, Lua current stable extension path, TypeScript limited compatibility/future path, mana native work coordination, SDK preview. Improve organization and tone; reduce stale/internal planning language; include influences without overclaiming lineage. Do not change code.
