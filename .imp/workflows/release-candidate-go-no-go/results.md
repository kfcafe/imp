# Final 0.3.0 release candidate go/no-go

Status: GO, pending explicit user approval to create the `v0.3.0` tag.

## Child workflow statuses

- `archive-stale-history` — done.
- `dependency-advisory-cleanup` — done.
- `remove-mana-vocabulary` — done.
- `public-surface-final-audit` — done.
- `docs-hn-readiness` — done.
- `cli-tui-regression-smoke` — done_with_concerns.
- `install-smoke-release-candidate` — done.
- `provider-auth-smoke` — done.

Detailed review: `artifacts/child-results.md`.

## Final blockers

None found in the completed readiness evidence.

## Accepted concerns

- ACP scaffold dead-code warnings remain during workspace check/build. This is accepted because ACP is documented as scaffold/internal/out-of-scope for 0.3.0 unless separately verified.
- Dependency scan reports 4 unknown-severity advisories with no fixed version (`bincode`, `fxhash`, `paste`, `yaml-rust`). This is accepted for 0.3.0 and documented in `docs/dependency-audit.md`.

## Verification basis

Evidence includes:

- Public command surface capture and docs audit.
- README/docs HN-readiness updates and targeted stale-claim scans.
- Dependency cleanup and latest dependency scan disposition.
- Secrets scan with no leaks found.
- Source install smoke with isolated `CARGO_HOME`, installed `imp 0.3.0`, and help smoke.
- Provider/auth setup smoke and no-key error-path check without live model calls.
- Regression smoke rerun passing:
  - `cargo fmt --check`
  - `cargo test -p imp-core --lib`
  - `cargo test -p imp-tui --lib`
  - `git diff --check`

## Tag approval

Do not create the `v0.3.0` git tag until the user explicitly approves tagging.
