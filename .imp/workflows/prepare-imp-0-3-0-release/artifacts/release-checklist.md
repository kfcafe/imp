# imp 0.3.0 release checklist

## Release candidate readiness

- [x] Workspace version is `0.3.0` in `Cargo.toml`.
- [x] `imp --version` reports `imp 0.3.0`.
- [x] README install instructions reviewed.
- [x] CHANGELOG has a `0.3.0` entry.
- [x] ACP/editor adapter is documented as scaffold/internal/out-of-scope for 0.3.0 unless separately verified.
- [x] Archive root exists: `~/imp-archive/imp-0.3.0-release-candidate`.
- [x] No files moved to archive yet; manifest records that no file met the safe archival threshold.
- [ ] Full automated verification passed.
- [ ] Dependency advisories are resolved or explicitly accepted for RC.

## Local tag checklist

Do not tag until the user explicitly approves.

Before tag:

1. `cargo fmt --check`
2. `cargo check --workspace`
3. `cargo test -p imp-core`
4. `cargo test -p imp-cli`
5. `cargo test -p imp-tui`
6. CLI smoke tests from `verification-plan.md`
7. `audit_scan secrets`
8. `audit_scan deps` and advisory disposition recorded
9. `git diff --check`
10. Review `results.md` go/no-go summary

Tag command after explicit approval:

```sh
git tag -a v0.3.0 -m "Release imp 0.3.0"
```
