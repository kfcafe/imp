# Changelog

All notable changes to `imp` are documented here.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). `imp` is actively developed, so entries should be concise, user-facing, and grouped by release.

## [Unreleased]

Use this section for changes that have landed on `main` after the 0.1.3 draft.

### Added

- Added GitHub as a `web` search/read provider and wired GitHub credential setup through app surfaces.
- Added constrained `--print` worker controls for allowing/denying tools and write paths, plus JSON final output.

### Changed

- Prioritized current-project sessions in the session picker.
- Improved native mana run responsiveness during direct run orchestration.

## [0.1.3] - Unreleased

### Added

- Added a native `worktree` tool for creating, listing, and removing git worktrees separately from the general `git` tool.
- Removed defunct `ask_agent` helper-agent tool; use mana units/runs for durable delegation.
- Added `ask_user` schema support, including TUI multi-select prompts.
- Added mana `guide` and `template` actions so agents can inspect task/epic/decision/verify/orchestration guidance from the native mana tool.
- Added validation hints to mana actions to make schema errors and missing arguments easier for agents to recover from.
- Added skill-provided slash command discovery and command-palette display for Lua slash commands.
- Added `/update-imp` as the local Lua updater command for bumping, pushing, installing, and verifying nightly imp builds.

### Changed

- Renamed the `shell` tool to `bash` and simplified secret injection for shell commands.
- Simplified `git`, `scan`, `edit`, and related tool schemas and metadata so agents receive cleaner, more bounded tool interfaces.
- Made anchored edits a first-class edit mode and added explicit line-range support to `read`.
- Made TUI `/compact` run asynchronously with visible progress instead of blocking the interface.
- Made Lua slash commands run with the same background progress interaction as `/compact` instead of blocking the TUI.
- Strengthened mana worker context assembly and improved mana unit closure reliability after implementation.
- Switched project licensing metadata and repository license text to MPL-2.0.

### Fixed

- Hardened web read/search schema handling, URL read safety, and YouTube metadata/transcript extraction paths.
- Added clearer write overwrite behavior without expanding the write schema surface.
- Removed stale spawn references after the helper-agent redesign.
- Improved prompt wrapping, selected-tool indicators, tool-call inspector styling, and startup version display in the TUI.
- Deduplicated Lua extension loading and startup command surfacing so commands such as `/update-imp` appear once.
- Fixed Lua command reload policy handling and argument execution so reloaded extensions preserve configured capabilities and pass argv without shell joining.

### Documentation

- Documented Lua slash command behavior in the README.
- Added skill-writing reference updates.

### Internal

- Continued schema hardening across core tools, including `read`, `write`, `edit`, `scan`, `git`, `bash`, `web`, `ask_user`, and mana.

## [0.1.2] - 2026-04-28

### Fixed

- Deduplicated discovered instruction files so identical global `agents.md`/`AGENTS.md` content is not injected repeatedly into trivial prompts.
- Avoided reinjecting the same global `.imp/agents.md` when it is also discovered while walking ancestor project directories.

## [0.1.1] - 2026-04-28

### Documentation

- Expanded crate-level README files for published crates:
  - `imp-llm`
  - `imp-core`
  - `imp-lua`
  - `imp-tui`
  - `imp-cli`
- Added crate-specific descriptions, intended-use notes, status sections, and repository links for crates.io readers.

## [0.1.0] - 2026-04-28

Initial crates.io release of the imp crate family.

### Added

- Published `imp-llm`, the provider and streaming client layer.
- Published `imp-core`, the agent runtime with tools, sessions, context, hooks, mana integration, and early SDK surface.
- Published `imp-lua`, the Lua extension runtime.
- Published `imp-tui`, the fullscreen terminal interface.
- Published `imp-cli`, the command-line entrypoint and `imp` binary crate.

### Features

- Terminal UI and CLI chat entrypoints.
- Native tools for file I/O, editing, shell commands, git, structural code scanning, web reading/search, user prompts, memory, and mana coordination.
- Durable JSONL sessions with branch metadata and context compaction support.
- Provider support for Anthropic, OpenAI, Google, and OpenAI-compatible services.
- Secure credential storage through OS credential stores.
- Agent modes for full, worker, orchestrator, planner, reviewer, and auditor workflows.
- Lua extension support for tools, commands, and hooks.
- Direct mana task execution through `imp run <unit-id>`.

### Documentation

- Reworked the main README to position imp as an extensible coding agent built for efficiency and performance.
- Added plain-language documentation for mana as included durable task coordination.
- Documented install, quick start, tools, sessions, providers, customization, architecture, and CLI reference.

### Internal

- Standardized published crates on MIT license metadata.
- Added crates.io metadata and versioned internal dependencies for published crates.

[Unreleased]: https://github.com/kfcafe/imp/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/kfcafe/imp/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/kfcafe/imp/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/kfcafe/imp/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/kfcafe/imp/releases/tag/v0.1.0
