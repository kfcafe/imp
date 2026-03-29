---
id: '40'
title: Teach imp to synthesize completed child-job outcomes back into parent flow
slug: teach-imp-to-synthesize-completed-child-job-outcom
status: open
priority: 1
created_at: '2026-03-29T22:14:54.672562Z'
updated_at: '2026-03-29T22:14:54.672562Z'
acceptance: |-
  imp has an explicit parent-flow synthesis path for completed child jobs.
  The synthesis highlights findings, touched files or scope, unresolved issues, and next action.
  Child job outcomes become easier to use than raw transcript replay.
  The work remains imp-local and does not replace mana as the durable work graph.
labels:
- imp-core
- mana
- delegation
- synthesis
- context
dependencies:
- '38'
verify: cd /Users/asher/tower && cargo check -p imp-core
kind: job
paths:
- crates/imp-core/src/agent.rs
- crates/imp-core/src/system_prompt.rs
- crates/imp-core/src/session.rs
---

Improve imp so it can consume completed mana child jobs cleanly and fold their outcomes back into the parent workflow. Focus on concise synthesis of child results: what the child found, what files or areas it touched, what remains unresolved, and what the parent should do next. Avoid treating child jobs as transcript spam; the parent should be able to make progress from structured child outcomes. Keep this work inside imp behavior and prompting rather than moving durable project memory into imp.
