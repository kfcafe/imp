---
id: '254'
title: Fresh smoke trial for native imp run on an isolated mana unit
slug: fresh-smoke-trial-for-native-imp-run-on-an-isolate
status: open
priority: 1
created_at: '2026-04-14T08:42:58.661162Z'
updated_at: '2026-04-24T05:37:32.528742Z'
acceptance: A throwaway artifact exists at `.tmp/imp-run-trial/native-imp-run-smoke.txt` containing the phrase `native imp run smoke test succeeded`, and no source files outside `.tmp/imp-run-trial/` are modified.
notes: |-
  ---
  2026-04-14T17:38:34.970304+00:00
  Sequenced after new stage-1 smoke job 256 so the trial proceeds as: (1) one-shot native imp print sanity check, then (2) isolated native `imp run <unit-id>` smoke on this unit.

  ---
  2026-04-14T17:45:13.730792+00:00
  User explicitly confirmed trial order: do option 1 (one-shot native imp sanity check) first, then option 3 (native `imp run` smoke on unit 254).

  ---
  2026-04-24T05:31:04.712360+00:00
  Graph hygiene pass 2026-04-24: dependent stage-1 smoke job 256 did not succeed, and no completed native imp run artifact is present for this unit. Reclassified from in_progress to open so status truth matches current state.
labels:
- imp
- trial
- smoke
- runtime
dependencies:
- '256'
verify: cd /Users/asher/tower && test -f .tmp/imp-run-trial/native-imp-run-smoke.txt && rg -q 'native imp run smoke test succeeded' .tmp/imp-run-trial/native-imp-run-smoke.txt
checkpoint: '5d82dd80fd9d5756a0b2ab91a3e3ad5e7746336c'
verify_hash: b82372b3c1cc93fe0695b2e8e4b651256689b0fa03c462116e55ae1c4b33b423
kind: job
paths:
- '.tmp/imp-run-trial/native-imp-run-smoke.txt'
attempt_log:
- num: 1
  outcome: abandoned
  agent: imp
  started_at: '2026-04-14T17:40:58.560761Z'
  finished_at: '2026-04-24T05:37:32.528742Z'
---

Goal: exercise the native `imp run <unit-id>` path end-to-end on a harmless isolated unit.

Current state:
- We want a fresh trial of the canonical imp single-unit worker runtime.
- This should avoid product code changes and keep output confined to a throwaway temp path inside the repo.

Steps:
1. Create the directory `.tmp/imp-run-trial/` if it does not already exist.
2. Write `.tmp/imp-run-trial/native-imp-run-smoke.txt` with a short note that includes the phrase `native imp run smoke test succeeded` and mentions that it was produced by the native imp run path.
3. Do not modify files outside `.tmp/imp-run-trial/`.
4. If you need to inspect the repo for context, keep it minimal and do not make unrelated changes.

Files:
- .tmp/imp-run-trial/native-imp-run-smoke.txt (create/overwrite — throwaway smoke-test artifact)

In scope:
- exercising the native imp run execution path
- one throwaway artifact under `.tmp/imp-run-trial/`

Out of scope:
- code changes in `imp/`, `mana/`, docs, configs, or `.mana/` beyond normal run bookkeeping
- broad repo edits or cleanup

Do not:
- touch source files
- change workspace config
- create additional artifacts unless required by the runtime
