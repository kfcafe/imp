# Dependency advisory cleanup results

Status: done

## Advisory list

Initial scan found five affected packages:
- `bincode` 1.3.3 — RUSTSEC-2025-0141, no fixed version.
- `fxhash` 0.2.1 — RUSTSEC-2025-0057, no fixed version.
- `lru` 0.12.5 — RUSTSEC-2026-0002 / GHSA-rhfx-m35p-ff5j, fixed in 0.16.3+.
- `paste` 1.0.15 — RUSTSEC-2024-0436, unmaintained/no fixed version.
- `yaml-rust` 0.4.5 — RUSTSEC-2024-0320, no fixed version.

## Fix or acceptance rationale

Fixed:
- Upgraded workspace `ratatui` from 0.29 to 0.30 and ran `cargo update -p ratatui`.
- This moved `lru` from 0.12.5 to 0.16.4, removing the only fixable advisory.

Accepted for launch:
- `bincode` and `yaml-rust` are transitive through `syntect` for local syntax highlighting; advisories list no fixed version.
- `fxhash` is transitive through `readability-rust`/`scraper`; advisory lists no fixed version.
- `paste` is a transitive build-time proc-macro through `ratatui`; advisory lists no fixed version.

Detailed reverse dependency trees and disposition are recorded in workflow artifacts.

## Verification result

- `audit_scan deps` rerun: `lru` advisory gone; 4 remaining advisories, 0 fixable.
- `cargo check --workspace` passed with pre-existing `imp-cli` ACP dead-code warnings.
