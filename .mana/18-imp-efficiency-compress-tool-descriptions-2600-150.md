---
id: '18'
title: 'imp efficiency: compress tool descriptions (~2600 → ~1500 tokens)'
slug: imp-efficiency-compress-tool-descriptions-2600-150
status: open
priority: 1
created_at: '2026-03-22T23:59:18.310303Z'
updated_at: '2026-03-24T06:26:55.923499Z'
notes: |2

  ## Attempt 1 — 2026-03-24T06:26:55Z
  Exit code: 1

  ```
  Traceback (most recent call last):
    File "<string>", line 19, in <module>
      assert total < 4000, f'Total tool def size {total} >= 4000'
             ^^^^^^^^^^^^
  AssertionError: Total tool def size 8140 >= 4000
  ```
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
attempts: 1
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
