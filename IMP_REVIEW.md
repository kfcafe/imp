# IMP Review

Date: 2026-03-25
Status: external review memo

## Purpose

This document reviews `imp` against the current coding-agent landscape — especially Cursor, Cline, Roo Code, OpenHands, OpenCode, Aider, Factory, and related tools.

The goal is not to ask `imp` to copy those products. The goal is to identify:

1. what `imp` already does unusually well,
2. what power is present but underexposed,
3. what is still genuinely missing,
4. how `imp` should evolve without blurring into `mana`, Wizard, or Familiar.

---

## Executive summary

`imp` is stronger than a casual read of the README suggests.

It already has several serious runtime primitives that many agent tools either do not have or only expose through polished UI:

- append-only branching sessions with resume/fork,
- context masking + compaction + auto-resume,
- distinct runtime modes (`worker`, `planner`, `reviewer`, `orchestrator`),
- pre-edit file snapshots and rollback primitives,
- file-read tracking with staleness warnings,
- persistent memory across sessions,
- session search,
- hook-based policy interception,
- a proper UI abstraction for approvals and structured prompts,
- Lua-powered extensibility.

The biggest opportunity is **not** to reinvent the runtime. It is to **surface and unify the power that already exists**, then add a small number of missing primitives where the market now expects them.

The clearest next moves are:

1. productize checkpoints,
2. make planning first-class and visible,
3. formalize approval policies,
4. add LSP/diagnostic intelligence,
5. support detached/background local work more explicitly,
6. tighten the built-in edit/test/fix loop,
7. keep docs and surfaced defaults aligned with the capabilities already implemented.

---

## What `imp` already does unusually well

### 1. Session architecture is strong

`imp` already has a better session model than many OSS coding agents:

- append-only JSONL persistence,
- resume recent session,
- open specific session,
- branch navigation,
- fork from any point,
- session picker,
- in-memory ephemeral mode when persistence is not desired.

This is a real foundation for long-running work, provenance, replay, and future background execution.

### 2. Context management is not naive

`imp` already implements serious context engineering:

- observation masking,
- compaction thresholds,
- preserved reasoning and tool-call structure,
- auto-resume after compaction,
- original task reinjection.

That is materially ahead of tools that simply let context bloat until the model degrades.

### 3. Runtime roles are well thought through

The role and mode system is a differentiator:

- `worker`
- `planner`
- `reviewer`
- `orchestrator`
- `auditor`
- `full`

This is better than a one-size-fits-all “agent mode.” It creates a clean basis for future subagents and safer delegation.

### 4. Safety primitives already exist

`imp` is not starting from zero on safety.

It already has:

- file-read tracking,
- stale-file detection,
- unread-edit warnings,
- pre-edit original-content snapshots,
- rollback primitives,
- hook interception before tool calls,
- UI confirm/select/input abstractions.

That means the real missing layer is policy and UX, not core mechanism.

### 5. Persistent learning is already present

The combination of:

- `memory.md`,
- `user.md`,
- learning nudges,
- session indexing,
- `session_search`,

means `imp` is already thinking beyond single-session statelessness.

This is a meaningful differentiator in OSS.

### 6. Extensibility is serious, not decorative

`imp` has a real extension architecture:

- Lua runtime,
- hook system,
- custom tools,
- custom commands,
- configurable shell tools,
- layered resource discovery.

This gives it a path to ecosystem growth without bloating the core runtime.

---

## What is underexposed today

These are capabilities that appear to exist in the implementation or architecture, but are not yet surfaced as clearly as they should be.

### A. Checkpoint/rollback safety

`imp` already stores pre-edit originals and can roll back files, but the user-facing concept is weak compared with the market’s expectation of “checkpoints.”

### B. Planning as a visible workflow

`imp` already has planner/orchestrator roles, but planning is still easier to understand by reading the code than by using the product.

### C. Approval flows

The runtime has UI request primitives and before-tool-call interception, but there is not yet a strong default policy story.

### D. Memory and session search

These are strategically important features, but they are easy to miss. They should feel like signature capabilities.

### E. Built-in tool surface consistency

Some implemented capabilities are not obviously surfaced through the canonical builder/tool list. The runtime should make sure the default registered built-ins match the public feature story.

Examples worth reviewing:

- `memory`,
- `session_search`,
- `multi_edit` / edit delegation behavior,
- any approval-related or policy-related affordances.

This is partly a docs issue and partly a default-registration issue.

---

## Recommendations

## P0 — Highest priority

### 1. Productize checkpoints as a first-class user feature

**Why**

The market now expects safe rewind:

- Cursor checkpoints,
- Roo checkpoints,
- branch/restore concepts in several tools.

`imp` already has the underlying file snapshot mechanism. It should expose it as a first-class concept.

**Suggested shape**

- Automatically create named checkpoints before major edits or tool waves.
- Expose `checkpoint list`, `checkpoint diff`, `checkpoint restore` in CLI/TUI.
- Show checkpoints in the session tree/timeline.
- Distinguish between:
  - conversation branch points,
  - file-state checkpoints.

**Goal**

Turn an internal safety mechanism into a trust-building user feature.

---

### 2. Make planning visible and sticky

**Why**

Modern agent tools increasingly separate planning from execution.

`imp` already has planner/orchestrator roles, but planning should become a clear user workflow rather than an implied internal mode.

**Suggested shape**

