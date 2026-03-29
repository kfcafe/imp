---
id: '39'
title: Teach imp to decompose broad work into scoped mana child jobs
slug: teach
status: open
priority: 1
created_at: '2026-03-29T22:14:05.539077Z'
updated_at: '2026-03-29T22:14:42.888674Z'
acceptance: |-
  imp has explicit decomposition guidance or behavior for broad work that should become child mana jobs.
  The implementation prefers sharply scoped child jobs over vague catch-all delegation.
  The work stays inside imp boundaries and reuses mana as the planning and delegation substrate.
  Relevant tests or prompt-level regression coverage are added where practical.
labels:
- imp-core
- mana
- delegation
- orchestration
- prompting
dependencies:
- '38'
kind: epic
decisions:
- Confirm whether this should land as prompt guidance only, runtime heuristics in imp-core, or both.
---

Improve imp so it more deliberately decomposes broad or multi-stage work into scoped mana child jobs instead of continuing in one bloated thread. Focus on heuristics and prompt or runtime behavior that produce good child jobs: narrow scope, clear deliverable, explicit patch or no-patch expectations, and useful file or subsystem targeting when available. Keep this local to imp behavior and prompting; mana already provides the job substrate. Reuse the delegated child-job contract from the related design unit rather than inventing a new workflow model.
