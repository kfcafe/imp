---
id: '258'
title: Restore terminal before interactive TUI panic/error output so prompt box is not corrupted
slug: restore-terminal-before-interactive-tui-panicerror
status: closed
priority: 1
created_at: '2026-04-15T01:24:31.548608Z'
updated_at: '2026-04-15T01:58:55.812927Z'
acceptance: Interactive TUI installs fatal-output handling that restores raw mode/alt screen before panic text is printed; non-panic fatal interactive errors are printed after terminal teardown; focused tests cover the new behavior enough to prevent regression.
notes: |-
  ---
  2026-04-15T01:31:40.181816+00:00
  Implemented terminal-restoration handling for imp interactive TUI. Added imp-tui terminal restore helper plus a scoped panic hook in crates/imp-tui/src/terminal.rs so panics restore raw mode/alt-screen before Rust prints panic text. Added a belt-and-suspenders restore call in crates/imp-cli/src/lib.rs after interactive-mode fatal errors so CLI eprintln! output lands on the normal terminal. Added focused terminal tests for restore sequences and panic-hook invocation.

  ---
  2026-04-15T01:51:52.586965+00:00
  Follow-up design note: current fix is pragmatic but not ideal end-state. It uses a scoped global panic hook in imp-tui plus a CLI-side restore fallback. Cleaner architecture would centralize interactive fatal-path teardown inside InteractiveRunner/TerminalSession and wrap the top-level interactive future in a panic boundary, leaving the hook only for panics escaping spawned/background tasks if still needed.

  ---
  2026-04-15T01:58:26.942257+00:00
  Architecture cleanup done via child job 258.1. Fatal interactive teardown is now centralized in imp-tui InteractiveRunner::run_guarded() instead of split across a scoped panic hook and imp-cli fallback restore. TerminalSession restore is explicit and idempotent; CLI just prints returned errors after guarded runner teardown.
labels:
- bug
- imp
- tui
- panic
dependencies:
- '258.1'
verify: cd /Users/asher/tower && cargo test -p imp-tui && cargo test -p imp-cli run_interactive
kind: job
paths:
- /Users/asher/imp/crates/imp-tui/src/terminal.rs
- /Users/asher/imp/crates/imp-tui/src/interactive.rs
- /Users/asher/imp/crates/imp-cli/src/lib.rs
decisions:
- 'Adopt the cleaner interactive fatal-path architecture as the durable design for this bugfix: `imp-tui::interactive::InteractiveRunner::run_guarded()` owns top-level panic/error capture and terminal restoration; `imp-tui::terminal::TerminalSession` owns explicit idempotent restore state; `imp-cli` prints the returned fatal error after teardown instead of performing its own restore fallback. Avoid relying primarily on a scoped global panic hook unless a future background-task escape path proves it necessary.'
---

Bug in /Users/asher/imp interactive TUI: when a panic occurs during a live session, Rust's panic text is emitted while the app still owns the alternate screen/raw terminal, so the panic overlays the prompt/editor box instead of appearing as normal terminal output after cleanup. Fix interactive fatal-output handling so panics restore the terminal before printing, and ensure fatal interactive startup/runtime errors are also printed only after TUI teardown. Inspect imp-tui terminal/session startup and imp-cli run_interactive entrypoints; prefer a small reversible fix with focused tests.