- Add an explicit Plan Mode in TUI/CLI.
- Plans should produce a structured artifact:
  - steps,
  - likely files,
  - risks,
  - checks to run.
- Maintain a live todo/checklist during execution.
- Let the user approve, edit, or discard the plan before action.

**Goal**

Make planning observable, editable, and reusable.

---

### 3. Add a first-class approval policy layer

**Why**

The primitives exist, but policy is still mostly implicit.

The best coding agents now distinguish between:

- can do silently,
- should ask,
- should never do.

**Suggested shape**

Configurable per tool or tool class:

- read-only tools → allow,
- file writes → ask or allow,
- shell execution → ask or scoped allow,
- network/web → ask,
- destructive shell patterns → block or hard-confirm.

Support policy scopes:

- built-in defaults,
- user config,
- project config,
- temporary session override.

**Goal**

Build a coherent trust model on top of existing confirm/select/hook infrastructure.

---

### 4. Add LSP / diagnostics integration

**Why**

This is the clearest real capability gap versus newer runtimes.

Tree-sitter and search are strong, but serious code agents increasingly benefit from:

- diagnostics,
- go-to-definition,
- references,
- rename/symbol operations,
- language-aware edits.

**Suggested shape**

Start with read-oriented capabilities:

- diagnostics list,
- goto definition,
- find references,
- hover/signature.

Then consider write-side affordances later:

- rename symbol,
- code actions.

**Goal**

Improve precision and reduce exploratory token burn.

---

### 5. Support detached/background local execution explicitly

**Why**

`imp` already has the persistence model needed for resumable work, but the workflow still feels foreground-first.

Users increasingly expect:

- start a job,
- detach,
- come back later,
- inspect progress,
- reattach.

**Suggested shape**

- `imp run --detach` or equivalent for local background work,
- list running local sessions/jobs,
- reattach and inspect logs,
- TUI status view for detached sessions.

This should stay local-first and not try to become Familiar.

**Goal**

Give long-running work a first-class local operational model.

---

### 6. Tighten the built-in test/lint/fix loop

**Why**

Aider in particular proved the value of a tight edit → test → fix loop.

`imp` has all the raw ingredients, but this workflow should feel more native.

**Suggested shape**

- Project-configured checks:
  - lint,
  - typecheck,
  - test,
  - build.
- Scoped checks for fast feedback.
- Failure output routing back into the next turn.
- Better distinction between:
  - developer feedback checks,
  - mana verify gate.

**Goal**

Make the runtime naturally converge on correct code, not just changed code.

---

### 7. Align surfaced defaults with implemented capabilities

**Why**

The internal architecture appears broader than the canonical surface in some places.

That creates confusion:

- users underuse features,
- docs drift from runtime behavior,
- extensions feel more necessary than they should.

**Suggested audit**

Review the canonical builder and default tool registration path against the intended product surface:

- `memory`
- `session_search`
- `multi_edit`
- any review/planning-specific helpers
- anything listed in docs but not consistently surfaced

**Goal**

Reduce “hidden power” and tighten the public contract of the runtime.

---

## P1 — Strong next differentiators

### 8. Session replay and provenance UX

`imp` already stores rich session history.

Build better ways to inspect it:

- replay timeline,
- diff two branches of a session,
- show tool-call provenance,
- export a run as a structured bundle.

This supports debugging, evals, demos, and trust.

---

### 9. Packaged skills/tools with stronger metadata

The extension system is real. The next step is better packaging and distribution:

- versioning,
- dependency requirements,
- compatibility metadata,
- install/update/disable commands,
- discoverable registries.

Do this without turning `imp` into OpenClaw-style plugin sprawl.

---

### 10. First-class subagents

The role system is ready for it.

A future `imp` could support local specialist subagents like:

- planner,
- reviewer,
- researcher,
- fixer.

This should build on the existing mode/role discipline rather than bypass it.

---

## P2 — Later

### 11. Session sharing and portable run artifacts

Useful later, especially once Wizard and Familiar need better cross-surface handoff.

### 12. Mobile/chat ingress

This is not a core `imp` job. If it ever matters, it should remain a thin integration layer, not a shift in product identity.

---

## Recommended boundaries

`imp` should remain:

- the worker runtime,
- the tool-executing local agent,
- the context/session engine,
- the extension host.

`imp` should **not** become:

- a task tracker,
- a cloud control plane,
- a team dashboard,
- a messaging gateway,
- a generic orchestration substrate.

Those belong to `mana`, Wizard, and Familiar.

---

## How `imp` should support the rest of the stack

### For `mana`

`imp` should expose:

- explicit plan artifacts,
- explicit verify/check outputs,
- stronger provenance,
- clean reviewer/planner worker roles.

That makes `mana` smarter without bloating it.

### For Wizard

`imp` should emit runtime state that is easy to supervise:

- checkpoints,
- current plan,
- active todo item,
- current phase,
- recent tool events,
- detached-session state.

### For Familiar

`imp` should be a disciplined worker that can run unattended:

- policy-aware,
- observable,
- resumable,
- easy to meter,
- predictable in headless mode.

---

## Final take

`imp` does **not** need a dramatic identity change.

It already looks like a serious open coding-agent runtime. The next win is to:

- expose the power it already has,
- formalize policy and planning,
- add LSP-grade precision,
- and make long-running local work feel first-class.

That would make `imp` not just technically capable, but obviously capable to users and contributors.
