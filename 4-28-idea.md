# 4/28 idea: imp sandboxing, shell envelopes, and agent-friendly execution

Date: 2026-04-28  
Status: design notes / implementation direction  
Related projects: `aush`, `mana`

## Summary

The useful lesson from Vercel's `just-bash` is not that `imp` should collapse into a tiny `bash/read/write` agent. The stronger lesson is that coding agents become more trustworthy when command execution is structured, bounded, inspectable, and optionally isolated.

For `imp`, the best direction is:

```text
imp = agent/runtime/policy/work executor
```

`imp` should keep its governed native tools for operations where policy, secrets, durable state, result structure, or user interaction matter. It can use AUSH as an excellent shell backend, but it should not make raw shell the only abstraction.

The most important product idea is `imp --sandbox`:

> Let the agent inspect, edit, and verify freely in an isolated workspace, then show a diff and apply only after approval.

This complements AUSH's shell-level direction:

```text
aush = agent-friendly shell/runtime
imp  = agent runtime and policy layer
mana = durable work graph and coordination substrate
```

## Design principle

> imp should make agent behavior trustworthy.

Trustworthy does not mean slow or over-confirmed. It means:

- shell output does not drown the context window;
- file writes can be staged before reaching the user's real workspace;
- destructive/network/secrets operations are policy-aware;
- tool results include enough metadata for the model to reason correctly;
- durable work and retries go through mana;
- boundaries such as local, sandbox, container, and remote execution are visible.

## Non-goals

This document does not propose that `imp` should:

- remove native tools in favor of only shell;
- make AUSH mandatory;
- embed mana's durable graph directly in session history;
- implement a full VM/container orchestrator as the first sandbox version;
- silently escalate commands between local, sandbox, remote, or network-enabled environments;
- treat executable scripts as equivalent to governed native tools.

---

# Idea 1: `imp --sandbox`

## Goal

`imp --sandbox` should provide speculative agent execution.

The user should be able to run:

```bash
imp --sandbox -p "fix the failing parser test"
imp run --sandbox 12.1
```

and get this flow:

1. `imp` creates an isolated workspace.
2. The agent inspects and edits inside the sandbox.
3. The agent runs the relevant checks inside the sandbox.
4. `imp` summarizes changed files and verification results.
5. `imp` shows a diff.
6. The user chooses apply, discard, or keep for inspection.

The core promise:

> Agent writes do not touch the user's active workspace until accepted.

## User experience

One-shot prompt:

```bash
imp --sandbox -p "fix cargo test -p imp-core shell_tool"
```

Interactive:

```bash
imp --sandbox
```

Inside the TUI or chat shell:

```text
/sandbox status
/sandbox diff
/sandbox apply
/sandbox discard
```

Mana worker:

```bash
imp run --sandbox 12.1
```

End-of-run summary:

```text
Sandbox changes ready.

Verify:
  cargo test -p imp-core shell_tool  PASS

Changed:
  crates/imp-core/src/tools/shell.rs
  crates/imp-core/tests/shell_tool.rs

Apply changes to /Users/asher/imp?
  [a] apply  [d] discard  [v] view diff  [k] keep sandbox
```

## First implementation: git worktree backend

Do not start with true OverlayFS. Start with git worktrees.

For git repos:

1. Detect repo root and current HEAD.
2. Create a temporary worktree:

   ```text
   /tmp/imp-sandbox/<repo-name>-<id>/
   ```

3. Run the agent with cwd set to the worktree.
4. Run shell commands and verification inside the worktree.
5. Compute diff relative to the source branch/base.
6. Apply a patch back to the original workspace only after approval.
7. Remove or keep the worktree based on user choice.

Benefits:

- simple mental model;
- works with normal tools;
- no platform-specific overlay dependency;
- easy diff generation;
- safe rollback by deleting the worktree.

## Non-git fallback: temp-copy backend

For non-git directories:

1. Copy project files into a temp dir using ignore rules where possible.
2. Run the agent there.
3. Compute a directory diff.
4. Apply selected changes back on approval.

This is less elegant than worktrees, but still useful.

## Later backends

After the UX is proven:

