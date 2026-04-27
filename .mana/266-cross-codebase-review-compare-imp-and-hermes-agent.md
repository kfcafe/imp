---
id: '266'
title: 'Cross-codebase review: compare imp and hermes-agent for gaps and adoptable ideas'
slug: cross-codebase-review-compare-imp-and-hermes-agent
status: open
priority: 2
created_at: '2026-04-21T02:21:52.012180Z'
updated_at: '2026-04-21T02:30:46.967382Z'
acceptance: Review inspects both codebases directly, names concrete evidence for major findings, and returns prioritized recommendations for what imp should adopt, what to avoid, and where imp is already stronger.
notes: |-
  ---
  2026-04-21T02:23:33.623825+00:00
  Review decomposition captured from planning:

  1) Primary pass: compare core agent/runtime surfaces first.
  - imp focus: crates/imp-core (agent loop, system_prompt, tools, mana integration, session/compaction, guardrails, worker path), crates/imp-cli, crates/imp-tui, crates/imp-llm, extension seams.
  - hermes focus: run_agent.py, model_tools.py, agent/* internals, tools/registry + key tools, cli/tui bridge, session/memory/skills/context components.

  2) Secondary pass: shorter product/platform comparison for ideas worth borrowing.
  - gateway / messaging surfaces
  - cron / scheduled automations
  - multi-environment execution backends
  - research / evaluation / trajectory infrastructure

  3) Evaluation lenses:
  - agent loop and stop/continue policy
  - runtime boundaries and hostability
  - tool registration and tool UX
  - context assembly / compression / prefill
  - memory / skills / learning loop
  - delegation / orchestration substrate
  - sessions / persistence / branching / search
  - provider auth / failover / model metadata
  - extension and plugin architecture
  - operational and product UX
  - evals / telemetry / observability

  4) Deliverable shape:
  - gaps in imp
  - concrete hermes implementations worth adopting now
  - longer-term architectural borrowings
  - patterns to avoid copying
  - places imp is already stronger

  Ground findings in inspected code paths, not README claims alone. Because this is cross-project work, keep root-scope mana as source of truth.

  ---
  2026-04-21T02:28:11.500442+00:00
  User confirmed full scope: include both core agent/runtime comparison and broader Hermes product/platform features (gateway, cron, multi-environment execution, RL/research tooling) in the review.

  ---
  2026-04-21T02:30:46.967374+00:00
  Progress: gathered initial architecture evidence from imp-core and Hermes core files plus audit scans. Next pass is focused evidence collection on sessions/search, tool registry/extension seams, delegation, gateway/cron/env backends, and evaluation/testing surfaces before synthesizing findings.
labels:
- review
- imp
- hermes-agent
- architecture
kind: job
decisions:
- 'Decision: prioritize the core agent/runtime comparison first, then include broader Hermes product/platform features only as a secondary, shorter section focused on ideas plausibly adoptable into imp.'
---

Perform a read-only comparative review of /tower/imp and ~/hermes-agent. Identify architectural, runtime, tooling, context-assembly, extension, evaluation, UX, and operational gaps in imp; identify concrete ideas and existing implementations in hermes-agent worth adopting into imp; separate near-term adoptable patterns from longer-term architectural borrowings; ground findings in inspected files and code paths rather than high-level README claims. Produce a concise written assessment and, if useful, follow-up mana units for high-value adoption opportunities.
