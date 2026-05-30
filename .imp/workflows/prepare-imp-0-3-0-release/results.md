# Prepare imp 0.3.0 release candidate — results

Status: done with concerns / release-candidate blocked on dependency advisory decision

## Completed

- Created and ran the durable 0.3.0 release workflow.
- Audited release/product surface, stale docs, dead-code candidates, provider/auth paths, and dependency/security findings.
- Confirmed workspace version is already `0.3.0` and `imp --version` prints `imp 0.3.0`.
- Removed an unused direct `mana-core` dependency from the experimental `imp-gui` crate.
- Clarified README/docs index so ACP editor adapter scaffold is internal/out-of-scope for 0.3.0 unless separately verified.
- Updated CHANGELOG for the cleanup/docs changes.
- Created `~/imp-archive/imp-0.3.0-release-candidate/MANIFEST.md`; no files were archived because no file met the safe threshold of clearly unused/dead/stale.
- Wrote release and local tag checklist at `artifacts/release-checklist.md`.

## Verification

Passed:

```sh
cargo fmt --check
cargo check --workspace
cargo check -p imp-gui
cargo test -p imp-core --lib
cargo test -p imp-cli --lib
cargo test -p imp-tui --lib
cargo run -q -p imp-cli -- --version
cargo run -q -p imp-cli -- workflow validate prepare-imp-0-3-0-release
cargo run -q -p imp-cli -- list-models
git diff --check
audit_scan secrets
```

Notes:

- `cargo check --workspace` passes but reports warnings from the pre-existing/untracked ACP scaffold code about unused scaffold fields/functions.
- `audit_scan secrets` passes: gitleaks found no leaks.

## Blocking concern

`audit_scan deps` still reports RustSec/OSV advisories in transitive dependencies:

- `bincode 1.3.3` via `syntect -> imp-tui`
- `yaml-rust 0.4.5` via `syntect -> imp-tui`
- `lru 0.12.5` and `paste 1.0.15` via `ratatui -> imp-tui`
- `fxhash 0.2.1` via `selectors -> scraper -> readability-rust -> imp-core`
- `serde_yml 0.0.12` / `libyml 0.0.5` via optional `mana-core` compatibility paths

This is the concrete remaining blocker for a hardened public 0.3.0 RC. The next decision is whether to accept these advisories for RC with documented rationale, or do a dependency replacement/upgrade pass before tagging.

## Final tag

Not created. Final `v0.3.0` tag still requires explicit user approval after the dependency advisory decision.
