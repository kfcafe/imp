---
id: '37'
title: Add first-class usage accounting and reporting to imp
slug: add-first-class-usage-accounting-and-reporting-to
status: open
priority: 1
created_at: '2026-03-28T10:29:18.479526Z'
updated_at: '2026-03-29T22:12:28.929741Z'
notes: |-
  ---
  2026-03-29T22:12:17.266767+00:00
  Backlog review note: usage schema, persistence helpers, and reporting code are already present in the tree. Prioritize finishing verification, docs, and polish and close delivered subwork instead of treating this as a greenfield feature.
labels:
- feature
- usage
- analytics
- imp-core
- imp-cli
- imp-tui
- imp-llm
kind: epic
feature: true
---

Implement first-class token/cost usage accounting and reporting for imp. We already receive per-request usage from providers and sometimes persist it incidentally inside assistant messages, but imp has no canonical usage store and no `imp usage` reporting command. Build a plan-compatible v1 with these product decisions locked in:

1. V1 should report from existing session data immediately, even if incomplete.
2. The canonical future store is session-local structured custom entries in the existing session JSONL, not mana and not a separate global ledger.
3. Historical cost should be stored at request time; do not rely only on recomputing from the current pricing table.
4. Reporting semantics are every completed request ever made, with fork/copy dedupe rather than only the current branch tip.

Required outcomes:
- Define a versioned usage session entry schema and persistence helpers in imp-core.
- Persist usage consistently across interactive, print, headless, and ImpSession-driven flows.
- Add `imp usage` CLI reporting that reads both new canonical usage entries and legacy assistant-message usage as a fallback for existing history.
- Include dedupe semantics for forked/copied session history so totals are better than naive ccusage-style scans.
- Keep ownership inside imp: imp-llm normalizes provider usage, imp-core owns persistence/session integration, imp-cli owns reporting UX.

Suggested decomposition:
- schema/design + aggregation semantics
- imp-core persistence plumbing
- entry-point/runtime wiring across CLI/TUI/ImpSession paths
- CLI reporting commands and rendering
- tests/docs/polish

Keep the implementation local to imp. Do not add mana-level storage, do not depend on claude.ai, and do not introduce a separate canonical global usage ledger. A derived cache/index is acceptable later, but not as the source of truth in this epic.
