---
id: '45'
title: Explore AST-backed symbolic plan layer for imp
slug: explore-ast-backed-symbolic-plan-layer-for-imp
status: open
priority: 2
created_at: '2026-04-07T17:13:35.370422Z'
updated_at: '2026-04-07T17:14:00.187532Z'
notes: |-
  ---
  2026-04-07T17:14:00.187528+00:00
  User explicitly requested that `mana` be loaded with `read` before continuing. Reloaded `/Users/asher/tower/imp/.imp/skills/mana/SKILL.md` and will use its doctrine for future unit design, decomposition, retries, and worker handoff. Treat 45 as exploratory only and non-blocking for script-tool implementation.
labels:
- architecture
- imp
- ast
- plan-layer
- exploration
verify: test -n "ast-plan-layer-exploration-tracked"
kind: epic
---

Goal: explore whether imp should add a structured AST/symbolic plan layer for readonly composition, distinct from both normal tool calls and the future script sandbox.

Current state:
- This is a speculative adjacent idea raised while discussing guest runtimes and the script tool.
- No current imp runtime supports a first-class plan/query language.
- The value proposition is not yet proven; this work should stay exploratory and should not block script-tool implementation.

Questions to answer:
1. What concrete failure modes in imp would justify a plan layer?
2. Should the plan representation be JSON AST, S-expression syntax, or remain internal-only?
3. Which readonly operations would belong in a first version?
4. How would a plan layer interact with existing tools, the TUI, and session logging?
5. What evidence would show this is worth keeping versus unnecessary complexity?

Scope:
- design/research only
- imp-local `.mana/`
- adjacent to script-tool work, but not a prerequisite for shipping a minimal script sandbox

Out of scope:
- implementation
- choosing a full general-purpose language runtime
- expanding this into the main extension architecture before evidence exists
