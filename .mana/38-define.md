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
verify: test -n "design-only"
kind: job
---

## Current State
We want imp to use mana jobs as its delegated-worker substrate instead of inventing a second planning system. Right now that intention exists conceptually, but we do not yet have a clear, reusable contract for what a good imp-authored child job description looks like.

## Task
Write the delegated child-job contract that imp should follow when creating mana child jobs.

The writeup should define the standard shape of a child job description, including:
1. goal / current-state framing
2. scope boundaries
3. expected deliverable
4. explicit patch or no-patch guidance
5. important files or subsystem focus when known
6. done condition and verify expectations

## Files to Modify
- `ARCHITECTURE.md`
- `README.md`
- `crates/imp-core/src/system_prompt.rs` — only if a small prompt-level reference belongs there

## Required Guidance
The contract should make it easy for a future agent to author child jobs that are:
- sharply scoped
- executable in one pass where practical
- useful to a parent job after completion
- consistent with mana best practices for descriptions and verification

## Scope Boundaries
- Do **not** invent a new todo/planning model outside mana
- Do **not** add orchestration runtime behavior here; this is the contract/design unit
- Keep the writeup concise and operational, not philosophical

## How to Verify
Run: `test -n design-only`

## Done When
- a concise delegated-job contract exists in imp docs or design notes
- the contract explains how imp should author child job descriptions for mana
- it clearly treats mana jobs as the delegated-worker substrate, not a second planning system
