# imp-cli

`imp-cli` is the command-line entrypoint crate for [imp](https://github.com/kfcafe/imp).

It builds the `imp` binary and wires together the terminal UI, CLI chat shell, one-shot prompt mode, auth/setup commands, secrets commands, mana task execution, import helpers, and RPC/headless entrypoints.

## What this crate provides

- `imp` binary entrypoint
- terminal UI launch
- CLI chat shell
- one-shot prompt execution
- login/auth and secrets commands
- web-provider credential setup
- direct mana task execution via `imp run <unit-id>`
- extension import helpers such as `imp import --from pi`
- headless/RPC-oriented command paths

## Intended use

This is the crate most users install when they want the imp binary:

```bash
cargo install imp-cli
imp
```

Homebrew/release installs are usually preferred for end users:

```bash
brew tap kfcafe/tap && brew install imp
```

## Status

The CLI is an active user-facing surface. Some headless/RPC-oriented paths are still evolving; the normal terminal UI, CLI chat, auth/secrets, and direct mana execution workflows are the primary supported surfaces.

## Repository

- Main README: <https://github.com/kfcafe/imp>
- Crate: <https://crates.io/crates/imp-cli>
