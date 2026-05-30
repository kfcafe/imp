# Target Surface

Status: draft target surface for `tighten-imp-product-surface`.

This artifact converts the product boundary into concrete retained, removed, deferred, internal, and folded surfaces.

## Target product surface

imp should launch as:

```text
TUI + one-shot + workflows + tools + skills + sessions + auth/config + Lua extensions
```

RPC is nice-to-have if low-cost. ACP is future/optional and should not shape current product cleanup.

## Default system prompt surface

Target one-line identity:

```text
You are imp, a practical and helpful software engineer. Use available tools, skills, and workflows when they help. Read files before editing them.
```

Prompt builder should append only obvious context:

- available tools
- available skills
- project instructions such as AGENTS.md
- generic prompt appendices such as `~/.imp/prompt.md` and `.imp/prompt.md`
- environment basics

Do not keep personality/soul, memory, project facts, guardrails, mode doctrine, or workflow doctrine as named default prompt-builder layers unless a later decision justifies them.

## Retained TUI commands

Keep as first-class TUI commands:

- `/new` — start a new session.
- `/resume` — resume/session picker; important.
- `/model` — switch model/provider.
- `/compact` — compact context.
- `/quit` — exit.
- `/loop` — continue/auto-loop current intent.
- `/stop` — stop active work/loop.
- `/reload` — reload config/extensions.
- `/setup` — setup wizard.
- `/secrets` — secrets manager.
- `/login` — provider login.
- `/name` — name current session.
- `/tree` — session tree.
- `/settings` — settings.

Decide before implementation:

- `/fork` — likely useful for session branching, but should be consciously kept or removed.
- `/copy` — useful UX affordance, but not yet confirmed core.
- `/status` — likely replace with visible TUI state rather than command; decide.

## Removed TUI command surface

Remove from default product surface:

- `/plan`
- `/run`
- `/debug`
- `/review`
- `/verify`
- workflow-profile slash commands generally
- `/improve`
- `/improve-safe`
- `/improve-merge`
- `/improve-help`
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

Rationale:

- task-type commands are workflow/natural-language concerns, not top-level commands.
- durable-work overlap should collapse into workflows.
- incomplete commands should not be visible.
- personality/memory/mana/eval/improve/prototype are old experiments or backend concepts.

Compatibility aliases are not required. Since imp is still pre-launch and historical data is markdown/files, removal can be direct after approval.

## Future workflow aliases

Future idea, explicitly out of scope for this workflow:

- `/work`
- `/workflow`
- `/workflows`

If added later, they should be aliases for a single durable workflow control surface. Do not add `/work` in this workflow.

## Minimal CLI surface

Keep:

```sh
imp
imp -p "..."
imp workflow run <id>
```

`imp workflow run <id>` is the only planned workflow CLI surface for now.

Do not start with workflow flags or broad subcommands. Avoid:

```sh
imp workflow list
imp workflow show
imp workflow validate
imp workflow update
imp workflow status
```

unless a real use case appears. Files and TUI can cover inspection initially.

Maybe keep:

```sh
imp --mode rpc
```

RPC is nice-to-have for launch if low-cost, but it should remain advanced/internal and not expand the public command surface.

Remove/archive:

```sh
imp chat
```

CLI chat is a second grammar and product surface; TUI + one-shot are enough.

## Model-facing tools

Keep canonical default model tools:

- ask
- bash
- edit
- git
- read
- write
- scan
- web
- workflow

Remove/fold candidates:

- mana tool — remove/isolate compatibility; workflow replaces durable work.
- prototype tool — fold into workflow artifacts/evidence.
- memory tool — remove command/product concept; reconsider optional prompt appendix/data only.

Configured shell tools and Lua tools can remain extension mechanisms if they do not bloat the default surface.

## Workflow surface

Keep:

- workflow files under `.imp/workflows/<id>/workflow.yaml`
- workflow validation
- workflow tool for model-facing lifecycle operations
- future workflow runner/subagent execution

Improve:

- workflow runner should own strict orchestration.
- ambient workflow controller should be defuncted for normal TUI/chat.
- workflow state should be workflow-native, not mana-root/unit based.

Fold into workflows:

- plan/debug/review/verify task type commands
- improve mode
- eval candidates
- prototype evidence
- mana/imp-work durable progress
- autonomous multi-step coding sessions
- future subagent work

## Prompt/runtime surface

Keep:

- one-line system identity
- tools in prompt
- skills in prompt
- AGENTS.md/project instructions
- generic prompt appendices
- environment basics

Remove from default prompt:

- personality/soul layer
- memory/user profile layer
- project facts/mana status
- guardrails layer
- verbose mode/role instructions
- workflow doctrine
- closeout status formatting requirements

Runtime-specific prompts should be scoped to explicit actions, especially workflow runner steps.

## Docs surface

Launch-facing docs should describe only:

- what imp is
- TUI use
- one-shot use
- workflows
- tools/skills
- Lua extensions
- auth/config
- optional RPC if retained

Move/archive or de-emphasize:

- mana-next
- imp-work
- improve mode
- eval candidate product docs
- prototype product docs
- GUI/wireframe docs
- planned ACP/MCP/sync/agents docs unless clearly roadmap
- old rebuild proposals

## Runtime surface

Keep:

- runtime events/state needed by TUI, one-shot, workflows, and low-cost RPC.
- provider/auth/config/secrets flows.
- session persistence and run artifacts only where useful.

Improve:

- add retention/global storage for `.imp/runs` eventually.
- reduce hidden continuation prompts.
- make workflow-run strictness explicit rather than ambient.

## Classification summary

Keep:

- TUI
- one-shot
- workflows
- canonical tools
- skills
- sessions
- auth/config/providers
- Lua extensions
- low-cost RPC/runtime internals

Remove:

- CLI chat
- personality command/editor/backend
- soul.md product concept
- imp-gui from default build surface
- visible mana/scope commands
- improve/eval/prototype/memory as product commands
- workflow-profile slash commands
- incomplete checkpoint restore command

Archive:

- GUI experiments/wireframes
- personality/soul design docs
- CLI chat design docs
- mana/imp-work compatibility docs
- old rebuild/proposal docs
- prototype/eval docs if product surfaces are removed

Fold into workflows:

- plan/run/debug/review/verify workflow profiles
- improve mode
- eval evidence
- prototype experiments
- mana/imp-work durable state
- future subagent execution

Internal/optional:

- RPC
- ACP future adapter work
- stats/usage/evidence commands if retained for development only
- memory as plain optional context files, not product command

## Decisions still needed

1. Keep or remove `/fork`?
2. Keep or remove `/copy`?
3. Keep `/status`, or rely on always-visible TUI state?
4. Keep RPC as launch-facing docs or internal advanced docs?
5. Delete soul/personality fully, or keep generic prompt appendices only? Current preference: generic appendices only.
6. Should memory survive only as plain files/context, or be removed entirely from prompt/product surface?
