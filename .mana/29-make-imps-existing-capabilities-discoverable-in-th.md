---
id: '29'
title: Make imp's existing capabilities discoverable in the TUI
slug: make-imps-existing-capabilities-discoverable-in-th
status: open
priority: 1
created_at: '2026-03-26T03:11:02.796218Z'
updated_at: '2026-03-26T03:11:02.796218Z'
labels:
- feature
- ux
- imp-tui
verify: 'cd /Users/asher/tower/imp && rg ''name: "memory"|name: "recall"|name: "plan"'' crates/imp-tui/src/views/command_palette.rs && rg ''"/memory|"/recall|"/plan'' crates/imp-tui/src/app.rs && rg ''persistent memory|past sessions|mana-backed planning|resume|fork'' crates/imp-tui/src/views/welcome.rs && cargo check -p imp-tui'
fail_first: true
kind: epic
---

Improve discoverability of important existing imp capabilities in the current TUI without adding new backend systems.

Do the following:

1. Add explicit discoverability commands to the command palette and slash-command list.
   - Add lightweight built-in entries for: `memory`, `recall`, and `plan`.
   - These commands do not need to implement new backend functionality.
   - They must produce concise, useful in-app guidance messages explaining what the capability is, how it fits into imp, and what the user should do next.

2. Implement handlers for those commands in `app.rs`.
   - `/memory` should explain that imp can keep persistent memory across sessions and that this is an existing runtime capability.
   - `/recall` should explain that imp can search past sessions / prior conversations.
   - `/plan` should explain that mana is the planning system in this stack: plans are represented as atomic mana units with dependencies, imp should use mana for decomposition/planning, and imp should not present planning as a separate local todo system.
   - Keep each response short, user-facing, and understandable without reading code.

3. Improve the welcome/setup flow copy.
   - Add a short capabilities section in the welcome flow summary step.
   - Mention at minimum: resumable/forkable sessions, persistent memory, recalling/searching past sessions, and mana-backed planning.
   - Keep it short, scannable, and written for a non-developer user.

4. Improve existing help text.
   - Update `/help` output in `app.rs` so it includes the new discoverability commands.
   - Describe them in plain English, not just by name.

5. Keep scope tight.
   - Do not implement checkpoints in this unit.
   - Do not implement LSP in this unit.
   - Do not add widget rendering in this unit.
   - Do not add a new local planning model or checklist system.
   - Do not add placeholder commands that do nothing; each new command must return a useful explanation message when invoked.

Desired outcome: a user opening imp’s TUI should be able to discover that imp remembers useful context across sessions, can help recall past work, can resume/fork sessions, and uses mana as the canonical planning/dependency system. The user should be able to learn this from the welcome/setup flow, the command palette, and `/help` without reading the README.
