---
id: '256'
title: Run one-shot native imp print smoke before imp run trial
slug: run-one-shot-native-imp-print-smoke-before-imp-run
status: open
priority: 1
created_at: '2026-04-14T17:38:27.452766Z'
updated_at: '2026-04-24T05:37:32.544472Z'
acceptance: The local imp binary runs successfully in one-shot print mode and writes `.tmp/imp-run-trial/one-shot-print.txt` containing the exact phrase `native imp print smoke ok`.
notes: |-
  ---
  2026-04-14T17:41:39.365505+00:00
  Attempt 1 timed out after 120s when running `../target/debug/imp --no-tools -p ...` from `imp/` with default config (`model = gpt-5.4`, `thinking = xhigh`). No output artifact was created. Next attempt should override to a faster one-shot configuration (`--provider openai --model gpt-5.4-mini --thinking off --max-tokens 32 --no-session --no-tools`) to isolate binary/runtime startup from long model latency.

  ---
  2026-04-14T17:48:36.027194+00:00
  Attempt 2 also timed out after 180s even with explicit fast overrides: `./target/debug/imp --provider openai --model gpt-5.4-mini --thinking off --max-tokens 32 --no-session --no-tools -p ...`. Need to diagnose whether one-shot prompt mode is hanging on provider/network/auth/runtime rather than simply being slow.

  ---
  2026-04-24T05:31:04.667283+00:00
  Graph hygiene pass 2026-04-24: prior attempts timed out and left only an empty artifact file at .tmp/imp-run-trial/one-shot-print.txt. This is not active execution today. Reclassified from in_progress to open retry-needed investigation/smoke job; keep notes about the two timed-out attempts as prior evidence.
labels:
- imp
- trial
- smoke
- runtime
verify: test -f .mana/256-run-one-shot-native-imp-print-smoke-before-imp-run.md && rg -q '^id:' .mana/256-run-one-shot-native-imp-print-smoke-before-imp-run.md
checkpoint: '5d82dd80fd9d5756a0b2ab91a3e3ad5e7746336c'
verify_hash: '5f93c77c0ad86e028ca51ab201b2a5a677c3b6bc1aa8630fdc6246458d6ddfd6'
kind: job
paths:
- '.tmp/imp-run-trial/one-shot-print.txt'
attempt_log:
- num: 1
  outcome: abandoned
  agent: imp
  started_at: '2026-04-14T17:40:58.533738Z'
  finished_at: '2026-04-24T05:37:32.544472Z'
---

Goal: do the cheapest possible native imp binary sanity check before exercising the full `imp run <unit-id>` path.

Current state:
- We want to trial the native imp tool in two stages.
- Stage 1 should be a minimal one-shot print invocation that proves the local imp binary starts and returns output successfully.
- Stage 2 is the isolated mana-backed `imp run` smoke test tracked separately.

Steps:
1. Create `.tmp/imp-run-trial/` if needed.
2. Run the local imp binary in one-shot print mode with a very small prompt, capturing stdout to `.tmp/imp-run-trial/one-shot-print.txt`.
3. The prompt should request the exact phrase `native imp print smoke ok` so verification is deterministic.
4. Do not modify source files, config, or mana units other than normal run bookkeeping.

Files:
- .tmp/imp-run-trial/one-shot-print.txt (create/overwrite — captured output from one-shot imp print mode)

In scope:
- one-shot native imp binary sanity check
- one throwaway artifact under `.tmp/imp-run-trial/`

Out of scope:
- code changes in `imp/`, `mana/`, docs, configs, or `.mana/`
- the full mana-backed worker path beyond sequencing into the follow-up trial

Do not:
- touch source files
- broaden into runtime debugging unless the smoke check fails
