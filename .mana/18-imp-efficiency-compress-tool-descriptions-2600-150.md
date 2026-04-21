---
id: '18'
title: 'imp efficiency: compress tool descriptions (~2600 → ~1500 tokens)'
slug: imp-efficiency-compress-tool-descriptions-2600-150
status: in_progress
priority: 1
created_at: '2026-03-22T23:59:18.310303Z'
updated_at: '2026-04-17T07:56:55.639731Z'
notes: |2-

  ## Attempt 1 — 2026-03-24T06:26:55Z
  Exit code: 1

  ```
  Traceback (most recent call last):
    File "<string>", line 19, in <module>
      assert total < 4000, f'Total tool def size {total} >= 4000'
             ^^^^^^^^^^^^
  AssertionError: Total tool def size 8140 >= 4000
  ```


  ---
  2026-04-17T07:50:17.075855+00:00
  Starting step 3 from the agreed near-term order after checking steps 1 and 2. Will run the unit verify gate to see whether the current workspace already contains the tool-description compression work before making further edits.

  ---
  2026-04-17T07:51:29.786805+00:00
  Ran the current verify script in the workspace state. Result: fail, current measured tool-definition total is 5333 chars (target < 4000). The scoped measurement in the verify script only covers the currently shipped built-in set `{ask, edit, read, write, bash, scan, web, mana}` because older tool files like grep/find/ls/diff no longer exist as separate Rust tool definitions. Largest current contributors under that measurement:
  - `scan`: 973
  - `edit`: 849
  - `bash`: 686
  - `mana`: 620
  - `ask`: 602
  - `web`: 591
  - `read`: 558
  - `write`: 454
  This means step 3 is still genuinely open and is the next concrete code-change target in the agreed order.

  ---
  2026-04-17T07:54:28.436194+00:00
  Continuing step 3. I inspected the current shipped tool set and verified the measured total is still 5333 (> 4000). Next action is to trim the current high-cost tool descriptions/parameter schemas in `{scan, edit, bash, mana, ask, web, read, write}` rather than chasing removed legacy tool files.

  ---
  2026-04-17T07:56:55.639724+00:00
  Implemented step 3 by trimming the current shipped tool descriptions/parameter schemas in `{write, read, ask, bash, edit, web, scan, mana}`. Re-ran the sizing verify script against the unit's measured tool set and reduced total tool-definition size from 5333 to 3209 (< 4000). Also ran `cargo check -p imp-core` to confirm the schema-string edits still compile.
verify: |-
  cd /Users/asher/tower && python3 -c "
  import re, os
  total = 0
  for name in ['grep','edit','diff','read','write','find','ls','bash','ask']:
      p = f'imp/crates/imp-core/src/tools/{name}.rs'
      if not os.path.exists(p): continue
      c = open(p).read()
      d = re.search(r'fn description.*?\"(.+?)\"', c, re.DOTALL)
      pm = re.search(r'fn parameters.*?json!\((\{.*?\})\s*\)', c, re.DOTALL)
      total += len(d.group(1) if d else '') + len(pm.group(1) if pm else '')
  for name in ['scan','web','mana']:
      p = f'imp/crates/imp-core/src/tools/{name}/mod.rs'
      if name == 'mana': p = f'imp/crates/imp-core/src/tools/{name}.rs'
      if not os.path.exists(p): continue
      c = open(p).read()
      d = re.search(r'fn description.*?\"(.+?)\"', c, re.DOTALL)
      pm = re.search(r'fn parameters.*?json!\((\{.*?\})\s*\)', c, re.DOTALL)
      total += len(d.group(1) if d else '') + len(pm.group(1) if pm else '')
  assert total < 4000, f'Total tool def size {total} >= 4000'
  print(f'Tool def size: {total} chars (target: <4000)')
  "
checkpoint: a846e944534744ae687283dced47e94faf379f10
verify_hash: '6116ddc7bd6225e3615db21273fd0aa396aeb49662344e5c1aea71e258fb6d55'
attempts: 1
claimed_by: imp
claimed_at: '2026-04-06T22:38:54.941995Z'
history:
- attempt: 1
  started_at: '2026-03-24T06:26:55.815588Z'
  finished_at: '2026-03-24T06:26:55.923487Z'
  duration_secs: 0.107
  result: fail
  exit_code: 1
  output_snippet: |-
    Traceback (most recent call last):
      File "<string>", line 19, in <module>
        assert total < 4000, f'Total tool def size {total} >= 4000'
               ^^^^^^^^^^^^
    AssertionError: Total tool def size 8140 >= 4000
kind: job
attempt_log:
- num: 1
  outcome: abandoned
  agent: imp
  started_at: '2026-04-06T22:38:54.941995Z'
---

## Problem
12 tool definitions consume ~2,600 tokens per API request. The biggest offenders:
- grep: 437 tokens (verbose boolean query docs, 12 params)
- mana: 435 tokens (6 commands with nested params)
- web: 290 tokens
- scan: 263 tokens
- ask: 256 tokens

These tokens are sent on EVERY turn. Shaving 40% saves ~1,100 tokens/turn → ~22K tokens over a 20-turn session.

## Approach
1. Shorten descriptions — remove redundant explanations, use terse language
2. Remove param descriptions that are self-evident from the name (e.g., "path" doesn't need "Directory or file to search")
3. Collapse enum descriptions into the enum values themselves
4. Remove params the model rarely uses (e.g., `allowTests` default is fine)

Target: ~1,500 tokens total (40% reduction).

## Files
- `imp/crates/imp-core/src/tools/grep.rs` — trim description + params
- `imp/crates/imp-core/src/tools/mana.rs` — trim command params
- `imp/crates/imp-core/src/tools/web/mod.rs` — trim
- `imp/crates/imp-core/src/tools/scan/mod.rs` — trim
- `imp/crates/imp-core/src/tools/ask.rs` — trim
- All other tool files — audit and trim

## Acceptance
- All tools still compile and pass tests
- Total tool definition size (sum of description + params JSON chars) < 4000 chars (was ~7500)
