---
id: '21'
title: 'imp efficiency: smarter tool output truncation'
slug: imp-efficiency-smarter-tool-output-truncation
status: open
priority: 2
created_at: '2026-03-23T00:00:21.665478Z'
updated_at: '2026-04-17T08:01:10.381119Z'
notes: |2

  ## Attempt 1 — 2026-03-24T06:26:56Z
  Exit code: 1

  ```

  ```


  ---
  2026-04-17T07:50:17.150125+00:00
  Preflight for step 4 from the agreed near-term order. Will inspect current verify state after the step-3 check so I do not duplicate already-landed tool-truncation work.

  ---
  2026-04-17T07:51:36.538294+00:00
  Preflighted step 4 while checking the current workspace state. Existing verify command is stale because it still points at `imp/crates/imp-core/src/tools/grep.rs`, but this repo now exposes the remaining built-in set through `{ask, edit, read, write, bash, scan, web, mana}` and no standalone `grep.rs` exists. Step 4 should be revisited after step 3, with the unit updated to inspect the current tool set and define truncation work against the actual files in this workspace rather than the removed grep file path.

  ---
  2026-04-17T07:57:29.931722+00:00
  Continuing step 4 after finishing step 3. Current unit verify is stale because it references removed legacy tool files. Next I am re-inspecting the current truncation behavior in the shipped tool set (`scan`, `web`, `bash`, `read`, `mana`) to identify the remaining substantive gap before updating the unit and implementing it.

  ---
  2026-04-17T07:59:05.466874+00:00
  Refreshed diagnosis after finishing step 3 and re-inspecting current truncation behavior in the shipped tool set.

  Current observed state in this workspace:
  1. The unit's original verify path is stale because standalone `grep/find/ls/diff` tool files no longer exist as separate Rust tool definitions.
  2. Truncation infrastructure already exists and reports counts in several tools:
     - `bash`: tail/head truncation with notes like `showing first/last X of Y lines`
     - `scan`: `truncate_output()` already reports `showing first X of Y lines`
     - `web`: `truncate_output()` already reports `showing first X of Y lines`
     - `mana`: `truncate_with_note()` already reports `showing first X of Y lines`
     - `read` / `write` / `git` already emit structured line/byte truncation notes
  3. That means part of the original design is already landed. The remaining useful work is narrower:
     - audit whether any current tool still truncates without an explicit shown/total count
     - reduce overly large default caps where the current behavior is still too context-heavy
     - update the verify gate to target the actual current tool set rather than removed grep-specific files

  Revised implementation target:
  - treat this unit as a current-tool-set truncation audit + cleanup pass, not a grep-specific limit change
  - likely focus files: `crates/imp-core/src/tools/bash.rs`, `crates/imp-core/src/tools/scan/mod.rs`, `crates/imp-core/src/tools/web/mod.rs`, and `crates/imp-core/src/tools/mana.rs` if any remaining inconsistency is found
  - acceptance should now be framed around explicit truncation notes and any adjusted defaults in the current tool set

  Next action: update the verify command/description to reflect the current tool set before making behavior changes, so future passes do not chase removed files.
  ## Attempt 2 — 2026-04-17T08:01:10Z
  Exit code: 2

  ```
  grep: imp/crates/imp-core/src/tools/grep.rs: No such file or directory
  ```
verify: cd /Users/asher/imp && grep -q 'DEFAULT_LIMIT.*50\|of.*matches\|of.*results' crates/imp-core/src/tools/grep.rs
checkpoint: '55c295c8901f6c58954da8e2bae9bbb1c578e7f8'
verify_hash: '55ad6a7f846ba231c25ba58d6f6c5c10b2a7a0e13d71a340161f3109ae693c58'
attempts: 2
history:
- attempt: 1
  started_at: '2026-03-24T06:26:56.676229Z'
  finished_at: '2026-03-24T06:26:56.730668Z'
  duration_secs: 0.054
  result: fail
  exit_code: 1
- attempt: 2
  started_at: '2026-04-17T08:01:10.325402Z'
  finished_at: '2026-04-17T08:01:10.381111Z'
  duration_secs: 0.055
  result: fail
  exit_code: 2
  output_snippet: 'grep: imp/crates/imp-core/src/tools/grep.rs: No such file or directory'
kind: job
attempt_log:
- num: 1
  outcome: abandoned
  agent: imp
  started_at: '2026-04-06T21:30:50.666617Z'
  finished_at: '2026-04-06T21:37:56.565660Z'
autonomy_disposition:
  kind: blocked
  blockers:
  - verify_failed
  review: unknown
  approval: unknown
  verify: failed
  visibility: unknown
  attempt_pressure: near_limit
  risk: unknown
  provenance: mixed
  continuation_budget: 1
---

## Problem
Tool outputs use fixed-size truncation (2000 lines, 50KB). This is context-blind — grep returning 100 matches when the model probably needs 10, scan dumping entire file structures, web read returning full pages. Every extra token burns context window and money.

## Design
1. grep: Default limit from 100 to 50 for line search. For block search, already 10.
2. scan: When extracting a single file, trim to just the requested file's output
3. web read: Consider more aggressive default truncation for large pages
4. All tools: Add a note about total results when truncating ("50 of 342 matches shown")

## Files
- `imp/crates/imp-core/src/tools/grep.rs` — adjust defaults
- `imp/crates/imp-core/src/tools/scan/mod.rs` — trim output
- `imp/crates/imp-core/src/tools/web/mod.rs` — review truncation

## Acceptance
- grep default limit reduced
- Truncation messages show total count
- Tests pass
