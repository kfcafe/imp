# Slash Command Audit

Status: focused slash-command audit for `tighten-imp-product-surface`.

## Evidence inspected

Files:

- `crates/imp-tui/src/views/command_palette.rs`
- `crates/imp-tui/src/app.rs`

Key symbols/areas:

- `builtin_commands()`
- `merge_extension_commands()`
- `merge_skill_commands()`
- `merge_workflow_commands()`
- `CommandPalettePage::{Commands, Skills, Workflows}`
- `execute_command()`
- `try_workflow_command()`
- `try_skill_command()`
- `try_lua_command()`
- tests around command execution/palette entries

## Current command discovery model

The command palette currently merges:

1. built-in commands
2. extension commands
3. workflow profile commands
4. skill commands

Pages:

- Commands: built-ins + extensions
- Skills: skill commands
- Workflows: workflow profile commands

This is better than one flat list, but built-ins are still bloated and extension/profile/skill command surfaces can make `/` feel like a launcher for every experiment.

## Current built-in commands

`builtin_commands()` currently exposes:

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

Additional dispatch commands not all in built-ins:

- plan
- workflow
- workflows
- session
- welcome
- q
- mana-scope
- mem
- workflow profile names
- Lua extension commands
- skill commands

## Target retained built-ins

Keep in default built-in command list:

- new
- resume
- model
- compact
- quit
- loop
- stop
- reload
- setup
- secrets
- login
- name
- tree
- settings

Need user decision before code cut:

- fork
- copy
- status

Recommendation:

- keep `fork` only if session branching is actively valuable.
- keep `copy` if it saves frequent UI effort.
- remove `status` if runtime/session/workflow state is visible in TUI chrome/sidebar.

## Commands to remove from default built-ins and dispatch

### Remove/fold into workflows

- plan
- run
- workflow profile names such as debug/review/verify/implement/research
- improve
- improve-safe
- improve-merge
- improve-help
- eval

Rationale:

- task-type commands are workflow/natural language concerns.
- improve/eval/prototype concepts should fold into workflow evidence/execution.
- future `/workflow`/`/workflows` aliases can exist, but `/work` is out of scope now.

### Remove mana/compatibility commands

- mana
- scope
- mana-scope

Rationale:

- workflows replace mana as durable primitive.
- mana markdown/files remain historical, not product surface.

### Remove personality/memory product commands

- personality
- memory
- mem

Rationale:

- personality backend targeted for removal in favor of prompt appendices.
- memory should not be special prompt/product surface by default.

### Remove incomplete/defunct/non-core commands

- session
- checkpoints
- restore-checkpoint
- hotkeys
- export
- clean
- queue
- autonomy

Rationale:

- `session` is defunct compatibility.
- restore is not wired as real restore.
- hotkeys can be help/docs/settings if needed.
- export is non-core unless user confirms.
- clean mostly belongs to improve/sandbox cleanup.
- queue/autonomy are runtime internals/settings, not core slash commands.

## Commands to retain but perhaps rename/descope later

### loop/stop

Keep because user explicitly wants loop/stop and they are runtime control primitives.

But update descriptions:

Current `/loop` description says:

```text
Loop current mana work or a prompt (/loop [message|continue])
```

Target description should remove mana:

```text
Continue or loop the current request (/loop [message|continue])
```

Current `/stop` description says:

```text
Stop active imp work and clear pending/queued loop work
```

Target description can stay or become:

```text
Stop active work and clear queued loop prompts
```

### reload

Keep because it supports config/Lua extension reload.

Current description:

```text
Reload extensions
```

Target:

```text
Reload config and extensions
```

## Prefix/dispatch concern

`execute_command()` canonicalizes and dispatches a large command match. Prior audit found prefix behavior around command palette execution can be surprising when command set is large.

Recommendation:

- After tightening, require exact command names for execution.
- If prefix completion remains, only allow unique visible built-in matches.
- Do not let skills/workflows/extensions change built-in prefix behavior.

