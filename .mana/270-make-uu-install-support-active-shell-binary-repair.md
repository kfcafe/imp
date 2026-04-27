---
id: '270'
title: Make `uu install` support active-shell binary repair for local source installs
slug: make-uu-install-support-active-shell-binary-repair
status: open
priority: 2
created_at: '2026-04-24T03:50:54.246889Z'
updated_at: '2026-04-24T03:50:54.246889Z'
acceptance: '`uu install` can run an explicit post-install repair step for the imp repo, and from the imp repo root the one-command flow leaves `imp` in the shell resolving to the newly installed binary even when a shadow binary path like ~/bin/imp exists earlier on PATH. Focused verification or tests cover command construction / hook invocation.'
verify: bash -lc 'cd /Users/asher/imp && uu install --dry-run'
kind: job
paths:
- /Users/asher/uu
- /Users/asher/imp
---

Patch the `uu` repo in ~/uu so `uu install` can support a one-command local install/upgrade flow for tools like imp whose active shell command may be shadowed by a user-local bin path (for example ~/bin/imp) ahead of ~/.cargo/bin/imp. Inspect how `uu install` currently detects project type and shells out to cargo. Implement the narrowest mechanism that lets a project participate in post-install active-path repair without broad packaging redesign. Then update imp to use that mechanism so `uu install` alone is sufficient there.
