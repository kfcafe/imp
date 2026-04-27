---
id: '259'
title: Audit panic usage and detached task failure policy across imp
slug: audit-panic-usage-and-detached-task-failure-policy
status: closed
priority: 2
created_at: '2026-04-15T02:28:45.140975Z'
updated_at: '2026-04-15T02:53:41.862592Z'
notes: |-
  ---
  2026-04-15T02:30:13.848543+00:00
  Important design nuance to verify in follow-up: `catch_unwind` catches the unwind but does not prevent Rust's panic hook from firing first. That means the new `run_guarded()` boundary is good for cleanup/structured error return but may not, by itself, suppress default panic text while the TUI still owns the terminal. Detached-task analysis should explicitly account for panic-hook timing and whether a minimal interactive panic hook remains necessary as a last-resort terminal-restore mechanism.

  ---
  2026-04-15T02:44:45.396059+00:00
  Analysis pass completed. Findings: (1) Most `panic!` sites reported by grep are inside `#[cfg(test)]` modules in `imp-core::{context,compaction,imp_session,tools::mana,tools::bash}`, `imp-cli::lib`, and `imp-tui::app`; these are low priority. (2) Important production detach/supervision patterns: `imp-tui::app` supervises `agent_task` and `login_task` via `JoinHandle::is_finished` + await and surfaces `JoinError` as runtime signals; `imp-core::imp_session` awaits `agent_task` and converts panic to `Error::Config("Agent task panicked: ...")`; `imp-cli` RPC mode supervises spawned agent runs via stored join handles and emits protocol errors on join failure. (3) Real fire-and-forget runtime tasks remain in `imp-core::hooks` (non-blocking shell hooks spawned and ignored), `imp-core::retry::stream_with_retry` (spawned stream forwarder with no external join handle), provider streaming internals in `imp-llm::{anthropic,openai,openai_compat,google}` (spawn and forward bytes/events to channels), and small UI follow-up tasks in `imp-tui::app` / `imp-core::tools::mana`. (4) Production unwrap/unreachable worth cleanup: `imp-cli::run_interactive` still has `unwrap_or_else(|| SessionManager::new(...).unwrap())`; `imp-core::tools::bash` uses `child.stdout.take().unwrap()` and `child.stderr.take().unwrap()` after piping configuration; `imp-core::tools::extend` has `_ => unreachable!()` in an action match that can be replaced with an explicit error branch even if logically impossible. (5) Policy recommendation: use `Result` for world/external failures, reserve panic for invariants/bugs and tests, and require supervision or explicit loss-tolerance for spawned tasks. (6) Important nuance: `catch_unwind` catches unwinds but the panic hook still fires first, so interactive terminal protection may still require a minimal panic-hook restore for panics escaping through the TUI while the alt-screen is active.

  ---
  2026-04-15T02:53:41.862589+00:00
  Decomposed next cleanup phase into child jobs: 259.2 covers non-blocking hook supervision in `imp-core/src/hooks.rs` as the highest-value detached-task fix; 259.3 covers retry/provider spawned-task audit and targeted hardening after hooks. Execution order should be 259.2 first, then 259.3 if hook supervision lands cleanly.
labels:
- analysis
- panic
- error-handling
- imp
claimed_by: imp
claimed_at: '2026-04-15T02:43:03.662630Z'
kind: job
attempt_log:
- num: 1
  outcome: abandoned
  agent: imp
  started_at: '2026-04-15T02:43:03.662630Z'
---

Audit panic usage and detached-task failure handling across the imp codebase, with emphasis on interactive/runtime paths. Explain panic semantics in Rust for project context, distinguish expected recoverable errors from invariant violations, inventory production vs test panic sites, and identify spawned tasks whose failures are awaited vs effectively detached. Propose a cleanup policy that minimizes user-visible panics, converts recoverable runtime failures to Result/error values, and defines how detached task failures should be surfaced without corrupting terminal/UI state.

Current findings to preserve:
- Interactive top-level TUI run path is now guarded by `InteractiveRunner::run_guarded()`.
- Repo-wide grep shows many `panic!`/`unwrap`/`expect` sites are test-only, but there are runtime-relevant spawned tasks across `imp-tui`, `imp-cli`, `imp-core`, and `imp-llm`.
- Detached panics that do not unwind through the guarded runner remain a separate category and likely need explicit task supervision or JoinError handling.

Requested outputs:
1. Plain-language explanation of Rust panic vs Result/error.
2. Categorized inventory: test-only panics, startup/config panics, runtime panics, spawned-task panic risks.
3. Recommendation for when panic is acceptable in this repo and when it should be replaced.
4. Concrete cleanup plan with first targets and verification approach.

Relevant files/directories:
- /Users/asher/imp/crates/imp-tui/src/app.rs
- /Users/asher/imp/crates/imp-tui/src/interactive.rs
- /Users/asher/imp/crates/imp-cli/src/lib.rs
- /Users/asher/imp/crates/imp-core/src/agent.rs
- /Users/asher/imp/crates/imp-core/src/imp_session.rs
- /Users/asher/imp/crates/imp-core/src/tools
- /Users/asher/imp/crates/imp-llm/src/providers

No implementation yet unless requested; this unit is for analysis and planning.
