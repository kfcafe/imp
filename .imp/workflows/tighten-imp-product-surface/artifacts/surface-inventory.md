# Current Product Surface Inventory

Status: draft inventory for `tighten-imp-product-surface`.

## Target framing from user

imp should become a minimal, extensible coding agent with:

- TUI as the primary interactive product surface.
- One-shot prompt mode as the primary non-interactive surface.
- Workflows as the durable orchestration primitive.
- Pi influence: minimal, extensible coding agent.
- Codex influence: solid, well-engineered agentic runtime.
- No compatibility obligation for old experimental commands/surfaces.
- No `/work` implementation in this workflow, though `/work`, `/workflow`, and `/workflows` can be future aliases.
- `/goal` and `/btw` are acknowledged as possible agent-harness patterns but intentionally out of scope for this cleanup.

## First-class surfaces to keep

### TUI

Keep as the main product surface.

Evidence:

- default interactive mode enters TUI in `crates/imp-cli/src/lib.rs`.
- TUI owns the slash-command palette and runtime controls in `crates/imp-tui/src/app.rs` and `crates/imp-tui/src/views/command_palette.rs`.

### One-shot prompt mode

Keep as the scriptable/non-interactive path.

Evidence:

- `imp -p` / print mode handled by `run_print_mode` in `crates/imp-cli/src/lib.rs`.

### Workflows

Keep and strengthen as the only durable orchestration primitive.

Evidence:

- native workflow tool is registered in `crates/imp-core/src/builder.rs`.
- schema/runtime docs in `docs/workflows.md`.
- implementation under `crates/imp-core/src/workflow/` and `crates/imp-core/src/tools/workflow.rs`.

### Lua extensions

Keep as the minimal extensibility mechanism, at least for now.

Evidence:

- `crates/imp-lua/` is the shipped extension runtime.
- TUI loads Lua commands/tools and `/reload` reloads config/extensions.
- README and `docs/extensions-lua.md` describe Lua as stable extension runtime.

## Candidate retained TUI commands

User-confirmed or strongly supported:

- `/new` — fresh session.
- `/resume` — important session recovery/browsing command.
- `/model` — model selector.
- `/compact` — context management.
- `/quit` — exit.
- `/loop` — useful steering/control primitive.
- `/reload` — reload config/extensions.
- `/setup` — setup wizard.
- `/secrets` — credential/API-key manager.
- `/login` — OAuth login.
- `/name` — name current session.
- `/tree` — session tree.
- `/settings` — settings editor.
- `/stop` — stop active work/loop.

Still undecided:

- `/fork` — likely useful but should be confirmed.
- `/personality` — likely remove; user explicitly suspects it is unnecessary.
- `/copy` — useful UX affordance but not mentioned by user; decide deliberately.

## Current TUI built-in command surface

Current `builtin_commands()` exposes many top-level commands:

- improve
- improve-safe
- improve-merge
- improve-help
- eval
- status
- autonomy
- clean
- loop
- queue
- run
- stop
- scope
- model
- settings
- mana
- tree
- fork
- compact
- new
- resume
- name
- copy
- export
- personality
- memory
- checkpoints
- restore-checkpoint
- reload
- hotkeys
- login
- secrets
- setup
- quit

Additional TUI command routes exist for:

- `/plan`
- `/workflow`
- `/workflows`
- workflow-profile slash commands such as plan/review/verify/debug/research/implement
- Lua extension commands
- skill commands

## Current CLI command surface

`crates/imp-cli/src/lib.rs` currently exposes:

- `chat` interactive CLI shell
- `tui`
- `view`
- `settings`
- `personality`
- `setup`
- `login`
- `secrets`
- `mana` behind feature
- `stats`
- `usage`
- `evidence`
- `eval`
- `import`
- `install-local`
- `web-login`

CLI mode flags include:

- `interactive`
- `chat`
- `rpc`
- `json`

## Surfaces that feel bloated/redundant

### Task-type slash commands

Remove from default product identity. Workflows or natural language should absorb these.

- `/plan`
- `/run`
- `/debug`
- `/review`
- `/verify`
- workflow-profile slash commands generally

### Legacy/durable-work overlaps

Remove or fold into workflows.

- `/mana`
- `/scope`
- mana run/status UI
- imp-work references
- standalone prototype/eval concepts
- improve mode

### Experimental or high-maintenance control surfaces

Likely remove unless explicitly justified:

- `/personality` and personality backend
- CLI chat shell
- `imp-gui` default build/member surface
- standalone `/memory` command and memory tool, unless retained as core project/user context
- `/autonomy`, unless replaced by settings/config only
- `/queue`, if `/loop` + `/stop` provide enough control
- `/clean`, if it exists only for improve sandbox cleanup
- `/checkpoints` and `/restore-checkpoint`, especially because restore is not wired in TUI
- `/export`, unless session export is actively valued
- `/hotkeys`, likely replace with help/settings docs

## Keep but tighten

### `/reload`

Keep because it supports minimal extensibility: reload config and Lua extensions.

### `/settings`, `/setup`, `/login`, `/secrets`

Keep because they support setup/config/auth and are valuable product controls.

### `/tree`, `/resume`, `/name`

Keep because session continuity is core to the TUI experience.

### `/loop`, `/stop`, `/compact`

Keep as runtime/session control primitives.

## Product risks to address

1. Command palette has become a junk drawer.
2. Workflow profiles create task-type slash commands that compete with workflows.
3. Durable orchestration nouns overlap: mana, workflow, run, scope, improve, prototype, eval.
4. CLI chat creates a second grammar and partial parity burden.
5. Personality has a large backend and UI for questionable value.
6. `imp-gui` increases build/product scope before core product is tight.
7. Docs present old experiments near current product concepts.
