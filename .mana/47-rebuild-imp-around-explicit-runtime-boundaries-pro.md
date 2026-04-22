---
id: '47'
title: Rebuild imp around explicit runtime boundaries, protocol-first workflow, and evidence-driven completion
slug: rebuild-imp-around-explicit-runtime-boundaries-pro
status: open
priority: 1
created_at: '2026-04-08T10:18:34.356594Z'
updated_at: '2026-04-22T15:58:30.584747Z'
notes: |-
  ---
  2026-04-22T15:58:30.566631+00:00
  Execution guidance: do not work the umbrella epic directly. Start from one bounded child track. Recommended entrypoints based on current conversation: (1) 47.6.1 if the goal is to make the rebuild sequence explicit before deeper design work, (2) 47.1.1 if the goal is to lock the mana↔imp protocol boundary first, (3) 47.3.2 if the goal is to push the opencode-inspired product layer around worktree UX now. Treat 47 itself as an orchestration/spec umbrella only.
labels:
- imp
- refactor
- rebuild
- epic
- architecture
verify: test -n "imp-rebuild-umbrella-epic-tracked"
kind: epic
decisions:
- The umbrella rebuild epic 47 should not be executed directly. Work should begin from a bounded child track, with the preferred starting units currently being 47.6.1 (migration ordering), 47.1.1 (mana↔imp protocol boundary), or 47.3.2 (user-facing worktree UX), depending on whether sequencing, contracts, or productized worktree UX is the immediate priority.
---

This is the umbrella rebuild epic for imp. Goal: keep imp as a pure-Rust runtime and product, but refactor it so the implementation more literally matches Tower's intended architecture: mana owns durable truth, imp owns live execution, and workflow trust comes from explicit contracts, execution boundaries, verification artifacts, and review gates rather than from large implicit runtime objects.

Context:
- `imp_rebuild_strategy.md` is the canonical design reference for this epic.
- We are not rewriting imp from scratch.
- We are not adding TypeScript to imp itself.
- We are reorganizing the runtime around clearer seams, smaller crates/modules, and stronger protocol/evidence boundaries.

In scope:
- formalize the mana ↔ imp contract
- decompose imp-core into cleaner runtime/context/tools/policy/worker seams
- define or introduce a runner boundary for execution, worktrees, and sandboxing
- make verification/review/evidence first-class workflow stages
- harden capability enforcement and extension boundaries
- sequence the migration so the product remains usable while internals change

Out of scope:
- rewriting the TUI from scratch
- introducing a web control plane inside imp
- broadening scope upward into Wizard/Familiar/platform work

Important remaining decisions to resolve explicitly:
1. where the shared contracts crate should live
2. whether the first runner boundary lives inside imp or as a Tower sibling layer
3. how much verification evidence imp emits vs what mana must persist durably
4. what the long-term guest-runtime model is for Lua/extensions
5. how aggressively to split crates now vs via modules first

Definition of done for this umbrella epic:
- the rebuild is broken into clear executable tracks with explicit decisions and child work
- every major track has a concrete next-step epic or job that another agent can pick up cold
- the migration order is explicit enough to avoid a rewrite-style reset

Suggested child tracks:
- contracts and ownership boundary
- imp-core decomposition
- runner/worktree/sandbox substrate
- verification/review/evidence pipeline
- capability policy and extension hardening
- migration and compatibility rollout
