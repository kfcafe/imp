---
id: '271'
title: Harden spawn and mana tool termination so close/spawn calls cannot hang indefinitely
slug: harden-spawn-and-mana-tool-termination-so-closespa
status: open
priority: 1
created_at: '2026-04-24T17:19:49.867277Z'
updated_at: '2026-04-24T17:19:49.867277Z'
labels:
- imp
- mana
- spawn
- tooling
- hang
- reliability
verify: cd /Users/asher/tower && mana show 271 >/dev/null
kind: epic
feature: true
---

Goal: eliminate or contain cases where imp's native spawn tool or mana tool call path never returns, especially around mana close and spawn flows.
Current state: user has observed mana close or spawn calls that do not end and appear to run forever. Likely surfaces include imp tool wrappers, worker spawn/attach paths, and mana close/run plumbing.

Planned outcomes:
1. Reproduce and classify the hang modes with concrete call paths and logs.
2. Identify whether the root cause is in imp tool orchestration, mana command execution, stream completion handling, wait/join behavior, or missing timeout/cancellation boundaries.
3. Implement targeted hardening plus regression coverage so stuck calls fail fast or complete cleanly.
4. Document any remaining known limits and create follow-up units for residual architecture debt.

Candidate files:
- imp/crates/imp-core/src/tools/mana.rs
- imp/crates/imp-core/src/mana_worker.rs
- imp/crates/imp-core/src/agent.rs
- mana/crates/mana-core/src/ops/close.rs
- mana/crates/mana-cli/src/commands/close/
- mana/crates/mana-cli/src/commands/run/
- mana/crates/mana-cli/src/mcp/tools.rs

Out of scope:
- unrelated mana graph cleanup
- broad spawn architecture redesign beyond what is needed to stop indefinite hangs

Do not:
- paper over hangs with only larger global timeouts if the underlying wait/completion contract is wrong
- broaden into general provider/runtime reliability unless directly implicated
