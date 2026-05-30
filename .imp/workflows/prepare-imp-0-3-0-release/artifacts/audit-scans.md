# Audit scans

## Quality

`audit_scan quality /Users/asher/imp` did not run any scanners because none of semgrep, qlty, gitleaks, osv-scanner, or shellcheck were selected by that mode as quality tools on this machine. Treat as unavailable, not passed.

## Security

`audit_scan security /Users/asher/imp`:

- semgrep unavailable
- gitleaks clean: no leaks found
- shellcheck clean
- osv-scanner found dependency advisories; see dependency section

## Secrets

`audit_scan secrets /Users/asher/imp`:

- gitleaks clean: no leaks found
- semgrep unavailable

## Dependencies

`audit_scan deps /Users/asher/imp` found 7 affected packages in `Cargo.lock`:

| advisory | package | version | severity/fix |
|---|---:|---:|---|
| RUSTSEC-2025-0141 | bincode | 1.3.3 | no fixed version listed |
| RUSTSEC-2025-0057 | fxhash | 0.2.1 | no fixed version listed |
| RUSTSEC-2025-0067 / GHSA-gfxp-f68g-8x78 | libyml | 0.0.5 | CVSS 8.7, no fixed version listed |
| RUSTSEC-2026-0002 / GHSA-rhfx-m35p-ff5j | lru | 0.12.5 | CVSS 2.7, fixed in 0.16.3 |
| RUSTSEC-2024-0436 | paste | 1.0.15 | no fixed version listed |
| RUSTSEC-2025-0068 / GHSA-hhw4-xg65-fp2x | serde_yml | 0.0.12 | CVSS 6.9, no fixed version listed |
| RUSTSEC-2024-0320 | yaml-rust | 0.4.5 | no fixed version listed |

Release blocker: dependency advisories need triage before a hardened HN release. At minimum determine which are transitive, whether they are reachable in imp runtime paths, and whether dependency upgrades/removals are practical.

## Dependency tree triage update

`cargo tree -i` evidence:

- `serde_yml` and `libyml` are pulled by optional `mana-core` paths. Removing unused `mana-core` from `imp-gui` does not remove them from `Cargo.lock` because `mana-core` remains an optional workspace dependency for compatibility features in `imp-core`, `imp-tui`, and `imp-cli`.
- `yaml-rust` and `bincode` are pulled by `syntect -> imp-tui`.
- `lru` and `paste` are pulled by `ratatui -> imp-tui`.
- `fxhash` is pulled by `selectors -> scraper -> readability-rust -> imp-core`.

A narrow safe code cleanup was applied: removed unused direct `mana-core` dependency from `crates/imp-gui/Cargo.toml`. Verification: `cargo check -p imp-gui` passed.

Remaining advisory findings require dependency replacement/upgrade decisions beyond a single safe cleanup:

- replace or gate `syntect` if bincode/yaml-rust advisories are unacceptable;
- upgrade `ratatui` or accept its transitive `lru`/`paste` advisories;
- replace `readability-rust`/`scraper` stack or accept `fxhash` advisory;
- remove optional mana compatibility features or wait for a `mana-core` release that does not depend on `serde_yml`/`libyml`.
