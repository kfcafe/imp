---
id: '285'
title: Verify installed imp binary includes latest secrets fix
slug: verify-installed-imp-binary-includes-latest-secret
status: open
priority: 1
created_at: '2026-04-27T23:18:50.997775Z'
updated_at: '2026-04-28T00:05:44.006488Z'
acceptance: Active `imp` binary resolves to the expected rebuilt executable; `imp secrets doctor` passes; the previously failing provider/runtime path no longer reports a missing API key, or a precise follow-up bug is recorded with provider/model/error details.
notes: |-
  ---
  2026-04-27T23:54:25.102881+00:00
  User's `imp secrets doctor/list` output shows Google is specifically broken: `Google (google) api_key broken (api_key:missing)`. This means auth metadata exists but the secure-storage/keychain value for `google.api_key` is absent or unreadable. Next verification/remediation steps: re-save only Google with `imp secrets google` using default field `api_key`; then run `imp secrets show google` and `imp secrets doctor`. If Google remains missing immediately after re-save, investigate OS keychain write/read path for service `imp` account `google:api_key` rather than broad provider resolution. Do not print secret values.

  ---
  2026-04-27T23:54:57.550788+00:00
  User clarified Google remains broken after multiple remove/readd cycles, so this is likely OS keychain persistence/readback rather than user not re-saving. Need diagnose macOS keychain entries for service/account metadata only: expected current service `imp`, account `google:api_key`; legacy services include `imp-cli`, `impeccable`, `mana`. Check whether entries exist, whether CLI write reports success but immediate read via doctor fails, and whether duplicate/stale keychain entries or keychain access control are involved. Avoid printing secret values; use metadata-only keychain commands or `-g` only if output is redacted/not shown.

  ---
  2026-04-28T00:01:25.436902+00:00
  User reported `security: SecKeychainSearchCopyNext: The specified item could not be found in the keychain.` for metadata-only keychain lookup. Interpret carefully: if run after `imp secrets rm google` or before re-saving, expected; if run immediately after `imp secrets google` says saved, then keyring write is not landing in the searched keychain/service/account. Next: verify exact binary path/version, re-save Google once, immediately check `security find-generic-password -s imp -a 'google:api_key'`, and inspect `~/.imp/auth.json` metadata for google without secrets. If still absent, likely keyring backend write failure being swallowed or writing to a different keychain; inspect keyring crate behavior / macOS default keychain.

  ---
  2026-04-28T00:05:22.639127+00:00
  User pasted current doctor detail: `Google (google) — broken (api_key:missing)` and `api_key: missing from secure storage`. Now inspecting active binary, auth metadata for google, and keychain metadata-only existence for service/account combinations without exposing secret values.

  ---
  2026-04-28T00:05:44.006487+00:00
  Externalized the durable follow-up as a separate executable unit focused on macOS keychain write/read failure. Current unit remains the verification thread; new child/follow-up unit captures evidence, reproduction, files to inspect, scope boundaries, and verify gates.
labels:
- secrets
- verification
- follow-up
kind: task
---

After fixing the AuthStore secret-field lookup mismatch in imp, verify the user's installed `imp` binary is rebuilt from the current repo and no stale binary remains on PATH. Steps: from `/Users/asher/imp`, run `which imp` and `imp --version` to identify the active binary; if it is an installed binary rather than `cargo run`, rebuild with `cargo install --path . --force`; rerun `imp secrets doctor` and a minimal provider/runtime path that previously reported a missing API key. Do not print or log secret values. If failure persists, capture the exact provider/model name and the full missing-key error text.
