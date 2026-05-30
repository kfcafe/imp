# Remove prototype tool

## Summary

- Removed the model-facing `prototype` tool from the native tool registry.
- Removed the `prototype` module export so the implementation is no longer compiled into the native tool surface.
- Updated README and tool docs to stop advertising `prototype` as a native tool.
- Reframed workflow docs around investigation/evidence records instead of a standalone prototype tool.

## Verification

- `cargo check -p imp-core -p imp-tui` passed.
- Focused search confirms the native registry/module export and docs table no longer expose `prototype`; remaining matches are the now-unreachable source file and historical/design references.

## Notes

- Existing workflow schema support for prototype-style steps/records was intentionally preserved.
- `crates/imp-core/src/tools/prototype.rs` remains in the tree but is no longer exported by `tools::mod` or registered by `register_native_tools`. A later cleanup can delete the file and any dedicated TUI formatting once no compatibility or history concerns remain.
