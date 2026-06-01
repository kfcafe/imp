# HN docs readiness results

Status: done

## Claims

- Docs gaps found: see `artifacts/docs-gap-audit.md`.
- Docs changes made: see `artifacts/docs-updates.md`.
- Stale public claims checked: targeted scan passed after correcting shell quoting for the backticked `mana` pattern.

## Verification

Reviewed diff:

```sh
git diff -- README.md docs/index.md .imp/workflows/docs-hn-readiness
```

Ran narrow docs verification:

```sh
git diff --check -- README.md docs/index.md .imp/workflows/docs-hn-readiness && \
if rg -n 'legacy `mana`|ACP/editor adapters planned|bash-equivalent mana|serde_yml|mana_updated|mana refs|mana/evidence|mana detection|mana-unit' README.md docs/index.md docs/*.md -S; then
  echo "stale docs claims remain"
  exit 1
else
  echo "HN docs targeted stale-claim scan clean."
fi
```

Result: `HN docs targeted stale-claim scan clean.`