```text
worktree    = default for git repos
temp-copy   = fallback for non-git dirs
container   = dependency-isolated execution
overlay     = true copy-on-write filesystem if practical
remote      = remote builder / larger machine
```

Every backend must be visible in tool results and summaries.

## Boundary visibility

Sandbox status should be explicit in all relevant tool calls:

```json
{
  "sandbox": {
    "enabled": true,
    "backend": "git-worktree",
    "sandbox_root": "/tmp/imp-sandbox/imp-abc123",
    "apply_target": "/Users/asher/imp"
  }
}
```

The agent and user should never have to guess where writes went.

## Handling uncommitted user changes

This is the hardest product detail for a worktree backend.

Possible policies:

### Conservative default

If the source workspace is dirty, ask:

```text
Current workspace has uncommitted changes.
Sandbox can start from HEAD only, or include a patch of current changes.

[start from HEAD] [include current changes] [cancel]
```

### Include-current-changes mode

`imp` can generate a patch from the source workspace and apply it to the sandbox. The final diff must distinguish:

- pre-existing user changes;
- new sandbox changes.

This is more complex but important for real workflows.

### Recommendation

Phase 1 should support clean workspaces well and refuse or ask on dirty workspaces. Do not silently copy dirty state.

## Applying changes

Application should use patch/diff, not blind file copying.

Potential flow:

1. Generate diff from sandbox relative to base.
2. Show summary and diff.
3. On approval, attempt `git apply` in target workspace.
4. If patch conflicts, stop and report the conflict.
5. Never overwrite target files silently.

## Sandbox plus mana

`imp run --sandbox <id>` is especially compelling.

Mana provides:

- task scope;
- acceptance criteria;
- verify gate;
- notes and attempts;
- dependency context.

Sandbox provides:

- safe execution workspace;
- isolated edits;
- diff before apply.

Possible behavior:

```bash
imp run --sandbox 12.1 --defer-verify
```

The worker executes in the sandbox. On success, it reports:

- unit id;
- sandbox root;
- verify result;
- patch path;
- apply status.

Closing the mana unit should happen only after changes are applied and verify passes in the target workspace, unless the user explicitly chooses a different workflow.

---

# Idea 2: consume AUSH command result envelopes

AUSH should produce shell-level command result envelopes. `imp` should consume them when AUSH is the shell backend.

Useful fields:

- command;
- cwd;
- exit code;
- duration;
- stdout/stderr preview;
- byte and line counts;
- truncation state;
- risk classification;
- backend;
- sandbox labels;
- suggested next commands.

`imp` shell tool results should preserve this metadata rather than flattening it to text.

Conceptual result:

```json
{
  "exit_code": 101,
  "duration_ms": 1842,
  "stdout_preview": "...",
  "stderr_preview": "...",
  "stdout_truncated": true,
  "stderr_truncated": false,
  "suggested_next": [
    "cargo test -p imp-core shell_tool -- --nocapture",
    "rg -n \"shell_tool\" crates/imp-core tests"
  ],
  "risk_class": "build_or_test",
  "backend": "aush-daemon",
  "sandbox": {
    "enabled": true,
    "backend": "git-worktree"
  }
}
```

## Why it matters

Agents often make poor next moves because tool output is ambiguous. Structured shell metadata lets the model know:

- it saw truncated output;
- the command timed out;
- stderr was large;
- the operation was a build/test, not a user-facing error;
- the right next step is a narrower command.

This is more valuable than simply raising the output limit.

---

# Idea 3: output budgets as policy, not accident

`imp` already has to protect context. The shell tool should make output limits explicit.

The shell tool should return:

- stdout bytes returned;
- stdout bytes total if known;
- stderr bytes returned;
- stderr bytes total if known;
- truncation reason;
- whether the beginning, end, or both were kept;
- suggestions for narrowing.

Example assistant-visible note:

```text
stdout truncated: returned first 20,000 bytes of 4.8 MB.
Use rg/head/tail to narrow output before drawing conclusions.
```

The agent should be trained by the tool result to rerun narrower commands instead of guessing.

This applies whether the backend is Bash, Zsh, or AUSH.

---

