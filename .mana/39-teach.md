---
id: '39'
title: Teach imp to decompose broad work into scoped mana child jobs
slug: teach
status: closed
priority: 1
created_at: '2026-03-29T22:14:05.539077Z'
updated_at: '2026-03-31T04:38:47.705290Z'
acceptance: |-
  imp has explicit decomposition guidance or behavior for broad work that should become child mana jobs.
  The implementation prefers sharply scoped child jobs over vague catch-all delegation.
  The work stays inside imp boundaries and reuses mana as the planning and delegation substrate.
  Relevant tests or prompt-level regression coverage are added where practical.
notes: |-
  ---
  2026-03-29T22:30:20.137590+00:00
  Backlog modeling decision: land this as both prompt guidance and lightweight runtime heuristics where needed. Keep the first pass narrow and testable; do not build a second planning system.

  ---
  2026-03-31T04:38:47.676830+00:00
  Implemented as part of MANA_DELEGATION_GUIDANCE const in system_prompt.rs. The const includes: decomposition heuristics (when to create child jobs vs do it yourself), child job description quality rules (concrete steps, file paths, embedded context, scope boundaries, anti-patterns), verify command guidance (existence checks, avoid bare runners), and sizing rules (one outcome, 1-5 files, pick the approach). Guidance is injected for Full, Orchestrator, and Planner modes when the mana tool is present. 3 regression tests added: delegation appears in Full mode with mana, skipped for Worker, skipped without mana tool.
labels:
- imp-core
- mana
- delegation
- orchestration
- prompting
dependencies:
- '38'
verify: cd /Users/asher/tower && rg -q 'decompos' imp/crates/imp-core/src/system_prompt.rs && rg -q 'child.job\|child_job' imp/crates/imp-core/src/system_prompt.rs && cargo test -p imp-core system_prompt && cargo check -p imp-core
kind: job
---

## Current State
Once the delegated child-job contract exists, imp should start using it when work becomes too broad or multi-stage for one coherent worker pass. Right now decomposition is not yet explicit enough: broad work can remain in one thread instead of being turned into sharply scoped mana child jobs.

## Task
Teach imp to decompose broad work into scoped mana child jobs.

Implement a first pass that uses both:
1. prompt guidance telling the model when to decompose into child jobs
2. lightweight runtime heuristics where needed to keep that behavior narrow and testable

## Files to Modify
- `crates/imp-core/src/agent.rs`
- `crates/imp-core/src/system_prompt.rs`
- `crates/imp-core/src/tools/mana.rs`

## Decomposition Rules to Prefer
When work is broad or multi-stage, child jobs should be:
- narrowly scoped
- explicit about deliverable
- explicit about patch or no-patch behavior
- anchored to files, subsystems, or investigation targets when known
- easier to complete than the original broad request

## Scope Boundaries
- Reuse the contract from unit `38`
- Do **not** build a second planning system outside mana
- Keep the first pass small enough to verify with focused tests or prompt-level checks

## Edge Cases
- avoid creating vague child jobs like "investigate bug"
- avoid over-decomposing tiny tasks
- decomposition should preserve parent clarity rather than creating transcript clutter

## How to Verify
Run: `cd /Users/asher/tower && cargo check -p imp-core`

## Done When
- imp has explicit decomposition guidance or behavior for broad work that should become child mana jobs
- the implementation prefers sharply scoped child jobs over vague catch-all delegation
- tests or prompt-level regression coverage exist where practical
