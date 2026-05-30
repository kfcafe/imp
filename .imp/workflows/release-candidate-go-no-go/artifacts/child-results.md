# Child workflow results review

Reviewed HN release readiness child workflows for imp 0.3.0.

## Completed child workflows

- `archive-stale-history` — done.
  - Archived stale launch-confusing root/docs history to `~/imp-archive/stale-history-2026-05-29`.
  - Current public docs retained.
  - Verification included targeted stale-link grep, `cargo fmt --check`, and `cargo check --workspace`.
- `dependency-advisory-cleanup` — done.
  - Fixed the only fixable advisory by upgrading `ratatui` and removing old `lru`.
  - Accepted 4 remaining no-fixed-version advisories for 0.3.0.
- `remove-mana-vocabulary` — done.
  - Removed active legacy mana public surface and verified active source/README/docs index clean.
- `public-surface-final-audit` — done.
  - Captured command surface and fixed stale public-surface claims.
- `docs-hn-readiness` — done.
  - Updated README install/quickstart/status/technical docs links.
  - Targeted stale docs scan clean.
- `cli-tui-regression-smoke` — done_with_concerns.
  - Initial failures exposed stale tests; fixed them.
  - Rerun passed `cargo fmt --check`, `cargo test -p imp-core --lib`, `cargo test -p imp-tui --lib`, and `git diff --check`.
  - `cargo check --workspace`, `cargo test -p imp-cli --lib`, build, CLI help smoke, secrets scan, and deps scan were also recorded.
- `install-smoke-release-candidate` — done.
  - Isolated `cargo install --path . --force --locked` succeeded.
  - Installed binary reports `imp 0.3.0`; help smoke passed.
- `provider-auth-smoke` — done.
  - Model list, login/secrets help, secrets list/doctor, and isolated no-key error path checked.
  - Artifact secret-pattern scan clean.

## Final blockers

No release blockers found in completed child workflow evidence.

## Accepted concerns

- ACP scaffold dead-code warnings remain during workspace check/build. Accepted because ACP is explicitly scaffold/internal/out-of-scope for 0.3.0 unless separately verified.
- Dependency scan reports 4 unknown-severity, no-fixed-version advisories (`bincode`, `fxhash`, `paste`, `yaml-rust`). Accepted for 0.3.0 per `dependency-advisory-cleanup` and `docs/dependency-audit.md`.
- `provider-auth-smoke` used local configured secrets for list/doctor but did not expose secret values; isolated no-key behavior was tested separately.

## Decision readiness

Evidence supports GO for tagging/sharing imp 0.3.0 after explicit user approval to create the tag.
