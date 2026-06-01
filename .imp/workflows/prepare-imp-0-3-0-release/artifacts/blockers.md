# Release blockers

## Dependency advisories

`audit_scan deps` reports RustSec/OSV advisories in `Cargo.lock`. This blocks a hardened public RC until triaged. Immediate likely remediation is to remove/replace `serde_yml`/`yaml-rust`/`libyml` if unused in favor of `serde_yaml`, and update `lru` if possible.

## ACP/editor scope ambiguity

README currently links `docs/acp.md` and lists `ACP editor adapter scaffold` as preview/planned. The user directed ACP/editor integration to be out of scope unless polished and documented. For RC clarity, docs should demote or remove ACP launch promises unless verification is added.

## Existing unrelated dirty files

README, `crates/imp-cli/src/lib.rs`, `docs/index.md`, `crates/imp-cli/src/acp/`, and `docs/acp.md` were already dirty/untracked before this run. Treat carefully and avoid overwriting user changes.
