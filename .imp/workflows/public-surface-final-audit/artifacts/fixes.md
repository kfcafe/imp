# Public surface fixes

Applied docs cleanup for the stale public-surface claim identified in the audit:

- `docs/autonomy-modes.md`: replaced obsolete `bash-equivalent mana blocking` wording with current `workflow/tool safety blocking` wording in the compatibility/default-safe-mode descriptions.
- `docs/autonomy-modes.md`: removed a stale `See also` list that pointed at deleted docs (`docs/reference-monitor-policy.md`, `docs/trace-and-evidence-format.md`, `docs/imp-next-workflow-runtime.md`).

Verification:

- Reviewed `git diff -- docs/autonomy-modes.md`.
- Ran `git diff --check -- docs/autonomy-modes.md` successfully after fixing the trailing blank-line warning.
- Ran targeted stale-claim scan successfully:

```sh
git diff --check -- docs/autonomy-modes.md && \
if rg -n "bash-equivalent (mana|workflow) blocking|serde_yml|mana_updated|mana refs|mana/evidence|mana detection|mana-unit" docs README.md -S; then
  echo "stale targeted claims remain"
  exit 1
else
  echo "No targeted stale public-surface claims remain."
fi
```

Result: `No targeted stale public-surface claims remain.`
