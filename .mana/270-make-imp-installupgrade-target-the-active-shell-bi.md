---
id: '270'
title: Make imp install/upgrade target the active shell binary
slug: make-imp-installupgrade-target-the-active-shell-bi
status: open
priority: 2
created_at: '2026-04-24T03:10:06.390234Z'
updated_at: '2026-04-24T03:55:59.374911Z'
acceptance: A fresh install/upgrade path exists that makes `imp` in the user's shell resolve to the newly installed binary, or the CLI explicitly detects and guides repair of stale shadow binaries overriding the new install. Docs and/or CLI verification make the active command path explicit. Verification should cover a mismatched-path scenario like `~/bin/imp` shadowing `~/.cargo/bin/imp`.
notes: |-
  ---
  2026-04-24T03:10:38.748379+00:00
  Observed live environment failure: `command -v imp` resolved to `/Users/asher/bin/imp` while `cargo install` refreshed `/Users/asher/.cargo/bin/imp`. User expectation is valid: after a 'fresh install', typing `imp` should launch the new build.

  ---
  2026-04-24T03:17:16.824730+00:00
  Implemented a real local upgrade/install fix in crates/imp-cli/src/lib.rs plus README docs. Added `imp install-local`, which installs the currently running build to the user-visible `imp` command path rather than assuming Cargo's bin dir is the active command. Resolution strategy: if an active `imp` on PATH exists under HOME (for example `~/bin/imp`), install there; otherwise prefer `~/bin/imp` if `~/bin` is on PATH, then `~/.local/bin/imp` if present on PATH, else fall back to `~/.cargo/bin/imp`. Added `--dry-run` and `--dest` overrides, and post-install messaging that reports whether `imp` now resolves to the new path. Updated README source-install instructions to use `cargo install --path .` followed by `~/.cargo/bin/imp install-local`. Verified with focused tests for active-path and PATH-fallback resolution, plus a CLI build and `./target/debug/imp install-local --dry-run` in the current environment, which correctly selected `/Users/asher/bin/imp`.

  ---
  2026-04-24T03:43:36.562493+00:00
  User feedback: current two-step source install (`cargo install` then `imp install-local`) is still too much. User expects `uu install` to be the one-command local install/upgrade entrypoint that leaves `imp` runnable directly from the shell. Follow-up direction: inspect how `uu install` is wired today and make the root/source install path perform the active-shell-path repair automatically, or fail with an explicit one-command path that `uu install` itself can execute.

  ---
  2026-04-24T03:53:28.019169+00:00
  Patched ~/uu so `uu install` now supports a repo-local post-install hook: if `tools/uu-post-install` or `tools/uu-post-install.sh` exists, it is appended after the normal install steps. Then added imp/tools/uu-post-install.sh to run `imp install-local` (or `$HOME/.cargo/bin/imp install-local` as fallback). This makes `uu install` from the imp repo root the one-command local install/upgrade flow that repairs the active shell `imp` path automatically. Updated imp README to recommend `uu install` as the preferred source install/upgrade flow. Verified in uu with targeted tests and dry-run output: `uu install -n -C /Users/asher/imp` now shows both `cargo install --path .` and `bash tools/uu-post-install.sh`.
design: 'Treat `cargo install` success as insufficient. The install UX must verify command resolution, not just artifact creation. Prefer a narrow operator-trust fix: either refresh the active shim/symlink in the expected shell bin dir or fail loudly with a concrete repair command when the active `imp` path does not match the new install location. Keep the solution reversible and easy to reason about.'
verify: bash -lc 'command -v imp && imp --help >/dev/null'
kind: job
paths:
- /Users/asher/imp/README.md
- /Users/asher/imp/crates/imp-cli/src/lib.rs
decisions:
- '`uu install` cannot reliably make `imp` in the user''s shell point at the fresh binary from the imp repo alone while `uu` remains a thin wrapper around `cargo install --path .`. Cargo install only writes to its own install root (typically `~/.cargo/bin`) and provides no post-install hook to repair a shadow binary earlier on PATH (for example `~/bin/imp`). Therefore, making `uu install` alone sufficient requires either a uu-side feature/change or a deliberate PATH policy change, not just imp repo changes.'
- 'A fully program-agnostic `uu install` that ''installs to system'' is not a safe default. Install semantics differ by ecosystem (Cargo, Go, uv/pip, npm, make/cmake, etc.), and many installs intentionally target user-local bins, virtual envs, or project deps rather than a system command path. The better abstraction is explicit command activation: let projects declare or let uu infer which executable names should be made active in a preferred user bin dir, with opt-in flags/policy for activation behavior. Default should remain non-destructive; activation should be explicit or policy-driven rather than silently writing to system paths.'
---

Make fresh installs/upgrades of imp update the binary users actually launch when they type `imp`. Current failure mode: `cargo install --path crates/imp-cli --force` refreshes `~/.cargo/bin/imp`, but user shells may resolve a stale shadow binary first (for example `~/bin/imp`), so the active command remains old even though Cargo rebuilt successfully. Solve the install/upgrade UX, not just the compile step. Treat success as active command resolution success.

Execution plan:
1. Inspect current install/upgrade docs, wrappers, and any helper scripts to identify the intended canonical command path.
2. Choose the narrowest fix that makes `imp` deterministic after install. Likely candidates:
   - add an explicit install/upgrade helper that installs/refreshes the active shim/symlink users actually run
   - add a startup/doctor diagnostic when the running binary path differs from `command -v imp` or when a stale shadow binary overrides Cargo's bin
   - tighten docs so the supported install flow explicitly verifies the resolved command path
3. Implement the minimal viable fix in imp, keeping scope to install/upgrade UX and path determinism.
4. Add focused verification for the stale-shadow-binary case and document the operator flow.

Non-goals:
- broad packaging redesign
- Homebrew/distribution strategy changes unless separately approved
