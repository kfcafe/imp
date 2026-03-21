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

## Configuration

imp is already mostly config-centric.

Config layering:
- built-in defaults
- `~/.config/imp/config.toml` for personal defaults
- `<project>/.imp/config.toml` for repo-shared behavior
- environment overrides such as `IMP_MODEL` and provider API keys
- CLI flags or interactive overrides on top

Use project config for shared agent behavior that should travel with a repo. Use user config for personal defaults across projects. Keep secrets like provider API keys in environment variables or a secrets system, not in committed `.imp/config.toml` files.

The canonical detailed reference remains `imp_core_plan.md`.
