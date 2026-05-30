# CLI/TUI regression smoke results

Status: done_with_concerns

## Commands run

Initial full smoke command wrote `artifacts/regression-output.log`:

- `cargo fmt --check` — passed.
- `cargo check --workspace` — passed with existing ACP dead-code warnings.
- `cargo test -p imp-core --lib` — initially failed on stale workflow/mana-era tests.
- `cargo test -p imp-cli --lib` — passed: 72 tests.
- `cargo test -p imp-tui --lib` — initially failed on one stale workflow summary test.
- `cargo build -q -p imp-install` — passed with existing ACP dead-code warnings.
- CLI help smoke for root/tui/workflow/secrets/evidence — passed and produced help output.
- `git diff --check` — passed.

Fixes made from smoke failures:

- Removed stale imp-core tests for deleted workflow-bash-equivalent helper/behavior.
- Removed stale imp-tui workflow-create summary assertion that no longer matches the native workflow tool action surface.
- Reconciled stale config role-policy assertions with the current native workflow tool/action permissions.

Rerun evidence in `artifacts/regression-rerun-output.log`:

- `cargo fmt --check` — passed.
- `cargo test -p imp-core --lib` — passed: 930 passed, 1 ignored.
- `cargo test -p imp-tui --lib` — passed: 323 passed.
- `git diff --check` — passed.

Additional release scans:

- `audit_scan secrets` — passed; gitleaks found no leaks.
- `audit_scan deps` — completed with 4 known unresolved advisories and 0 fixable vulnerabilities. This matches the accepted dependency-audit disposition for 0.3.0.

## Remaining warnings / concerns

- `cargo check --workspace` and build emit ACP scaffold dead-code warnings. These are consistent with ACP being scaffold/internal for 0.3.0 and are not release-blocking for the HN surface.
- Dependency scan still reports 4 unknown-severity advisories with no fixed version (`bincode`, `fxhash`, `paste`, `yaml-rust`), already recorded in `docs/dependency-audit.md` as accepted for 0.3.0.
