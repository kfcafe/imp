# Dependency audit notes

Last reviewed: 2026-05-29

## Current OSV findings and dependency paths

The 0.3.0 release-readiness scan was rerun with `osv-scanner scan source -r .` after dependency cleanup.

Fixed before launch:

- `lru 0.12.5` (`RUSTSEC-2026-0002` / `GHSA-rhfx-m35p-ff5j`) was removed by upgrading workspace `ratatui` from `0.29` to `0.30` and refreshing the lockfile. The lockfile now resolves `lru 0.16.4`.

Remaining findings are transitive dependencies with no fixed version reported by OSV:

- `bincode 1.3.3` (`RUSTSEC-2025-0141`) comes from `syntect 5.3.0`, used by the TUI for local syntax highlighting.
- `yaml-rust 0.4.5` (`RUSTSEC-2024-0320`) comes from `syntect 5.3.0`, used by the TUI for local syntax highlighting.
- `fxhash 0.2.1` (`RUSTSEC-2025-0057`) comes from `readability-rust 0.1.0 -> scraper 0.18.1 -> selectors 0.25.0`, used by web/readability parsing.
- `paste 1.0.15` (`RUSTSEC-2024-0436`) is a transitive build-time proc-macro dependency through `ratatui 0.30.0`.

## Launch disposition

- No critical/high/medium vulnerabilities remain in the scan output.
- The only fixable advisory from the scan (`lru`) is fixed.
- Remaining advisories are accepted for 0.3.0 with the rationale above and should be revisited when upstream crates publish fixed dependency paths or when imp replaces the relevant syntax-highlighting/readability stack.

## Verified commands

- `cargo check -p imp-tui` passed after the `ratatui` upgrade.
- `cargo check --workspace` passed after the dependency cleanup.
- `osv-scanner scan source -r .` reports 4 remaining advisories and 0 fixable vulnerabilities.