## Backend touchpoints by command group

### Improve

Files/functions:

- `crates/imp-tui/src/app.rs`
  - `set_improve_mode`
  - `improve_merge_command`
  - `workflow_context_prompt_for_request` improve context
- `crates/imp-core/src/config.rs` improve budget/settings

Action:

- remove commands first.
- later fold useful worktree/sandbox logic into workflow runner.

### Eval

Files/functions:

- `crates/imp-tui/src/app.rs` `eval_candidate_command`
- `crates/imp-core/src/eval_candidate.rs`
- `crates/imp-core/src/eval_candidate_closeout.rs`
- `crates/imp-cli/src/lib.rs` eval CLI

Action:

- remove TUI slash command first.
- decide whether CLI eval remains internal.
- fold useful capture into workflow evidence.

### Mana/scope/run

Files/functions:

- `crates/imp-tui/src/app.rs`
  - `set_active_mana_scope`
  - active run/scope/status handling
  - workflow/mana command descriptions
- `crates/imp-core/src/tools/mana.rs`
- `crates/imp-core/src/mana_*`

Action:

- remove visible commands.
- isolate backend/feature later.

### Workflow profiles

Files/functions:

- `crates/imp-core/src/workflow_profiles.rs`
- `crates/imp-tui/src/app.rs` `try_workflow_command`
- `crates/imp-tui/src/views/command_palette.rs` `merge_workflow_commands`

Action:

- stop surfacing profile names as slash commands by default.
- keep profile registry only if workflow runner uses it internally.

### Skills and Lua extension commands

Files/functions:

- `merge_skill_commands`
- `merge_extension_commands`
- `try_skill_command`
- `try_lua_command`

Action:

- keep as extensibility, but avoid making default command page noisy.
- consider Skills/Extensions pages only, not intermingled with core commands.
- skills should appear in prompt, not necessarily as top-level slash commands.

### Personality/memory

Files/functions:

- `open_personality`
- `handle_personality_key`
- `save_personality`
- `handle_memory_command`
- `crates/imp-core/src/personality.rs`
- `crates/imp-core/src/memory.rs`

Action:

- remove commands with backend removal phases.

### Checkpoints/restore

Files/functions:

- `checkpoints` command lists file checkpoints.
- `restore-checkpoint` currently reports restore is not wired.

Action:

- remove from product surface until real restore exists.
- keep checkpoint backend if sessions/runtime still need passive checkpoints.

## Test impact

Existing TUI tests assert old commands exist/execute:

- command list contains `mana`
- command list contains `checkpoints`
- command list contains `restore-checkpoint`
- `execute_command("personality")`
- `execute_command("memory")`
- `execute_command("workflows")`
- `execute_command("mana")`

Expected implementation test updates:

- replace palette assertions with retained command list.
- add assertions removed commands are absent.
- keep execution tests only for retained commands.
- remove or rewrite personality/memory/mana/workflows tests when backend removal/alias decision lands.

## Proposed implementation steps

1. Add explicit retained built-in command list in `builtin_commands()`.
2. Update retained command descriptions to remove mana/improve language.
3. Stop merging workflow profile commands into command palette by default.
4. In `execute_command()`, remove removed commands or have them fall through as unknown after backend deletion.
5. Keep Lua/skill execution only after exact command lookup or page-specific selection.
6. Update tests around command list and removed commands.

## Open decisions

1. Is `/fork` retained?
2. Is `/copy` retained?
3. Is `/status` retained?
4. Should `/workflow` and `/workflows` remain temporarily before `/work` exists, or be removed until the future workflow command surface is designed?
5. Should skills remain slash-invokable, or only prompt-listed/readable?

Current recommendation:

- keep skills in prompt; decide separately whether slash-invokable skills are worth command-palette complexity.
- keep Lua extension commands because Lua is shipped extensibility.
- remove workflow profile slash commands from default discovery.
- defer `/workflow`/`/workflows` until `imp workflow run <id>` and TUI workflow affordances are clarified.