# Idea 4: keep native tools; do not collapse to minimal Bash

The Gemini/just-bash discussion included a minimal tool surface:

```text
bash(command)
readFile(path)
writeFile(path, content)
```

This is useful as an interoperability baseline, but it should not be `imp`'s default architecture.

`imp` has good reasons to keep native governed tools:

| Tool area | Why shell is not enough |
| --- | --- |
| mana | durable work graph semantics, attempts, verify gates, dependencies |
| git | safer structured operations and policy around restore/commit/stage |
| edit | exact replacements are safer than whole-file rewrites |
| read | stable offset/limit and image support |
| web | provider policy, API keys, transcript extraction, response shape |
| audit_scan | consolidated scanners and normalized output |
| ask | explicit human coordination |
| secrets | metadata-only storage and safe injection |
| spawn | durable worker execution and orchestration boundaries |

The right principle:

> Minimize ambiguous tools, not total tools.

Shell remains essential for discovery, builds, tests, scripts, package managers, and existing developer workflows. Native tools remain essential for policy-aware operations.

## Optional eval mode

A minimal-tool mode could still be useful for experiments:

```bash
imp --tool-profile minimal
```

Possible profile:

```text
read
write/edit
shell
ask
```

This would be useful for evals or comparison against just-bash-like approaches, but not as the primary product mode.

---

# Idea 5: executable skills vs governed tools

Executable skills are a good idea, but `imp` should not treat every script as a trusted tool.

A useful split:

```text
AUSH skills = executable affordances
imp tools   = governed capabilities
imp skills  = instructions + optional governed tools
```

A simple script is fine for local glue:

```text
.aush/skills/summarize-test-failure
```

But a capability that accesses secrets, hits network, changes durable state, or returns sensitive data should be governed by `imp` policy.

Possible durable skill package:

```text
.imp/skills/github-pr/
  SKILL.md
  tool.sh
  policy.toml
```

Policy could declare:

```toml
[capabilities]
shell = false
filesystem = "read"
network = ["https://api.github.com/"]
secrets = ["github"]
```

This keeps extensibility simple without turning arbitrary scripts into privileged tools.

---

# Idea 6: explicit backend escalation

The Gemini discussion suggested transparent escalation from simulated shell to real VM.

For `imp`, escalation should be explicit and visible.

Possible execution backends:

```text
local
sandbox-worktree
sandbox-temp-copy
container
remote-builder
network-enabled-sandbox
production-shell
```

The user and agent should see boundary changes.

Bad:

```text
The agent silently switched from local dry shell to real networked VM.
```

Good:

```text
This command requires dependencies not available in the lightweight sandbox.
Escalate to container backend with network disabled? [y/N]
```

Tool results should include:

```json
{
  "backend": "container",
  "network": "disabled",
  "writes": "isolated",
  "apply_target": "/Users/asher/imp"
}
```

Trust depends on visible execution boundaries.

---

# Idea 7: virtual filesystem views for session and mana state

`imp` can benefit from file-like state views, but mutation should stay governed.

Potential read-only views:

```text
.imp/session/last-tool-call.json
.imp/session/context.json
.imp/session/summary.md
.mana/view/status.json
.mana/view/units/251.md
.mana/view/runs/latest.json
```

Agents are good at:

```bash
cat
rg
jq
ls
tree
```

So file projections can make state inspectable without adding bespoke tools for everything.

Boundary:

```text
Files are good for inspection.
Governed tools are better for mutation.
```

So:

```bash
cat .mana/view/status.json       # good
mana update 251 --notes "..."    # good
echo ... > .mana/index.yaml      # bad
```

---

# Idea 8: script tool remains separate

`imp` already has a proposal for script tool boundaries and policy. This direction should stay intact.

The product split should remain:

```text
imp run / worker execution = native Rust runtime path
script tool                = ephemeral bounded scripting
extension substrate         = durable packaged tools/hooks/commands
extend tool                 = authoring/management helper
```

`imp --sandbox` should not be implemented as a script tool. It is a runtime/workspace mode.

AUSH integration should not blur this boundary either. AUSH is a shell backend; the script tool is an agent-facing programmable transform surface; extensions are durable packages.

