# imp

imp is the worker/agent engine in the Tower ecosystem.

It owns:
- the agent loop
- tool execution
- session persistence
- context management
- model/provider integration

## Key docs
- `imp_core_plan.md` — canonical technical spec for the Rust imp implementation
- `../VISION.md` — ecosystem vision
- `../UMBRELLA.md` — umbrella structure and migration context
- `../AGENTS.md` — root instructions for agents

## Place in the stack

```text
wizard     → supervises and presents agent work
imp        → executes work
mana       → stores durable work state
```

## Current state

This folder is part of the Tower umbrella root and contributes crates to the root Cargo workspace.
The original standalone imp repo still exists outside Tower as rollback/safety during the migration.
