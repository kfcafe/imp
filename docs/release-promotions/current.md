# imp release promotion notes

Generated helper page:

```sh
./tools/release_commit_board.py --base release --target nightly --include-workflow --raw --output docs/release-promotions/commit-board.html
open docs/release-promotions/commit-board.html
```

Branch policy:

- `nightly` is the integration branch.
- `release` is stable and intentionally excludes unproven nightly work.
- Promote proven work to `release` with provenance-preserving cherry-picks (`git cherry-pick -x <sha>`), not ambiguous `nightly` merge commits.
- Keep release-only commits limited to packaging, versioning, changelog/docs, and narrowly verified hotfixes.

## Current classification seed

### Nightly-only

| Commit | Initial status | Notes |
| --- | --- | --- |
| `d652102` Prepare vanilla imp release | needs review | Current nightly tip. Do not promote wholesale unless all included release prep is desired and verified. |

### Release-only drift

| Commit | Initial status | Notes |
| --- | --- | --- |
| `9e6cd9c` Clean release branch artifacts | release-only | Likely stable-branch cleanup; verify exact deleted artifacts before keeping. |
| `eb3f46f` Use published mana crates for release build | release-only | Expected stable packaging behavior if release should build against published mana crates. |
| `42634db` Merge branch 'nightly' into release | suspicious | Ambiguous curated-release history; avoid this pattern going forward. |
| `b472ead` Merge branch 'nightly' into release | suspicious | Ambiguous curated-release history; avoid this pattern going forward. |
| `371150f` Merge branch 'nightly' into release | suspicious | Ambiguous curated-release history; avoid this pattern going forward. |
| `d36a3c1` Merge branch 'nightly' into release | suspicious | Ambiguous curated-release history; avoid this pattern going forward. |
| `2c50e96` Merge branch 'nightly' into release | suspicious | Ambiguous curated-release history; avoid this pattern going forward. |
| `34f8be6` Merge branch 'nightly' into release | suspicious | Ambiguous curated-release history; avoid this pattern going forward. |

### Workflow sprint commits

These are shown in the board with `--include-workflow`. Initial default is **defer** until there is evidence of real performance/reliability gain and release-grade smoke coverage.

| Commit/range | Initial status | Required evidence before release |
| --- | --- | --- |
| `0184de6..31e1a04` workflow sprint branch | defer | Demonstrated performance/UX gain, mana worker contract smoke, TUI event-loop smoke, and full release gate. |