---

# Implementation sketch for `imp --sandbox`

## CLI flags

Potential flags:

```bash
imp --sandbox
imp --sandbox -p "..."
imp run --sandbox <unit-id>
imp --sandbox-backend worktree|temp-copy|container|remote
imp --sandbox-keep
imp --sandbox-apply=ask|never|on-pass
```

Default:

```text
--sandbox-apply=ask
```

Do not auto-apply by default.

## Core types

Conceptual Rust types:

```rust
struct SandboxSession {
    id: SandboxId,
    backend: SandboxBackend,
    source_root: PathBuf,
    sandbox_root: PathBuf,
    base_ref: Option<String>,
    dirty_source_policy: DirtySourcePolicy,
}

enum SandboxBackend {
    GitWorktree,
    TempCopy,
    Container,
    Remote,
}

enum SandboxApplyMode {
    Ask,
    Never,
    OnPass,
}
```

## Runtime integration

The sandbox should alter the execution context once, not require every tool to reinvent path rewriting.

Agent context should include:

```text
You are operating in an imp sandbox.
Sandbox root: /tmp/imp-sandbox/...
Apply target: /Users/asher/imp
Do not claim changes are applied until the sandbox diff is accepted.
```

Tool execution should receive cwd as the sandbox root by default.

File tools should read/write sandbox paths, not target paths.

Git tool should run in sandbox context unless explicitly inspecting target.

## Diff and apply

Needed operations:

```text
sandbox status
sandbox diff
sandbox apply
sandbox discard
sandbox keep
```

Could start as internal helpers before becoming user-facing commands.

## Verification

For normal one-shot sandbox work:

1. agent runs relevant checks in sandbox;
2. user accepts patch;
3. `imp` optionally reruns final check in target after apply.

For mana work:

1. worker verify may run in sandbox;
2. patch must apply to target;
3. mana close should verify in target unless explicitly deferred.

This avoids closing durable work for changes that only exist in a temp dir.

---

# Suggested phases

## Phase 1: design and shell result metadata parity

- Define an internal shell result metadata shape independent of backend.
- Preserve truncation, duration, exit code, and backend metadata.
- If AUSH is the backend, map its envelope into this shape.
- If Bash/Zsh is the backend, synthesize what `imp` can know.

## Phase 2: `imp --sandbox` worktree MVP

- Support clean git workspaces.
- Create temp worktree.
- Run one-shot prompt in sandbox cwd.
- Show diff and changed files.
- Ask apply/discard.
- Apply patch back to target with conflict handling.

## Phase 3: sandbox in interactive and mana worker modes

- Add TUI/chat sandbox status.
- Add `/sandbox diff/apply/discard` commands.
- Add `imp run --sandbox <id>`.
- Ensure mana close semantics do not close before target verify.

## Phase 4: dirty workspace support

- Detect dirty source workspace.
- Ask whether to include current changes.
- Track base patch vs agent patch.
- Avoid mixing user changes with sandbox changes in summaries.

## Phase 5: backend expansion

- temp-copy fallback;
- container backend;
- remote builder backend;
- possibly true overlay backend.

---

# Open questions

1. Should `imp --sandbox` be a top-level global flag, a subcommand, or both?
2. Should sandbox mode be available inside the TUI after session start?
3. How should dirty workspaces be handled in the first version?
4. Should `imp run --sandbox` ever auto-apply on verify pass, or always require approval?
5. What shell backend should sandbox mode default to: current shell, Bash, Zsh, or AUSH if available?
6. Should sandbox patches be stored as session artifacts?
7. Should mana attempts record sandbox root and patch path?
8. How should secrets be handled in sandbox commands?
9. Should network be disabled by default inside sandbox backends?
10. How should large generated files be represented in sandbox diffs?

## Bottom line

The strongest product demo is:

```bash
imp --sandbox -p "fix the failing test"
```

or from AUSH:

```bash
cargo test ...
?? fix this
```

Then `imp` works safely, verifies, and asks before applying the patch.

That gives the just-bash-style speed and shell fluency without sacrificing `imp`'s real advantage: policy-aware agent execution with durable mana integration.
