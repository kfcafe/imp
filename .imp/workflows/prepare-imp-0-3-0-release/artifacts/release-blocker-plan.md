# Release blocker fix plan

- Triage OSV dependency findings with `cargo tree -i <crate>`.
- Remove direct vulnerable YAML stack if unused; prefer existing `serde_yaml`.
- Run `cargo update -p lru --precise 0.16.3` only if constraints allow.
- Re-run `cargo check --workspace` and `audit_scan deps`.
- Verify provider/auth CLI paths with help/list-models/secrets doctor smoke checks that do not require live secrets.
