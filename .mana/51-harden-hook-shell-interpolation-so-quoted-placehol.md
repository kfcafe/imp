---
id: '51'
title: Harden hook shell interpolation so quoted placeholders safely carry after_tool_call command text
slug: harden-hook-shell-interpolation-so-quoted-placehol
status: open
priority: 1
created_at: '2026-04-16T03:22:32.255914Z'
updated_at: '2026-04-16T03:27:18.978588Z'
notes: |-
  ---
  2026-04-16T03:27:18.978580+00:00
  Implemented the hook interpolation fix in crates/imp-core/src/hooks.rs. Added replace_placeholder() plus shell_single_quote()/shell_double_quote() so quoted placeholders like '{command}' and "{command}" are escaped as a single shell argument before sh -c runs. Added regression tests hook_interpolation_quoted_placeholder and hook_after_tool_call_nonblocking_quoted_command to cover commands with embedded single quotes, regex metacharacters, pipes, and $-prefixed text. Updated README hooks section to document quoted-placeholder behavior. Verified with targeted cargo tests for both new cases.
labels:
- bugfix
- hooks
- shell
- imp-core
verify: cd /Users/asher/tower/imp && cargo test -p imp-core hook_interpolation_quoted_placeholder hook_after_tool_call_nonblocking_quoted_command -- --nocapture
kind: job
---

Goal: fix imp hook shell interpolation so configs that pass after_tool_call values like {command} as a single shell argument do not break when the command contains quotes, regex metacharacters, subshell syntax, or heredocs.

Current state:
- Hook interpolation in crates/imp-core/src/hooks.rs does plain string replacement.
- User config at ~/.config/imp/config.toml uses:
  ~/.config/imp/hooks/rush-error-log.sh '{is_error}' '{exit_code}' '{command}' &
- When the bash tool command contains embedded single quotes, parentheses, regex, or heredoc syntax, interpolation produces an invalid shell snippet before sh -c runs it.
- This shows up as a non-blocking after_tool_call hook failure rather than the hook script receiving the original command string.

Steps:
1. Inspect interpolate_command and non-blocking shell hook execution in crates/imp-core/src/hooks.rs.
2. Add safe handling for quoted placeholder forms so templates like '{command}' or "{command}" are replaced with a shell-safe single argument containing the original value.
3. Keep existing raw placeholder behavior for unquoted forms unless a clearly safer backwards-compatible improvement is obvious.
4. Add focused regression tests covering after_tool_call command text containing embedded single quotes and shell-sensitive characters.
5. If needed, update hook docs/comments to clarify quoted placeholder behavior.

Files:
- crates/imp-core/src/hooks.rs (modify — interpolation + regression tests)
- README.md (optional modify — hook behavior note if wording needs clarification)

In scope:
- Hook shell interpolation and tests
- after_tool_call regression grounded in the reported rush-error-log config

Out of scope:
- Redesigning the entire hook system
- Changing hook execution away from sh -c
- Broad shell backend changes

Do not:
- Break raw unquoted placeholder expansion without reason
- Claim success without a regression test that exercises quoted placeholder behavior
