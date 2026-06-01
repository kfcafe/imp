# Remove mana vocabulary results

Status: done

## Work completed

- Removed dependency-backed legacy mana support from the active imp tree.
- Archived removed support files under `~/imp-archive/remove-mana-vocabulary/` with a manifest.
- Removed active mana feature/dependency surfaces from manifests and active modules.
- Renamed user-facing config/settings vocabulary to workflow terminology:
  - `Config.workflow`
  - `WorkflowConfig`
  - `WorkflowRunConfig`
  - `WorkflowScopePreference`
- Renamed active runtime/review/orchestration vocabulary from mana-oriented names to workflow-oriented names.
- Removed active README claim that legacy mana integration is supported.
- Added a changelog note that legacy mana integration was removed from the active public surface.

## Verification performed

- `cargo test -p imp-tui settings` — passed, 10 tests.
- `cargo check --workspace` — passed.
  - Existing `imp-cli` ACP dead-code warnings remain; they are unrelated to this cleanup.
- Final active-surface grep:
  - `rg -n '\bmana\b|\bMana\b|mana_' crates/imp-core/src crates/imp-cli/src crates/imp-tui/src README.md docs/index.md Cargo.toml crates/*/Cargo.toml`
  - Result: no matches.
- No mana serde/config aliases remain in active config surface.

## Remaining references reviewed

Historical/proposal docs and workflow artifacts were intentionally excluded by workflow scope. Active source, active crate manifests, README, and docs index are clean.
