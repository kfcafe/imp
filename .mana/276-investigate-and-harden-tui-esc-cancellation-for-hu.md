---
id: '276'
title: Investigate and harden TUI Esc cancellation for hung tool calls
slug: investigate-and-harden-tui-esc-cancellation-for-hu
status: in_progress
priority: 2
created_at: '2026-04-27T05:30:13.100459Z'
updated_at: '2026-04-27T05:34:02.502093Z'
notes: |
  ---
  2026-04-27T05:30:23.709078+00:00
  Initial verify command is malformed for Cargo: `cargo test -p imp-tui esc cancel` and `cargo test -p imp-core agent_cancel tools::bash` pass multiple test filters, which Cargo rejects. Use separate targeted test invocations or broader package tests while preserving the intended gate.

  ---
  2026-04-27T05:33:59.753528+00:00
  Implemented smallest hardening slice. Findings: Esc/Ctrl-C mapped to the same normal-mode cancel action. Before this change, cancel while streaming only sent AgentCommand::Cancel and immediately set `is_streaming=false`; because the active tool cancel token was internal to agent.rs, subsequent Esc presses no longer had a way to escalate while the agent task/tool process was still running. Core bash cancellation already existed for normal cooperative cancellation, but the TUI could lose visibility of the stuck task after first Esc.

  Changes: Agent/AgentHandle now share the active cancel token; Agent::run resets it at run/turn start and sets it on between-turn cancel. TUI cancel now treats `is_streaming || agent_task.is_some()` as cancellable: first Esc sends cancel and flips the shared token, marks the streaming UI message done, and suppresses completion notification; a repeated Esc when the token is already cancelled aborts the stuck agent JoinHandle and clears the handle. Added TUI regression `esc_cancel_first_requests_cancel_second_aborts_stuck_agent_task`.

  Verification passed:
  - `cargo test -p imp-tui esc_cancel -- --nocapture`
  - `cargo test -p imp-core agent_cancel -- --nocapture`
  - `cargo test -p imp-core bash_cancellation -- --nocapture`
  - `cargo check -p imp-tui -p imp-core`

  Caveat: repo was already dirty with unrelated Kimi/TypeScript-extension/sidebar work; this task touched only `crates/imp-core/src/agent.rs`, `crates/imp-tui/src/app.rs`, and mana metadata. The original unit verify command remains malformed because Cargo accepts only one test filter; use the verified split commands above.
  ## Attempt 1 — 2026-04-27T05:34:02Z
  Exit code: 1

  ```
  error: unexpected argument 'cancel' found

  Usage: cargo test [OPTIONS] [TESTNAME] [-- [ARGS]...]

  For more information, try '--help'.
  ```
labels:
- bugfix
- imp-tui
- cancellation
- tools
- esc
- runtime
verify: cd /Users/asher/tower/imp && cargo test -p imp-tui esc cancel -- --nocapture && cargo test -p imp-core agent_cancel tools::bash -- --nocapture && cargo check -p imp-tui -p imp-core
checkpoint: aee2b76bf088b6bdc571dd8f17db28ad8ef4c55f
verify_hash: eac499c1adee0ca5cb02d7250f0070c1009db94a77ffad2258d9a347d557187b
attempts: 1
claimed_by: imp
claimed_at: '2026-04-27T05:30:15.029745Z'
history:
- attempt: 1
  started_at: '2026-04-27T05:34:02.444969Z'
  finished_at: '2026-04-27T05:34:02.502090Z'
  duration_secs: 0.057
  result: fail
  exit_code: 1
  output_snippet: |-
    error: unexpected argument 'cancel' found

    Usage: cargo test [OPTIONS] [TESTNAME] [-- [ARGS]...]

    For more information, try '--help'.
kind: task
paths:
- crates/imp-tui/src/app.rs
- crates/imp-core/src/agent.rs
- crates/imp-core/src/tools/mod.rs
- crates/imp-core/src/tools/bash.rs
- '.mana/.6.7-propagate-cancellation-into-active-tool-execution.md'
attempt_log:
- num: 1
  outcome: abandoned
  agent: imp
  started_at: '2026-04-27T05:30:15.029745Z'
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

User reported that when an imp TUI tool call hangs indefinitely, pressing Esc repeatedly does not appear to stop the tool, and imp becomes sluggish enough that prompt text entry is very slow. Investigate current Esc handling and active tool cancellation behavior end-to-end, then implement the smallest hardening change.

Current suspicion: TUI key handling may only cancel/interrupt UI state or agent streaming while spawned tool futures/processes continue running; repeated Esc may enqueue state churn without a deterministic cancel escalation path.

Concrete steps:
1. Inspect TUI key handling for Esc in crates/imp-tui/src/app.rs and related input/editor modules. Document what first, second, and repeated Esc currently do in normal, streaming, tool-running, sidebar, selection, palette, and modal states.
2. Inspect imp-core agent/tool cancellation paths, especially crates/imp-core/src/agent.rs, crates/imp-core/src/tools/mod.rs, crates/imp-core/src/tools/bash.rs, and any existing cancellation unit .6.7 notes. Determine whether cancellation reaches active tool futures and OS child processes.
3. Reproduce or simulate a hung tool call with a long-running shell command or test double. Confirm whether prompt input latency correlates with still-running tool tasks or render/event-loop churn.
4. Implement the smallest safe hardening: Esc should have explicit semantics while a response/tool is active, with repeated Esc escalating predictably (for example: first requests graceful cancel, second force-cancels active tool/process, third clears stuck UI state if already canceled). Avoid broad UI redesign.
5. Add focused tests for cancellation semantics where practical; at minimum cover the core cancellation path for active tool execution and the TUI Esc state transition logic.
6. Update any user-facing help/status text if Esc semantics change.

Scope boundaries: do not redesign tool-call presentation/sidebar UX here; link to epic 275 for presentation work. Do not add new dependencies. Do not touch more than ~5 files without recording a plan first.
