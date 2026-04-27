---
id: '263'
title: Audit and isolate library-level stderr writes that can break embedded runtimes
slug: audit-and-isolate-library-level-stderr-writes-that
status: open
priority: 2
created_at: '2026-04-15T07:42:55.590948Z'
updated_at: '2026-04-15T07:53:10.944837Z'
acceptance: '1) Inventory non-CLI stderr writes in imp-core, imp-llm, imp-lua, and mana-core with a risk classification for embedded/TUI disruption. 2) For high-risk call sites, replace direct stderr writes with structured errors, UI/status surfaces, returned diagnostics, or tracing/logging that does not corrupt interactive screens. 3) Preserve intentional CLI-facing messaging in imp-cli and mana-cli. 4) Add focused regression tests or evidence for any changed high-risk path. 5) Document residual intentional stderr sites and why they remain acceptable.'
notes: |-
  Candidate high-risk files from scan: imp/crates/imp-core/src/agent.rs, session.rs, mana_worker.rs, tools/shell.rs; imp/crates/imp-llm/src/providers/anthropic.rs; imp/crates/imp-lua/src/lib.rs and bridge.rs; mana/crates/mana-core/src/ctx_assembler.rs, ops/context.rs, ops/update.rs, ops/create.rs, hooks.rs.

  ---
  2026-04-15T07:53:10.944831+00:00
  Scoped the TUI-protection pass to library/runtime stderr writes reachable while ratatui owns the terminal. Initial classification from code inspection: high-risk immediate fixes are imp-core/src/agent.rs: OAuth refresh failure currently eprintlns during live agent execution, and imp-core/src/mana_worker.rs: run_verify_command currently eprintlns failed verify stdout/stderr and is reachable via the imp tool's unit-spawn path. Lower-priority/deferred candidates include imp-core session malformed-line warnings, imp-core shell tool definition load warnings, imp-llm anthropic retry/SSE parse warnings, imp-lua runtime/extension/event-handler warnings, and mana-core ctx_assembler/context/hook post-create/post-update warnings. No eprintln occurrences were found in imp-tui itself.
design: 'Current known non-CLI eprintln counts from repo scan: imp-core=5, imp-llm=4, imp-lua=4, mana-core=12. No eprintln occurrences were found in imp-tui itself. Highest immediate suspicion is library/runtime code reachable from imp''s TUI or in-process tool execution. Prior incident: mana-core create verify-lint printed to stderr and broke ratatui flow; fixed by moving detail into structured error text and keeping CLI printing at the CLI layer.'
labels:
- audit
- stderr
- tui
- cross-project
assignee: imp
kind: epic
feature: true
---

Cross-project follow-up after fixing mana create verify-lint output breaking imp TUI flow. Audit remaining eprintln!/println! usage in library/runtime crates that may be invoked in-process by imp or other embedded callers. Prefer root scope because this touches the mana ↔ imp runtime boundary and output/side-effect expectations across projects.
