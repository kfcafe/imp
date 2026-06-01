# Verification plan

- cargo fmt --check
- cargo check --workspace
- cargo test -p imp-core
- cargo test -p imp-cli
- cargo test -p imp-tui
- CLI smoke: cargo run -q -p imp-cli -- --version
- CLI smoke: cargo run -q -p imp-cli -- workflow validate prepare-imp-0-3-0-release
- CLI smoke: cargo run -q -p imp-cli -- list-models
- Docs grep for stale version, blocked ACP promises, TypeScript extension shipping claims
