---
id: '54'
title: Fix local imp installation workflow so source installs work cleanly from the repo root
slug: fix-local-imp-installation-workflow-so-source-inst
status: open
priority: 1
created_at: '2026-04-16T04:57:04.659432Z'
updated_at: '2026-04-16T05:20:10.129537Z'
notes: |-
  ---
  2026-04-16T05:20:10.129531+00:00
  Implemented both install-side fixes. (1) Root install / `uu install`: made the repo root a thin installable package named `imp-install` that exposes a root `imp` binary via `src/main.rs` delegating to `imp_cli::run()`, while preserving the existing workspace members. Verified `uu install --dry-run` still resolves to `cargo install --path .`, and `cargo install --path . --root /tmp/imp-root-install-test --force` now succeeds from the repo root and produces a working `imp` binary. (2) macOS replaced-binary SIGKILL mitigation: isolated that `/Users/asher/.local/imp-current/bin/imp` was killable immediately after replacement while same bytes at a different name/path ran fine; ad-hoc `codesign --force --sign - <installed path>` fixes it immediately. Added `tools/imp-fix-signature.sh` helper and documented the workaround in README instead of trying to hide it in Cargo build hooks. Also fixed imp-cli exhaustiveness for the new AgentEvent::Warning variant so installs/builds pass. Verified installed-path help works again after running the signature helper on `~/.local/imp-current/bin/imp`.
labels:
- bugfix
- install
- cli
- ux
verify: cd /Users/asher/tower/imp && cargo check -p imp-cli && test -n "install-workflow-diagnosed"
kind: job
---

Goal: diagnose and fix local imp installation friction so the normal install path (including the user's `uu install` workflow) works without virtual-manifest confusion or post-install runtime/SIGKILL workarounds.

Current state:
- User normally uses `uu install` to install imp.
- Running install at `/Users/asher/tower/imp` currently hits a virtual-manifest error because `Cargo.toml` at the repo root is a workspace manifest, not a package manifest.
- A copied built binary to `/Users/asher/bin/imp` was observed to die with SIGKILL when invoked in this tool environment, but the cause is not yet isolated.
- Need to inspect the install workflow, likely `uu install` behavior, imp-cli packaging, and any repo-level affordances/documentation or thin wrapper needed to make install intent obvious and reliable.

Steps:
1. Inspect what `uu install` resolves to in this environment and how it chooses manifests.
2. Reproduce the virtual-manifest failure with the exact install workflow if possible.
3. Inspect whether this should be solved by repo/package structure, a helper manifest/package target, or install docs/tooling guidance.
4. Investigate the SIGKILL symptom enough to distinguish environment/sandbox artifact from a real local install/runtime issue.
5. Implement the smallest durable fix that makes the expected install path reliable, then verify with the real install command if feasible.

Files:
- Cargo.toml (read/modify if install affordance belongs at repo root)
- crates/imp-cli/Cargo.toml (read/modify if packaging/install metadata needs adjustment)
- README.md (modify if install guidance needs correction)
- any local install helper scripts/config discovered during investigation

In scope:
- Local install UX for imp from source
- Virtual-manifest install failure
- Enough SIGKILL investigation to avoid papering over a real bug

Out of scope:
- Broad release/distribution redesign
- Unrelated runtime bugs not connected to local install

Do not:
- Assume `uu install` semantics without inspecting it
- Claim the SIGKILL is fixed unless reproduced and verified
