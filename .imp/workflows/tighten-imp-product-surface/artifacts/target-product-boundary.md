# Target Product Boundary

Status: draft target boundary for `tighten-imp-product-surface`.

## Product thesis

imp should be a minimal, extensible coding agent:

```text
TUI + one-shot + workflows + tools + skills + sessions + auth/config + Lua extensions
```

RPC and ACP are launch-adjacent but not required for the first tight product cut.

- RPC: nice to keep launch-ready if low-cost because it is already implemented and documented.
- ACP/editor adapters: future/optional; do not shape the cleanup unless they fall out naturally from RPC/runtime boundaries.

## Core identity

Default system prompt target:

```text
You are imp, a practical and helpful software engineer. Use available tools, skills, and workflows when they help. Read files before editing them.
```

This prompt should be the identity layer, not a long doctrine block.

## First-class surfaces

### TUI — keep

Primary human-facing product.

Retained TUI concepts:

- chat/editor
- sessions
- resume/new/name/tree
- model switching
- compacting
- loop/stop runtime control
- setup/login/secrets/settings
- reload for config/extensions

### One-shot prompt — keep

Primary non-interactive human/script path.

Keep:

- `imp -p ...` / print mode
- headless event/runtime path where it supports one-shot/workflow execution

### Workflows — keep and center

Workflow is the durable orchestration primitive.

Workflows should own:

- plans
- steps
- checks
- blockers
- decisions
- artifacts
- results
- durable progress
- future long-running/subagent execution

Workflows should replace old visible concepts:

- plan/run/debug/review/verify slash commands as separate product commands
- improve mode
- eval candidate product surface
- standalone prototype flow
- mana/imp-work visible durable-work concepts

### Tools — keep tight default set

Keep canonical model-facing tools:

- ask
- bash
- edit
- git
- read
- write
- scan
- web
- workflow

Extra tools/modules should justify themselves against this core.

### Skills — keep in prompt

Skills should appear in the prompt because they are part of the minimal extensibility story.

Target behavior:

- list available skill names and concise descriptions
- model reads the skill file when useful
- no special prompt modes around skills

### Sessions — keep

Keep:

- session persistence
- resume
- tree
- name
- fork only if confirmed valuable
- copy/export only if confirmed valuable

### Auth/config/providers — keep

Keep:

- setup
- login
- secrets
- settings
- model/provider config

### Lua extensions — keep

Lua is current shipped extension support. Keep `/reload` for config/extensions.

Do not describe TypeScript extensions as shipped unless the repo implements them as product-ready.

### Runtime/RPC/ACP — classify carefully

RPC:

- implemented in `crates/imp-cli/src/lib.rs` as `--mode rpc` / JSONL.
- documented in README and `docs/rpc.md`.
- keep if low-cost, but it is not allowed to expand the core command/product surface.

ACP/editor adapters:

- planned/future in README/docs.
- classify as future/optional; remove from launch-facing README if not implemented.

Runtime event/state APIs:

- potentially valuable for TUI/RPC/future GUI/ACP.
- keep as internal/runtime infrastructure if not causing product bloat.

## Explicitly non-core / likely remove or archive

### CLI chat shell

Remove. TUI and one-shot are enough.

### imp-gui

Remove from default members. Decide later whether to archive/delete the crate.

### Personality/soul product concept

Remove `/personality`, personality backend, sliders, and soul editor.

Replacement:

- generic prompt appendix files, e.g. `~/.imp/prompt.md` and `.imp/prompt.md`.
- users can write any “soul” content there if they want.

### Mana/imp-work compatibility product surface

Remove from default user-facing surface. Fold durable value into workflows.

### Improve/eval/prototype/memory as product commands

- improve: fold into workflow execution policy if useful.
- eval: fold into workflow evidence or keep as internal dev tool only.
- prototype: fold into workflow artifacts/evidence; likely remove standalone tool.
- memory: remove slash command; reconsider as ordinary prompt appendix/context data, not product surface.

### Ambient workflow controller

Do not keep as normal TUI/chat leash.

Target:

- normal TUI/chat trusts the model and stays natural.
- strict workflow execution moves into explicit workflow runner/action.

## Candidate retained TUI commands

Retain:

- `/new`
- `/resume`
- `/model`
- `/compact`
- `/quit`
- `/loop`
- `/reload`
- `/setup`
- `/secrets`
- `/login`
- `/name`
- `/tree`
- `/settings`
- `/stop`

Decide:

- `/fork`
- `/copy`
- `/status`

Remove from default product surface:

- `/plan`
- `/run`
- `/debug`
- `/review`
- `/verify`
- `/workflow-profile commands`
- `/improve*`
- `/eval`
- `/autonomy`
- `/clean`
- `/queue`
- `/scope`
- `/mana`
- `/memory`
- `/checkpoints`
- `/restore-checkpoint`
- `/hotkeys`
- `/export`
- `/session`
- `/personality`

Future alias idea, out of scope now:

- `/work`, `/workflow`, `/workflows` as aliases for durable workflow control.

## Minimal CLI product surface

Keep:

```sh
imp                 # TUI
imp -p "..."        # one-shot prompt
imp workflow run <id>
```

Maybe keep:

```sh
imp --mode rpc
```

Remove or archive:

```sh
imp chat
```

Avoid broad workflow CLI surface initially. `imp workflow run <id>` is enough.

## Product boundary rule

For every feature/module, ask:

1. Does it support TUI, one-shot, workflows, tools, skills, sessions, auth/config, Lua extensions, or low-cost RPC?
2. If yes, is it simpler as workflow-native behavior or prompt appendix config?
3. If no, remove or archive it.
4. If it is only historical/design value, archive to `~/imp-archive` or `docs/archive`.
