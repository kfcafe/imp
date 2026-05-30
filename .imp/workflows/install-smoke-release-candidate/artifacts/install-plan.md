# Install smoke plan

Purpose: verify a new user can install the current 0.3.0 release candidate from source and run basic first commands, without publishing to Homebrew/crates.io or requiring provider credentials.

Commands:

1. Install from the current checkout into a temporary Cargo home so this smoke test does not overwrite the user's active `imp` binary:

```sh
rm -rf /tmp/imp-install-smoke-cargo-home
mkdir -p /tmp/imp-install-smoke-cargo-home
CARGO_HOME=/tmp/imp-install-smoke-cargo-home cargo install --path . --force --locked
```

2. Smoke the installed binary:

```sh
/tmp/imp-install-smoke-cargo-home/bin/imp --version
/tmp/imp-install-smoke-cargo-home/bin/imp --help
/tmp/imp-install-smoke-cargo-home/bin/imp tui --help
/tmp/imp-install-smoke-cargo-home/bin/imp workflow --help
/tmp/imp-install-smoke-cargo-home/bin/imp evidence --help
/tmp/imp-install-smoke-cargo-home/bin/imp secrets --help
```

3. Record binary location and help output line counts.

Rationale: the workflow's original command used `cargo install --path . --force && imp --version && imp --help`, but using an isolated `CARGO_HOME` is safer because it validates source install without replacing the developer's real command on PATH.
