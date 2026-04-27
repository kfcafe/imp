---
id: '280'
title: Review project gaps that would make imp stronger than current baseline
slug: review-project-gaps-that-would-make-imp-stronger-t
status: open
priority: 1
created_at: '2026-04-27T19:23:43.040906Z'
updated_at: '2026-04-27T19:23:55.205563Z'
acceptance: User receives a concise prioritized review; no code modifications are made.
notes: |-
  ---
  2026-04-27T19:23:49.503774+00:00
  Read-only review evidence gathered:
  - Repo is clean on branch split/imp at 0c82d65.
  - Core docs inspected: README.md, vNext.md, imp_rebuild_plan.md, imp_ontology.md, ARCHITECTURE.md, IMP_REVIEW.md, IMP_DEEP_REVIEW.md, ENGINEERING_GUARDRAILS.md, LEARNING_LOOP_SPEC.md.
  - Proposal docs inspected: guest-runtime-extension-substrate.md, imp-run-worker-contract-and-mana-run-handoff.md, mana-aware-runtime-context-read-path.md, tool-ab-harness-notes.md.
  - Crate map inspected: imp-cli, imp-core, imp-llm, imp-lua, imp-tui.
  - Largest files by current wc: imp-tui/app.rs 7078, imp-core/agent.rs 5782, imp-cli/lib.rs 5679, imp-core/tools/mana.rs 3514, imp-llm/providers/anthropic.rs 2463, imp-core/session.rs 2334, imp-tui/views/sidebar.rs 1781, imp-core/imp_session.rs 1777, imp-core/system_prompt.rs 1725, imp-core/config.rs 1550.
  - Existing mana roadmap already tracks many major gaps: 47 rebuild epic, 47.2 core decomposition, 47.3 runner/worktree/sandbox, 47.4 policy, 47.5 evidence, 27 mana tool UX, 29 discoverability, 37 usage reporting, 44 guest-runtime, 46 maturity pass, 267 opencode lessons.
  - Review synthesis: biggest gaps are not missing random features; they are making existing strengths reliable, visible, enforceable, and measurable: shared runtime bootstrap, hard policy boundaries, cancellation/tool execution semantics, evidence/eval loop, mana-aware context read path, productized checkpoints/planning, and install/onboarding polish.
labels:
- analysis
- imp
- roadmap
- competitive-review
verify: test -n "analysis-only"} aisuiu
kind: task
---

## Goal
Produce a concise architecture/product gap review for the current `/Users/asher/imp` repo focused on what would make imp stronger than its current baseline and stronger against peer coding-agent products.

## Scope
- In scope: read-only review of repo docs, mana roadmap, crate structure, known review findings, and high-signal code/module hotspots.
- Out of scope: code changes, broad implementation, dependency changes, or committing.

## Deliverable
- A prioritized gap analysis with evidence from inspected files and mana state.
- Call out what imp already has that should be amplified, and where the actual gaps are.

## Focus
- `README.md`
- `vNext.md`
- `imp_rebuild_plan.md`
- `imp_ontology.md`
- `IMP_REVIEW.md`
- `IMP_DEEP_REVIEW.md`
- `docs/proposals/*`
- crate structure under `crates/`

## Done / Verify
- Done when the user receives a ranked memo and durable review state is captured in mana.
- Verify: `test -n "analysis-only"`
