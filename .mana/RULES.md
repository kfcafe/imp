# imp — Project Rules

## Architecture
- Rust workspace with 5 crates: imp-llm, imp-core, imp-lua, imp-tui, imp-cli
- Dependency direction: imp-llm ← imp-core ← imp-lua ← imp-tui ← imp-cli
- imp-llm has zero dependencies on imp-core or any other imp crate
- Library crates use `thiserror` for typed errors; binaries use `anyhow` at boundaries

## Code Style
- `cargo fmt` before committing
- `cargo clippy` must pass (warnings OK during development, errors never)
- Self-documenting code; comments explain *why*
- All pub types/functions need doc comments
- Tests go in `#[cfg(test)] mod tests` at the bottom of each file

## Conventions
- Tool implementations go in `imp-core/src/tools/`, one file per tool
- Tools implement the `Tool` trait from `imp-core/src/tools/mod.rs`
- Provider implementations go in `imp-llm/src/providers/`, one file per provider
- Providers implement the `Provider` trait from `imp-llm/src/provider.rs`
- All tool output must be truncated (50KB / 2000 lines max) using helpers from `tools/mod.rs`
- Use `imp_llm::now()` for timestamps, not `std::time`

## Build & Test
- Build from tower root: `cd /Users/asher/tower && cargo check -p <crate>`
- Test from tower root: `cd /Users/asher/tower && cargo test -p <crate>`
- The workspace root is `/Users/asher/tower/Cargo.toml`
- Never change workspace-level deps without explicit approval

## Forbidden
- No `.unwrap()` in library code (use `?` or handle errors)
- No `println!` — use events/channels for agent output
- No blocking I/O in async contexts — use tokio equivalents
- Don't change the Provider trait, Tool trait, or message types without cross-crate impact analysis
