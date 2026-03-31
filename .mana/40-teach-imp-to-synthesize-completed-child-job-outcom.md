---
id: '40'
title: Teach imp to synthesize completed child-job outcomes back into parent flow
slug: teach-imp-to-synthesize-completed-child-job-outcom
status: closed
priority: 1
created_at: '2026-03-29T22:14:54.672562Z'
updated_at: '2026-03-31T04:39:46.032843Z'
acceptance: |-
  imp has an explicit parent-flow synthesis path for completed child jobs.
  The synthesis highlights findings, touched files or scope, unresolved issues, and next action.
  Child job outcomes become easier to use than raw transcript replay.
  The work remains imp-local and does not replace mana as the durable work graph.
notes: |-
  ---
  2026-03-31T04:39:46.005862+00:00
  Implemented as part of MANA_DELEGATION_GUIDANCE const. The 'After child jobs complete' section tells the model to: check completed children with mana show, synthesize what changed/touched/unresolved, summarize concisely for next step, and diagnose failures before retrying. This is prompt-only v1 — no runtime machinery. Regression tests cover the guidance appearing in Full mode and being skipped for Worker/no-mana.
labels:
- imp-core
- mana
- delegation
- synthesis
- context
dependencies:
- '38'
verify: cd /Users/asher/tower && rg -qi 'synthesize' imp/crates/imp-core/src/system_prompt.rs && rg -q 'After child jobs complete' imp/crates/imp-core/src/system_prompt.rs && cargo test -p imp-core system_prompt_delegation && cargo check -p imp-core
kind: job
paths:
- crates/imp-core/src/agent.rs
- crates/imp-core/src/system_prompt.rs
- crates/imp-core/src/session.rs
---

## Current State
If imp creates mana child jobs, the parent flow also needs a clean way to consume their results. Right now the risk is that child jobs become more transcript to reread instead of useful summarized outcomes the parent can act on.

## Task
Teach imp to synthesize completed child-job outcomes back into the parent flow.

Implement a first pass that helps the parent agent extract from completed child jobs:
1. what the child found
2. what files or scope it touched
3. what remains unresolved
4. what the parent should do next

## Files to Modify
- `crates/imp-core/src/agent.rs`
- `crates/imp-core/src/system_prompt.rs`
- `crates/imp-core/src/session.rs`

## Scope Boundaries
- Do **not** move durable project memory into imp
- Do **not** replace mana as the work graph or source of truth
- Focus on concise parent-flow synthesis, not a big memory subsystem

## Edge Cases
- child jobs that fail or end inconclusively should still produce useful synthesis
- synthesis should be easier to consume than replaying raw transcripts
- parent flow should be able to distinguish findings from unresolved questions

## How to Verify
Run: `cd /Users/asher/tower && cargo check -p imp-core`

## Done When
- imp has an explicit parent-flow synthesis path for completed child jobs
- the synthesis highlights findings, touched scope, unresolved issues, and next action
- child job outcomes become easier to use than raw transcript replay
