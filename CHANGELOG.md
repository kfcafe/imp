# Changelog

All notable changes to `imp` are documented here.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). `imp` is actively developed, so entries should be concise, user-facing, and grouped by release.

## [Unreleased]

Use this section for changes that have landed on `main` but are not published yet.

Suggested categories:

- Added
- Changed
- Fixed
- Documentation
- Internal

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

[Unreleased]: https://github.com/kfcafe/imp/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/kfcafe/imp/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/kfcafe/imp/releases/tag/v0.1.0
