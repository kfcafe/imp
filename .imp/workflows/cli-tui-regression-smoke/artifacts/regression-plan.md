# CLI/TUI regression smoke plan

Purpose: run a narrow automated pass over the public surfaces HN readers are likely to try, without live provider calls or manual TUI interaction.

Commands:

1. `cargo fmt --check`
   - Ensures Rust formatting is stable across touched code.
2. `cargo check --workspace`
   - Confirms all workspace crates compile together after release-surface changes.
3. `cargo test -p imp-core --lib`
   - Covers core runtime, tools, policy, workflow, session, and runtime event behavior.
4. `cargo test -p imp-cli --lib`
   - Covers CLI parsing/headless/RPC helper behavior that can be tested without live providers.
5. `cargo test -p imp-tui --lib`
   - Covers TUI library tests without launching an interactive terminal.
6. CLI help smoke using built binary:
   - `cargo build -q -p imp-install`
   - `./target/debug/imp --help`
   - `./target/debug/imp tui --help`
   - `./target/debug/imp workflow --help`
   - `./target/debug/imp secrets --help`
   - `./target/debug/imp evidence --help`
7. `git diff --check`
   - Catches whitespace/errors in the current review diff.
8. `audit_scan` secrets/deps if available through the local tool runner.
   - Secrets scan catches accidental credential leakage.
   - Dependency scan records advisory state for release readiness.

Non-goals:

- No live provider/API-key calls.
- No manual full-screen TUI launch.
- No destructive git mutation.
