---
id: '38'
title: Define delegated child-job contract for imp-authored mana work
slug: define
status: open
priority: 1
created_at: '2026-03-29T22:13:00.593958Z'
updated_at: '2026-03-29T22:13:40.613238Z'
acceptance: |-
  A concise delegated-job contract exists in imp docs or design notes.
  The contract explains how imp should author child job descriptions for mana.
  It explicitly treats mana jobs as the delegated-worker substrate, not a second planning system.
  The writeup includes guidance for goal, scope, expected output, done condition, and patch boundaries.
labels:
- design
- mana
- delegation
- docs
- imp-core
kind: job
---

Write the contract imp should use when it creates mana jobs as delegated subwork. Treat mana jobs as the subagent substrate: the child job description is the child worker brief. Define a standard shape for child job descriptions, including goal, scope boundaries, expected output, done condition, and explicit patch or no-patch guidance. Document how this differs from ad hoc planning notes, and keep the design inside imp boundaries rather than inventing a separate planning system. Update the relevant imp-facing docs or design notes in repo scope.
