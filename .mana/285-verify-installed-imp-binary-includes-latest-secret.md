---
id: '285'
title: Verify installed imp binary includes latest secrets fix
slug: verify-installed-imp-binary-includes-latest-secret
status: open
priority: 1
created_at: '2026-04-27T23:18:50.997775Z'
updated_at: '2026-04-27T23:18:50.997775Z'
acceptance: Active `imp` binary resolves to the expected rebuilt executable; `imp secrets doctor` passes; the previously failing provider/runtime path no longer reports a missing API key, or a precise follow-up bug is recorded with provider/model/error details.
labels:
- secrets
- verification
- follow-up
kind: task
---

After fixing the AuthStore secret-field lookup mismatch in imp, verify the user's installed `imp` binary is rebuilt from the current repo and no stale binary remains on PATH. Steps: from `/Users/asher/imp`, run `which imp` and `imp --version` to identify the active binary; if it is an installed binary rather than `cargo run`, rebuild with `cargo install --path . --force`; rerun `imp secrets doctor` and a minimal provider/runtime path that previously reported a missing API key. Do not print or log secret values. If failure persists, capture the exact provider/model name and the full missing-key error text.
