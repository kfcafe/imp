# AGENTS.md — imp

Read `../AGENTS.md` first.

This file adds project-specific guidance for work inside `imp/`.

## What imp is

Today, `imp` is the agent engine and native worker/runtime in the Tower repo.

For future-facing architecture work, use this vocabulary:
- `mana` = platform
- `imp` = agent + default human-facing environment on mana
- `runtime` = live execution layer
- `graph` = durable layer
- `extension` = packaged extensibility
- `action` = preferred system behavior term
- `task` = preferred work term

Use the vocabulary in new architecture docs and migration plans, but do not mechanically rewrite older docs or code when current names are more accurate.

## Project focus

Prioritize work that improves agent quality, runtime boundaries, context assembly, policy enforcement, tool behavior, embedding/hostability, extension seams, and structured outcomes back into mana.

## Extension reality

Current shipped extension support is Lua. Treat TypeScript extensions as the preferred future direction, but do not describe them as already shipped unless the repo implements them.

## Ownership boundaries

Put work in `imp/` when it concerns agent behavior, runtime execution, context assembly, tool registration/UX, provider/model integration, session behavior, execution policy, agent-facing interfaces, or embedding surfaces for apps built on mana.

Escalate to root architecture work when a change affects the mana/imp split, runtime vs graph boundaries, extension contracts, cross-app platform APIs, or Tower-wide naming/ontology.

## Module organization

Prefer local `AGENTS.md` files over crate READMEs for future agent instructions.

When decomposing large files, preserve behavior first:
- split by responsibility, not line count
- keep public API churn minimal unless the API is the problem
- move tests with the behavior they protect when practical
- avoid mixing mechanical moves with semantic changes
- run the narrowest crate-level check after each extraction

Current high-priority decomposition targets:
- `crates/imp-tui/src/app.rs`: app state, event loop, runtime signals, auth/secrets flow, render caches, agent event handling
- `crates/imp-core/src/agent.rs`: turn loop, tool execution, next-action assessment, retry handling, mode/policy enforcement
- `crates/imp-cli/src/lib.rs`: args, auth/setup, headless worker mode, RPC mode, chat shell, import/install helpers
- `crates/imp-core/src/tools/mana.rs`: schema/action dispatch, native run orchestration, run-state persistence, rendering, policy

## Useful docs

- `README.md`
- `imp_ontology.md`
- `imp_rebuild_plan.md`
- `../docs/architecture/mana-platform-target-architecture.md`
