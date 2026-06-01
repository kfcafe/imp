# Install smoke results

Status: done

## Install command used

Used an isolated Cargo home to avoid overwriting the developer's active `imp` binary:

```sh
rm -rf /tmp/imp-install-smoke-cargo-home
mkdir -p /tmp/imp-install-smoke-cargo-home
CARGO_HOME=/tmp/imp-install-smoke-cargo-home cargo install --path . --force --locked
```

## Smoke command results

Full output: `artifacts/install-smoke-output.log`.

Passed:

- Source install completed successfully.
- Installed binary path: `/tmp/imp-install-smoke-cargo-home/bin/imp`.
- `/tmp/imp-install-smoke-cargo-home/bin/imp --version` printed `imp 0.3.0`.
- Help smoke passed for:
  - root `--help`
  - `tui --help`
  - `workflow --help`
  - `evidence --help`
  - `secrets --help`

Notes:

- Build emitted ACP scaffold dead-code warnings, already documented as accepted for 0.3.0 because ACP is scaffold/internal.
