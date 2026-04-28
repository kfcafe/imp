# imp-core

`imp-core` is the agent runtime crate for [imp](https://github.com/kfcafe/imp).

It contains the agent loop, tool registry, session persistence, context assembly, hooks, policy/mode enforcement, mana integration, and the early Rust SDK surface used by hosts that want to embed imp.

## What this crate provides

- tool-using agent loop
- native tool registry and execution policy
- append-only session records and branch metadata
- context assembly, compaction, and masking support
- hooks and extension integration points
- mana task execution support
- early `imp_core::sdk` host-facing API

## Intended use

Most users should install the `imp` binary through Homebrew/releases and use the terminal UI or CLI shell.

Use `imp-core` directly if you are building a Rust host around imp's runtime or experimenting with embedded agent sessions.

Minimal SDK shape:

```rust,no_run
use imp_core::sdk::{ImpSession, Result, SessionOptions};

#[tokio::main]
async fn main() -> Result<()> {
    let mut session = ImpSession::create(SessionOptions {
        cwd: std::env::current_dir()?,
        ..Default::default()
    })
    .await?;

    session.prompt("Summarize this repository.").await?;
    session.wait().await
}
```

## Status

The runtime is actively used by imp. The SDK is still early and may change as embedding/hostability work continues.

## Repository

- Main README: <https://github.com/kfcafe/imp>
- Crate: <https://crates.io/crates/imp-core>
