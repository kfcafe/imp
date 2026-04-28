---
id: '290'
title: Complete imp codebase quality audit
slug: complete-imp-codebase-quality-audit
status: in_progress
priority: 2
created_at: '2026-04-28T00:26:53.440826Z'
updated_at: '2026-04-28T00:41:32.830204Z'
acceptance: A report exists in chat or mana notes covering codebase-wide quality, naming, organization, major risks, and prioritized recommendations, with scope caveats and evidence from inspected files. Inventory of inspected file categories is recorded.
notes: |
  ---
  2026-04-28T00:27:54.443841+00:00
  Audit inventory: 147 code/config files included by extension (.rs, .sh, .py, .html, .toml) excluding target/.git/.mana/node_modules/dist/build; 92,100 total lines. Rust source breakdown: imp-core 62 files/45,180 lines; imp-tui 32/22,362; imp-llm 20/11,511; imp-cli 3/6,903; imp-lua 4/2,358. Full scanner availability wrapper could not run semgrep/qlty because commands were not installed despite reported availability. `cargo clippy --workspace --all-targets -- -D warnings` currently fails in imp-llm oauth/kimi_code.rs with redundant_closure at line 173 and unnecessary_get_then_check at line 181. Large hotspots found: imp-tui/src/app.rs 7,094 lines; imp-cli/src/lib.rs 5,852; imp-core/src/agent.rs 5,786; imp-core/src/tools/mana.rs 3,525; imp-llm/src/providers/anthropic.rs 2,463; imp-core/src/session.rs 2,334. Root contains several zero-byte stray files likely from accidental shell redirection: CreateParams and files whose names begin with `{`. No code modified.

  ---
  2026-04-28T00:40:52.501527+00:00
  User approved audit steps 1 and 2: fixed strict clippy issues in crates/imp-llm/src/oauth/kimi_code.rs, removed zero-byte accidental root files (CreateParams and five files whose names started with `{`). User also asked to use unit 290 to plan large-file splits and preferred AGENTS.md over crate READMEs. Added module-organization guidance to /Users/asher/imp/AGENTS.md: preserve behavior first, split by responsibility, keep API churn minimal, move tests with behavior, avoid mixing moves with semantic changes, and run narrow crate-level checks after each extraction. Recorded decomposition targets: imp-tui app.rs, imp-core agent.rs, imp-cli lib.rs, imp-core tools/mana.rs.
  ## Attempt 1 — 2026-04-28T00:41:32Z
  Exit code: 127

  ```
  sh: Inventory: command not found
  ```
labels:
- audit
- quality
verify: Inventory code files with fd/rg and confirm audit notes cover all included source file groups.
attempts: 1
history:
- attempt: 1
  started_at: '2026-04-28T00:41:32.776170Z'
  finished_at: '2026-04-28T00:41:32.830201Z'
  duration_secs: 0.054
  result: fail
  exit_code: 127
  output_snippet: 'sh: Inventory: command not found'
kind: task
autonomy_disposition:
  kind: blocked
  blockers:
  - verify_failed
  review: unknown
  approval: unknown
  verify: failed
  visibility: unknown
  attempt_pressure: within_budget
  risk: unknown
  provenance: mixed
  continuation_budget: 2
---

Audit the imp codebase for general code quality, variable naming, organization, and maintainability. User explicitly requested reading every code file completely. Approach: inventory code files, exclude generated/build/vendor artifacts, read code files in full in manageable batches, run quality scanners where useful, record findings with evidence paths/lines, and produce a concise audit report with prioritized recommendations. Do not modify code unless separately requested.
