# Dependency audit notes

Last reviewed: 2026-05-27

## Current OSV findings and dependency paths

- `bincode 1.3.3` (`RUSTSEC-2025-0141`) comes from `syntect 5.3.0` built-in syntax/theme dump loading.
  - `imp-tui` needs `SyntaxSet::load_defaults_newlines()` and `ThemeSet::load_defaults()` for code highlighting.
  - Removing `default-syntaxes`/`default-themes` breaks those APIs.
  - Current mitigation: `imp-tui` disables broad syntect defaults and enables only `default-syntaxes`, `default-themes`, `parsing`, and `regex-onig`, which removed `yaml-rust` while preserving highlighting.

- `lru 0.12.5` (`RUSTSEC-2026-0002`) and `paste 1.0.15` (`RUSTSEC-2024-0436`) come from `ratatui 0.29.0`.
  - Disabling ratatui default features does not remove `lru`.
  - `cargo update -p lru --precise 0.16.3` is blocked by `ratatui`'s `lru = ^0.12` requirement.
  - `ratatui 0.30.0` upgrades `lru` but pulls a broad ~48-package update, so treat it as a deliberate UI dependency migration.

- `serde_yml 0.0.12` (`RUSTSEC-2025-0068`) and `libyml 0.0.5` (`RUSTSEC-2025-0067`) come from `mana-core 0.3.2`.
  - No newer `mana-core` version was published at the time of review.
  - Mitigation requires an upstream `mana-core` release or reducing/removing mana-core integration.

- `fxhash 0.2.1` (`RUSTSEC-2025-0057`) comes from `readability-rust 0.1.0 -> scraper 0.18.1 -> selectors 0.25.0`.
  - `readability-rust` has no newer published version.
  - Forcing `scraper 0.24+` or `selectors 0.32+` is blocked by semver constraints.
  - Mitigation requires replacing the HTML readability extractor, which is product-sensitive because it changes web-read output behavior.

## Verified commands

- `cargo update -p scraper --precise 0.24.0 --dry-run` fails because `readability-rust` requires `scraper = ^0.18`.
- `cargo update -p selectors --precise 0.32.0 --dry-run` fails because `scraper 0.18.1` requires `selectors = ^0.25.0`.
- `cargo update -p lru --precise 0.16.3 --dry-run` fails because `ratatui 0.29.0` requires `lru = ^0.12.0`.
