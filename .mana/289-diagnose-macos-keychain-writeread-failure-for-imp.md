---
id: '289'
title: Diagnose macOS keychain write/read failure for imp secrets
slug: diagnose-macos-keychain-writeread-failure-for-imp
status: in_progress
priority: 1
created_at: '2026-04-28T00:05:43.903766Z'
updated_at: '2026-04-28T00:09:36.048847Z'
acceptance: Google secret save either creates a readable macOS Keychain item under the expected service/account and doctor passes, or save fails clearly without leaving misleading auth metadata; tests/checks pass and no secret values are exposed.
notes: |-
  ---
  2026-04-28T00:08:06.389549+00:00
  User explicitly wants implementation, not more retry instructions. Proceeding to inspect and fix the write/read path directly. Focus on making `imp secrets` fail loudly if secure storage write does not round-trip, and avoid leaving metadata when the keychain write/readback is broken.

  ---
  2026-04-28T00:09:36.048833+00:00
  Root cause found: `keyring = "3.6"` was compiled without macOS `apple-native`; keyring's macOS fallback without that feature is the in-memory mock backend, so `imp secrets google` wrote metadata but the secret value never reached macOS Keychain. Fix: enable `keyring` feature `apple-native` in `crates/imp-llm/Cargo.toml` and add write-read verification in `KeyringBackend::set` so metadata is not saved if secure storage cannot read back the written value. Verified `cargo test -p imp-llm auth::tests::` and `cargo check -p imp-cli`. Still need install and manual keychain check.
labels:
- secrets
- macos
- keychain
- bug
dependencies:
- '285'
kind: task
---

Goal: determine why `imp secrets google` leaves `~/.imp/auth.json` metadata (`google` -> SecretFields [`api_key`]) but no macOS Keychain item exists for service `imp`, account `google:api_key`, causing `imp secrets doctor` to report `api_key:missing` even after repeated remove/readd.

Current evidence:
- Active binary: `/Users/asher/bin/imp`, version `imp 0.1.0`.
- `~/.imp/auth.json` exists and contains `google: { type: SecretFields, fields: [api_key] }`.
- `~/.config/imp/auth.json` exists but does not contain google.
- Metadata-only checks found no keychain entries for account `google:api_key` under services `imp`, `imp-cli`, `impeccable`, or `mana`.
- User has removed/readded multiple times; do not suggest repeated readd as the only remediation.

Steps:
1. Verify current source build is installed to `/Users/asher/bin/imp` or explicitly reinstall with `cargo install --path . --force --root /Users/asher`.
2. Reproduce with Google once only: run `/Users/asher/bin/imp secrets google` and immediately check `security find-generic-password -s imp -a 'google:api_key'` without `-g`.
3. If the keychain item is still absent, inspect `crates/imp-llm/src/auth.rs` `KeyringBackend::set`, `AuthStore::store_secret_fields`, and callers in `crates/imp-cli/src/lib.rs` / TUI save flow to ensure keyring write errors cannot be swallowed before metadata save.
4. Add a minimal diagnostic or code fix so metadata is not saved when keychain write fails, and error messages distinguish keychain write failure from later read failure.
5. If the keyring crate writes under a different service/account/keychain on macOS, align lookup/delete/status with the actual write target or document/fix the target.

Files:
- `crates/imp-llm/src/auth.rs` (primary: keyring backend, store/read/delete/status behavior)
- `crates/imp-cli/src/lib.rs` (CLI secrets save/remove/doctor callers)
- `crates/imp-tui/src/app.rs` (TUI secrets save flow, if needed)
- `crates/imp-core/src/storage.rs` (auth metadata path context only)

Scope boundaries:
- Do not print or log secret values.
- Do not use `security ... -g` unless output is safely redacted before display.
- Do not ask the user to repeatedly readd keys without new evidence.
- Prefer a focused fix in auth/keyring error handling over broad auth refactors.

Verify:
- `cargo test -p imp-llm auth::tests::`
- `cargo check -p imp-cli`
- Manual metadata-only check: after save, `security find-generic-password -s imp -a 'google:api_key'` finds an item, and `imp secrets doctor` no longer reports Google `api_key:missing`; or, if keychain write fails, `imp secrets google` exits with a clear write error and does not leave misleading metadata.
