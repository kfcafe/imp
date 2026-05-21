# CLI-first Interactive Shell Path

This document defines the default CLI-first interactive path for imp. It is transcript-oriented, not fullscreen-oriented: the shell should feel like a serious terminal tool with command mode, chat mode, guided flows, and explicit handoff to richer viewers when browsing is the job.

## Scope

The shell path covers plain interactive `imp` after the default-entrypoint migration and explicit `imp chat` during the migration. It does not redesign deep fullscreen TUI layout.

The unresolved default-entrypoint question is intentionally explicit: `imp chat` is the safe explicit command during migration, while plain `imp` should only become this shell after readiness criteria are met. The migration docs should continue to use explicit `imp chat` / `imp view` / `imp tui` until the default-entrypoint flip is approved.

## Core behavior

- Input is line-oriented by default: each submitted line starts a chat turn unless it is a shell command.
- Multiline input uses an explicit compose mode with visible continuation prompts, not hidden escape rules.
- Responses stream into the transcript as streamed responses arrive.
- Tool activity appears as compact inline notices, not pane-heavy sidebars.
- Ask/confirm/select/input requests appear inline in the transcript and accept normal terminal input.
- Each turn ends with terse turn summaries: tool count, verify/check status when relevant, token/cost/time if available.
- Sessions persist across turns using the same session substrate as current imp surfaces.
- A lightweight status/widget row appears near the prompt for model/thinking/session/project/unit state.

## Prompt shape

Example default rhythm:

```text
project imp · model gpt-5.4 · thinking medium · session cli-shell-2026-04-09
imp> explain the current mana worker path
thinking…
• tool read crates/imp-core/src/mana_worker.rs
The mana worker path loads a unit assignment, builds runtime context, runs the agent loop, and reports a structured worker result…
✓ turn complete · 1 tool · 8.4s
```

The status row should be compact and optional in non-interactive or narrow terminals. It may include active mana unit/run context when relevant.

## Multiline input

Multiline compose should be visible:

```text
imp> :paste
...> Write a migration plan with:
...> - compatibility shims
...> - rollback points
...> - verify gates
...> :submit
```

Required behavior:

- visible secondary prompt such as `...>`;
- explicit submit/cancel commands;
- no accidental send on intermediate newlines;
- preserve pasted formatting.

## Inline interactions

Shared UI requests should degrade naturally into terminal prompts:

```text
? Apply this edit? [y/N] y
```

```text
Select model:
  1. gpt-5.4-mini
  2. claude-sonnet
Select> 1
```

```text
Enter api_key> ********
```

Notifications should be terse:

```text
! warning: verify failed; restore checkpoint available via :checkpoints
```

## Command namespace

The canonical shell command prefix is `:`. Temporary `/command` compatibility may remain during migration for existing TUI muscle memory.

### Session/state

- `:new`
- `:resume [query]`
- `:fork`
- `:name <name>`
- `:export [path]`
- `:tree` or `:history`

### Runtime controls

- `:model [name]`
- `:thinking [level]`
- `:compact`
- `:reload`
- `:status`
- `:quit`

### Setup, config, and identity

- `:settings [field] [value]`
- `:personality [show|edit|set ...]`
- `:login [provider]`
- `:secrets [provider]`
- `:setup`

These must be first-class in the shell. CLI-first must not regress settings/personality/auth ergonomics or require fullscreen UI for important setup/control capabilities.

### Memory and checkpoints

- `:memory ...`
- `:checkpoints`
- `:restore-checkpoint <id-or-label>`

Checkpoint restore semantics should stay conservative where current support is inspect-first.

### Utility, files, and help

- `:help [topic]`
- `:commands`
- `:copy [target]`
- `:files [query]`
- useful `@file` inclusion behavior should remain where practical.

### Viewer and TUI handoff

- `:view [logs|tree|session|checkpoints|tools]`
- `:tui [settings|personality|session|logs]`

Browsing-heavy work should hand off to `imp view`; fullscreen-specific affordances should stay in `imp tui`.

### Extension commands

Lua/extension commands should remain discoverable and callable in the shell namespace, ideally through `:commands` and help/completion. The shell should not make extension commands TUI-only.

## Guided flows

Simple controls should stay inline. More complex operations should become guided CLI flows or open an editor/viewer.

Examples:

```text
imp> :settings
Settings
  1. Model provider
  2. Default model
  3. Thinking level
  4. Tool policy
Choose field> 2
Default model> gpt-5.4-mini
Save changes? [Y/n] y
✓ settings saved
```

```text
imp> :personality edit
Opening $EDITOR for project personality…
✓ personality updated
```

```text
imp> :setup
Welcome to imp setup.
Provider login:
  1. OpenAI
  2. Anthropic
Select> 1
Enter api_key> ********
✓ provider configured
```

These flows preserve capability parity with current richer surfaces without making fullscreen mode mandatory.

## What the default shell intentionally omits

The default shell should not recreate fullscreen TUI chrome inline. Omit:

- persistent sidebars;
- multi-pane layout;
- file tree panes;
- session browsers embedded in scrollback;
- complex checkpoint diff browsers;
- hotkey-driven overlay stacks;
- live render caches and viewport mechanics.

Those belong in `imp view` or optional `imp tui`.

## Comparison with fullscreen TUI

The fullscreen TUI remains valuable for dense browsing, visual session inspection, settings/personality panels, and workflows that benefit from persistent panes. The CLI shell is the default authoring/control surface: it prioritizes speed, scriptable mental model, readable transcripts, and terminal-native interaction.

## Output examples

### Tool notices and summary

```text
imp> fix the failing codeintel test
• tool bash cargo test -p imp-core codeintel -- --nocapture
✗ check failed: 1 test failed
• tool edit crates/imp-core/src/codeintel/mod.rs
• tool bash cargo test -p imp-core codeintel -- --nocapture
✓ check passed: 11 tests
Fixed the failing registry test by avoiding Debug formatting on a trait object.
✓ turn complete · 3 tools · verify passed · 42s
```

### Handoff to view

```text
imp> :view checkpoints
Opening imp view checkpoints for current session…
```

### Auth flow

```text
imp> :login openai
OpenAI API key is not configured.
Enter api_key> ********
✓ OpenAI configured for this machine
```

## Implementation boundary

- Shell owns transcript command parsing, prompt loop, inline rendering, and guided terminal flows.
- `imp-core` owns shared request/event semantics and runtime behavior.
- `imp view` owns browsing-heavy inspection.
- `imp tui` owns fullscreen layout and overlays.
- CLI direct subcommands should share capability implementations with shell commands where practical.
