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

## Module organization guidance

Prefer local `AGENTS.md` files over crate READMEs for instructions that should shape future agent work.
Use them to define ownership boundaries, extraction plans, naming conventions, and review rules close to the code they govern.

When decomposing large files, preserve behavior first:
- split by responsibility, not by arbitrary line count
- keep public API churn minimal unless the current API is the problem
- move tests with the behavior they protect when practical
- avoid mixing mechanical moves with semantic changes
- run the narrowest crate-level check after each extraction

Current high-priority decomposition targets:
- `crates/imp-tui/src/app.rs`: split app state, event loop, runtime signals, auth/secrets flow, render caches, and agent event handling
- `crates/imp-core/src/agent.rs`: split turn loop, tool execution, next-action assessment, retry handling, and mode/policy enforcement
- `crates/imp-cli/src/lib.rs`: split args, auth/setup, headless worker mode, RPC mode, chat shell, and import/install helpers
- `crates/imp-core/src/tools/mana.rs`: split schema/action dispatch, native run orchestration, run-state persistence, rendering, and policy

## Useful docs

- `README.md`
- `imp_ontology.md`
- `imp_rebuild_plan.md`
- `../docs/architecture/mana-platform-target-architecture.md`
