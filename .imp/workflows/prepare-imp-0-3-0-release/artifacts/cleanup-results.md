# Cleanup results

Applied:

- Removed unused direct `mana-core` dependency from `crates/imp-gui/Cargo.toml`.
  - Reason: `imp-gui` source does not reference `mana-core`, and the GUI is experimental/out of the default workspace surface.
  - Verification: `cargo check -p imp-gui` passed.

Not moved to archive yet:

- No code or docs were moved to `~/imp-archive` in this pass. Current evidence supports caution: several legacy/experimental paths are still compile-reachable or compatibility-oriented, and user scope says to archive only clearly unused/dead code and stale docs.

Remaining cleanup blockers:

- Dependency advisories remain in transitive dependencies.
- ACP/docs clarity still needs a focused docs edit once the pre-existing ACP changes are reconciled.
