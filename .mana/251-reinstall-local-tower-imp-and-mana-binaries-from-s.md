---
id: '251'
title: Reinstall local Tower imp and mana binaries from source on this Mac
slug: reinstall-local-tower-imp-and-mana-binaries-from-s
status: open
priority: 1
created_at: '2026-04-14T07:03:05.679895Z'
updated_at: '2026-04-14T07:03:05.679895Z'
acceptance: |-
  - A root-scoped install unit exists describing the local source reinstall plan for both `imp` and `mana`.
  - The unit records the current binary locations and versions verified in this session.
  - The unit captures the intended replacement/verification flow before any install commands are run.
verify: test -x /Users/asher/bin/imp && test -x /Users/asher/bin/mana && /Users/asher/bin/imp --version >/dev/null && /Users/asher/bin/mana --version >/dev/null
kind: job
---

Goal: perform a fresh local source install of the active Tower binaries (`imp` and `mana`) on this Mac without guessing at the current install state.

Current state already verified in this session:
- `imp` currently resolves to `/Users/asher/bin/imp`
- `mana` currently resolves to `/Users/asher/bin/mana`
- `imp --version` currently reports `imp 0.1.0`
- `mana --version` currently reports `mana 0.3.0`
- No install has been run yet in this conversation; only read-only inspection happened.

Plan:
1. Reconfirm current binary targets and note/backup the existing `/Users/asher/bin/imp` and `/Users/asher/bin/mana` install state before replacement.
2. Build/install fresh from the active Tower source root (`/Users/asher/tower`) rather than guessing from older paths.
3. Replace or relink the local binaries in a controlled way so PATH still resolves `imp` and `mana` from `/Users/asher/bin`.
4. Verify the installed binaries by checking:
   - `command -v imp`
   - `command -v mana`
   - `imp --version`
   - `mana --version`
5. Record any residual mismatch between built binaries and PATH resolution before closing the work.

Relevant paths:
- `/Users/asher/tower`
- `/Users/asher/imp`
- `/Users/asher/tower/mana`
- `/Users/asher/bin/imp`
- `/Users/asher/bin/mana`

Out of scope:
- remote/other-machine install
- release packaging or Homebrew publishing
- unrelated shell/profile cleanup beyond what is required for local PATH resolution

Important note:
- This is cross-project work because it affects both `imp` and `mana`, so it belongs in root mana.
- No install should happen until the user explicitly confirms they want the source install to proceed.
