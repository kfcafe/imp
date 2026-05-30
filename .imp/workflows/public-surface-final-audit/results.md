# Public surface final audit results

Status: done

## Claims

- Command surface audited: captured from `./target/debug/imp --help` and subcommand help in `artifacts/command-surface.md`.
- Docs surface audited: compared README/docs claims against the captured 0.3.0 public surface in `artifacts/docs-claims.md`.
- Stale claims fixed or explicitly deferred: fixed active stale public-surface wording and recorded evidence in `artifacts/fixes.md`.

## Verification

- Reviewed `git diff -- docs/autonomy-modes.md`.
- `git diff --check -- docs/autonomy-modes.md` passed.
- Targeted scan for stale public-surface claims passed with: `No targeted stale public-surface claims remain.`
