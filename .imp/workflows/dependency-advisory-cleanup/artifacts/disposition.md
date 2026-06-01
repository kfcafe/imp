# Dependency advisory disposition

Source scan: `audit_scan deps` / `osv-scanner scan source -r .`.

## Fixed

- RUSTSEC-2026-0002 / GHSA-rhfx-m35p-ff5j (`lru` 0.12.5)
  - Root cause: `ratatui` 0.29 pulled `lru` 0.12.5.
  - Fix: upgraded workspace `ratatui` from 0.29 to 0.30 and ran `cargo update -p ratatui`.
  - Result: lockfile now resolves `lru` 0.16.4.
  - Verification: `cargo check -p imp-tui` passed.

## Accepted for launch with rationale

- RUSTSEC-2025-0141 (`bincode` 1.3.3)
  - Transitive via `syntect` 5.3.0.
  - No fixed version is available from the advisory.
  - Usage is local syntax-highlighting dependency path, not a network-exposed deserialization boundary in imp.

- RUSTSEC-2025-0057 (`fxhash` 0.2.1)
  - Transitive via `selectors` -> `scraper` -> `readability-rust`.
  - No fixed version is available from the advisory.
  - Used for web/readability parsing; acceptable for 0.3.0 with future replacement of readability stack if needed.

- RUSTSEC-2024-0436 (`paste` 1.0.15)
  - Transitive proc-macro via `ratatui`.
  - Unmaintained/no fixed version in advisory.
  - Build-time macro dependency only; accepted for launch.

- RUSTSEC-2024-0320 (`yaml-rust` 0.4.5)
  - Transitive via `syntect` 5.3.0.
  - No fixed version is available from the advisory.
  - Local syntax-highlighting dependency path; accepted for launch.
