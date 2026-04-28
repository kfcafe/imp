# imp-llm

`imp-llm` is the provider and streaming client layer used by [imp](https://github.com/kfcafe/imp).

It contains the model/provider abstraction, streaming response handling, OAuth/API-key auth support, model metadata, and provider-specific request/response code for the imp coding agent.

## What this crate provides

- streaming LLM client primitives
- provider implementations for Anthropic, OpenAI, Google, and compatible APIs
- model registry and model alias handling
- OAuth and API-key authentication helpers
- response/event normalization for higher-level agent runtimes
- provider-specific error/context handling

## Intended use

Most users should install and run the `imp` binary rather than depend on this crate directly:

```bash
brew tap kfcafe/tap && brew install imp
imp
```

Use `imp-llm` directly if you want the lower-level Rust client pieces without the full agent loop, TUI, sessions, and tools from `imp-core` / `imp-cli`.

## Status

This crate is an active surface of imp, but APIs may still evolve as provider behavior, model capabilities, and the imp runtime change.

## Repository

- Main README: <https://github.com/kfcafe/imp>
- Crate: <https://crates.io/crates/imp-llm>
