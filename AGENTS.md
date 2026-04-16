# AGENTS.md — imp

Read `../AGENTS.md` first.

This file adds project-specific guidance for work inside `imp/`.

## What imp is

Today, `imp` is the agent engine and native worker/runtime in the Tower repo.

For future-facing architecture work, the naming direction is:
- `mana` = platform
- `imp` = agent + default human-facing environment on mana
- `runtime` = live execution layer
- `graph` = durable layer
- `extension` = packaged extensibility
- `action` = preferred system behavior term
- `task` = preferred work term

Use that vocabulary in new architecture docs and migration plans.
Do not mechanically rewrite older docs or code names when they still describe current repo reality more accurately.

## Project focus

Prioritize work that improves:
- agent quality
- runtime boundaries
- context assembly
- policy enforcement
- tool behavior
- embedding/hostability
- extension seams
- structured outcomes back into mana

## Current extension reality

Current shipped extension support is Lua.
Treat TypeScript extensions as the preferred future direction, but do not describe them as already shipped unless the repo actually implements them.

## Ownership heuristics

Put work in `imp/` when it is about:
- agent behavior
- runtime execution
- context assembly
- tool registration or tool UX
- provider/model integration
- session behavior
- policy enforcement during execution
- agent-facing interfaces
- embedding surfaces for apps built on mana

Escalate to root architecture work when the change affects:
- the mana/imp split
- runtime vs graph boundaries
- extension system contracts
- cross-app platform APIs
- naming and ontology used across Tower

## Useful docs

- `README.md`
- `imp_ontology.md`
- `imp_rebuild_plan.md`
- `../docs/architecture/mana-platform-target-architecture.md`
