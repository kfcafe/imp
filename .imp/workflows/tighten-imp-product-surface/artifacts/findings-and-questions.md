# Findings and Discussion Packet

Status: discussion summary for `tighten-imp-product-surface`.

This is the condensed readout from the audit/planning artifacts.

## Target product thesis

imp should launch as:

```text
TUI + one-shot + workflows + tools + skills + sessions + auth/config + Lua extensions
```

RPC is nice-to-have if low-cost. ACP is future/optional.

## Strongest audit findings

### 1. Bloat is mostly product/runtime surface, not core tools

The default model-facing tools are reasonably tight:

- ask
- bash
- edit
- git
- read
- write
- scan
- web
- workflow

The bloat comes from slash commands, CLI chat, personality/soul, mana/imp-work compatibility, improve/eval/prototype/memory concepts, prompt layers, docs, and ambient workflow runtime behavior.

### 2. The system prompt should become radically smaller

Target one-liner:

```text
You are imp, a practical and helpful software engineer. Use available tools, skills, and workflows when they help. Read files before editing them.
```

Prompt builder should append only obvious context:

- tools
- skills
- project instructions
- prompt appendices
- environment

Avoid config knobs for old internal concepts like personality, memory, project facts, guardrails, mode doctrine, or workflow doctrine. If those are still needed, they should become task-specific/runtime-specific prompts, not default global prompt layers.

### 3. Workflows should be powerful, but explicit

Normal TUI/chat should be natural and model-trusting.

Strict workflow behavior should live in explicit workflow execution:

```sh
imp workflow run <id>
```

Long-term, this command becomes the home for automation, cron, and subagents.

### 4. The ambient workflow controller is the wrong shape

The current controller is useful in spirit but currently:

- ambient in normal agent turns
- mana-root/unit shaped
- injects hidden continuation prompts
- can override final closeout status

Target: defunct ambient controller for normal TUI/one-shot, move strictness into workflow runner.

### 5. CLI workflow surface should stay tiny

Initial CLI surface:

```sh
imp workflow run <id>
```

No flags initially. No broad workflow subcommands unless proven necessary.

### 6. CLI chat should go

TUI and one-shot cover the product. CLI chat adds:

- second command grammar
- slash compatibility burden
- partial feature parity
- more code in `imp-cli/src/lib.rs`

Target: remove/archive CLI chat.

### 7. Personality/soul should be removed as product/backend concept

Replace with generic prompt appendices:

```text
~/.imp/prompt.md
.imp/prompt.md
```

If a user wants “soul,” they can write it there. No `/personality`, sliders, soul editor, or personality backend.

### 8. Mana/improve/eval/prototype/memory should collapse or leave product surface

- mana/imp-work: fold durable value into workflows; remove visible commands/docs.
- improve: fold useful sandboxing/worktree ideas into workflow execution policy later.
- eval: fold useful failure capture into workflow evidence or keep internal only.
- prototype: fold into workflow artifacts/evidence; likely remove standalone tool.
- memory: remove slash command; decide whether any plain file/context support remains.

### 9. imp-gui should leave default build

First low-risk implementation: remove `imp-gui` from `default-members`; decide archive/delete later.

### 10. Docs/root need active-vs-archive split

README and active docs should only describe current product. Old rebuild/mana/imp-work/personality/eval/prototype/GUI/future-extension docs should move to archive or roadmap.

## Proposed first implementation batch

After approval:

1. Remove `imp-gui` from default workspace members.
2. Tighten visible TUI command palette to retained commands.
3. Introduce minimal prompt builder shape / reduce default prompt toward one-liner.
4. Add minimal `imp workflow run <id>`.
5. Remove CLI chat.
6. Remove personality UI/backend.

Reason:

- Starts with visible/product-value cleanup.
- Avoids deep mana/controller deletion until workflow runner seam exists.
- Builds the replacement prompt/workflow architecture before removing large cross-cutting backends.

## Decisions needed before implementation

### Command decisions

1. Keep `/fork`?
2. Keep `/copy`?
3. Keep `/status`, or rely on always-visible TUI state?

Current recommendation:

- keep `/fork` only if you actively branch sessions.
- keep `/copy` only if it saves frequent manual UI work.
- remove `/status` if TUI can show active state directly.

### Runtime/API decisions

4. RPC: launch-facing advanced docs or internal only?

Current recommendation:

- keep RPC if low-cost, but document as advanced/internal.
- ACP remains future/optional.

### Prompt/memory decisions

5. Confirm removal of personality/soul product concept in favor of prompt appendices?
6. Should memory be removed entirely from prompt/product surface, or kept only as plain optional context files?

Current recommendation:

- remove personality/soul fully.
- remove `/memory`; do not inject memory by default.
- if memory survives, make it ordinary appendix/context, not a special command/tool.

### Archive decisions

7. Prefer `~/imp-archive` or in-repo `docs/archive` for old docs?

Current recommendation:

- use `~/imp-archive` for root clutter, GUI prototypes, old rebuild/mana/personality/eval docs.
- keep only a short active roadmap/archive index in repo if useful.

## Current workflow status

Completed planning artifacts:

- `codebase-audit.md`
- `prompt-audit.md`
- `surface-inventory.md`
- `backend-inventory.md`
- `target-product-boundary.md`
- `target-surface.md`
- `workflow-controller-review.md`
- `cli-workflow-surface.md`
- `archive-plan.md`
- `backend-cut-sequence.md`

Still pending by design:

- user approval on removal policy
- implementation checks
- docs updates
- actual code cuts
- final verification
