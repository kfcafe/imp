---
id: '73'
title: Code-intelligence outputs are transient by default and should be promoted into mana only as notes, verify evidence, facts, or durable decisions
slug: code-intelligence-outputs-are-transient-by-default
status: open
priority: 3
created_at: '2026-04-09T14:19:47.654498Z'
updated_at: '2026-04-09T14:19:47.654498Z'
labels:
- fact
verify: test -f .mana/73-code-intelligence-outputs-are-transient-by-default.md && rg -q '^id:' .mana/73-code-intelligence-outputs-are-transient-by-default.md
kind: epic
unit_type: fact
last_verified: '2026-04-09T23:16:10.195220Z'
stale_after: '2026-05-09T23:16:10.195220Z'
paths:
- docs/design/code-intelligence-evidence-promotion.md
- docs/design/code-intelligence-strategy.md
---

Captured by `docs/design/code-intelligence-evidence-promotion.md`. The design note defines the durable intake boundary: `imp` owns live code-intelligence runtime behavior; `mana` stores only promoted consequences that another worker should inherit cold. It distinguishes retry-relevant notes, closure/review evidence, stable facts, and durable decisions, and recommends normalized summaries plus artifact references over raw diagnostic or protocol dumps.
