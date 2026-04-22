---
id: '267'
title: Adopt highest-value product lessons from opencode into imp
slug: adopt-highest-value-product-lessons-from-opencode
status: open
priority: 1
created_at: '2026-04-22T15:37:11.567786Z'
updated_at: '2026-04-22T15:43:25.740921Z'
acceptance: There is a durable epic in project mana that records the chosen opencode-inspired focus areas, their intent, and follow-up decomposition targets for imp.
notes: |-
  ---
  2026-04-22T15:37:19.835204+00:00
  User explicitly picked three opencode-inspired focus areas as high-value for imp: 1) first-class worktrees, 2) delegation UX, 3) packaging/install/docs polish. Treat these as productization layers on top of mana/imp, not replacements for the substrate.

  ---
  2026-04-22T15:43:25.740918+00:00
  Decision: treat 267 as a cross-cutting product umbrella, not the main execution parent. Fold concrete work into existing epics where they already own the right substrate: worktree UX under 47.3, delegation UX under 27/28.1/29 depending on layer, and install/docs polish under 28.1 plus existing install bugfixes like 54.
labels:
- imp
- product
- ux
- worktrees
- delegation
- install
- docs
- competitive-analysis
verify: cd /Users/asher/tower/imp && mana list --status open >/dev/null
kind: epic
feature: true
---

Capture the concrete product-level improvements imp should steal from opencode without giving up mana/imp substrate advantages. Initial focus from user: first-class worktree UX, lightweight delegation UX over mana/orchestration, and packaging/install/docs polish. This epic should track user-facing productization work rather than substrate replacement.
