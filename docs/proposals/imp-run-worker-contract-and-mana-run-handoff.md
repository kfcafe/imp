# Imp Run Worker Contract and Mana Run Handoff

> Proposal for `28.1.1` — April 2026
>
> Defines how `imp run` should become the canonical single-unit worker
> runtime while preserving `mana run` as the durable parallel
> orchestrator, and preserving imp's native `mana` tool as the intended
> first-class user-facing orchestration UX.
>
> Depends on: `28.1` (worker-runtime epic), archived `29.7` (no new
> native imp orchestration tool action)

---

## Executive Summary

The intended stack is:

```text
imp native mana tool = first-class orchestration UX
mana run            = orchestration engine / durable parallel dispatch
imp run             = canonical single-unit worker runtime
```

This proposal strengthens the bottom two layers in service of the top
layer.

It does **not** introduce a new native imp orchestration tool action.
Archived unit `29.7` already established that imp should not grow a
separate durable orchestration substrate alongside mana.

Instead:
- imp's native `mana` tool remains the primary way a human or agent
  inside imp initiates orchestration work.
- `mana run` remains responsible for selecting ready work, scheduling
  units in parallel, batching verification, and recording durable run
  state.
- `imp run` becomes the strong, explicit worker/runtime contract that
  `mana run` dispatches for one unit at a time.

---

## Current State

### 1. Imp already has the intended first-class orchestration UX

`imp/crates/imp-core/src/tools/mana.rs` already provides:
- `mana(action="run")`
- `mana(action="run_state")`
- `mana(action="evaluate")`
- `mana(action="logs")`
- background run tracking
- follow-up summaries
- widget/status integration

This is the correct first-class orchestration surface inside imp.

### 2. Mana already owns durable orchestration

`mana/crates/mana-cli/src/commands/run/mod.rs` already owns:
- ready-unit selection
- dependency ordering
- parallel scheduling
- run summaries/state
- await-verify handling
- batch verify
- durable run accounting

This is the correct orchestration substrate.

### 3. Imp's worker path is still thinner and more ad hoc than it should be

`imp/crates/imp-cli/src/main.rs` currently implements `imp run <unit-id>`
through CLI-local logic:
- `load_mana_unit()` walks upward to find `.mana/`
- scans markdown files and matches filenames
- parses unit frontmatter manually into `ManaUnit`
- assembles prompt/task context in CLI code
- runs verify inline
- shells out to `mana close`

That means the actual worker/runtime boundary is weaker than the
orchestration and UX above it.

---

## Design Goal

Make `imp run` the canonical worker/runtime for **one** mana unit,
without changing the ownership boundary:

- **imp native mana tool** should be the first-class user-facing path.
- **mana run** should remain the orchestrator.
- **imp run** should be the worker contract underneath.

Said differently:

> We are not making direct CLI `imp run` the primary product surface.
> We are strengthening it so the native mana tool and mana orchestrator
> rest on a better worker substrate.

---

## Ownership Boundary

### Imp native mana tool owns
- the first-class orchestration UX inside imp sessions
- interactive invocation of mana orchestration
- presentation of run state, evaluate output, logs, and follow-up
  summaries
- status/widget integration

### Mana run owns
- durable orchestration of many units
- ready-target selection
- dependency ordering
- bounded parallel execution
- verify batching and await-verify policy
- durable run state and outcomes
- unit closure/failure accounting

### Imp run owns
- execution of one assigned mana unit
- canonical loading of unit execution context
- assembly of worker task context and prefill
- agent execution for that unit
- structured worker progress and final outcome reporting

### Imp must not own
- multi-unit scheduling policy
- dependency graph orchestration
- a second durable orchestration substrate separate from mana

This preserves the 29.7 decision.

---

## Primary UX Rule

The primary orchestration path should be:

```text
imp session
  -> native mana tool
    -> mana run orchestration
      -> parallel imp run workers
```

This has two implications:

1. The native `mana` tool in imp should remain the best-feeling way to
   start and inspect orchestration.
2. The worker/runtime improvements to `imp run` should be designed to
   support that orchestration path first, not to turn direct CLI use
   into the main product story.

Direct `imp run <id>` remains valuable, but as a secondary direct-entry
path for running a single unit.

---

## The Target Imp Run Contract

`imp run` should become a stable worker contract with explicit machine
and human modes.

### Human-facing shape

```bash
imp run <unit-id>
```

This should remain a convenient direct-entry path for one unit, with
clear progress, blocker, and outcome messaging.

### Machine-facing shape

This is what `mana run` should dispatch:

```bash
imp run <unit-id> \
  --mana-dir /path/to/.mana \
  --protocol worker-json \
  --defer-verify
```

The exact flag names may vary, but the contract must provide these
concepts:
- explicit mana root / unit scope
- explicit machine-readable protocol
- explicit verify handoff semantics
- explicit final worker outcome

---

## Required Worker Contract Semantics

### 1. Canonical unit resolution

`imp run` must not rely on ad hoc markdown filename matching as its
long-term substrate.

Instead it should consume a canonical mana execution bundle or canonical
mana-core loading path that provides:
- id
- title
- description
- acceptance
- verify
- notes
- attempts
- decisions
- dependency summaries
- explicit files/paths
- future facts/wiki context when relevant

This may be implemented either as:
- a new mana-core API returning an execution bundle, or
- a shared contract type introduced at the imp↔mana boundary.

But the important rule is:

> `imp run` should load execution data through a canonical mana contract,
> not through markdown scraping as the intended substrate.

### 2. Stable machine protocol

The machine-facing mode of `imp run` should emit a stable event stream.

Representative event types:
- `worker_start`
- `unit_loaded`
- `progress`
- `thinking`
- `tool`
- `warning`
- `worker_result`

The exact schema can be refined during implementation, but it should be
versionable and explicit enough for `mana run` to consume without
depending on imp-internal implementation details.

### 3. Structured final outcome

`imp run` should end with a structured outcome like:
- `completed`
- `awaiting_verify`
- `blocked`
- `failed`
- `cancelled`

With metadata such as:
- summary
- error
- tool_count
- turns
- tokens/cost when available
- runtime/model info
- optional touched paths if practical

Example:

```json
{
  "type": "worker_result",
  "unit_id": "28.1.2",
  "status": "awaiting_verify",
  "summary": "Implemented the requested change and left the unit ready for verify.",
  "error": null,
  "tool_count": 14,
  "turns": 6,
  "tokens": 18234,
  "cost": 0.12,
  "model": "gpt-5.4"
}
```

### 4. Explicit verify handoff

In orchestrated runs:
- `imp run` should not own final close semantics
- `imp run` should not shell out to `mana close`
- `mana run` should remain responsible for verify batching,
  await-verify handling, and durable outcome accounting

That means orchestrated worker mode should support explicit verify
handoff such as `--defer-verify`.

For direct human use, `imp run <id>` may still offer convenience modes
that run verify and optionally close, but those are secondary wrappers,
not the worker contract used by orchestration.

---

## Human Mode vs Worker Mode

The strengthened runtime should support two closely-related modes on the
same execution substrate.

### Worker mode
Used by `mana run`.

Characteristics:
- machine-readable output
- explicit mana root
- explicit verify deferral
- no shelling out to `mana close`
- structured final outcome for orchestrator consumption

### Direct human mode
Used as a convenience entrypoint.

Characteristics:
- human-readable progress
- clear unit assignment summary
- clearer blocker reporting
- optional inline verify
- optional close-on-pass convenience, if retained

The two modes should share the same underlying worker runtime. The human
path should be a wrapper, not a second independent implementation.

---

## Native Mana Tool Implications

Because imp's native `mana` tool is the intended first-class UX, it must
present the layering clearly.

Its summaries, widget content, and follow-up text should reflect:
- the native mana tool is the orchestration surface
- mana is the durable run engine
- imp workers are the execution layer underneath

It should not feel like a thin pass-through to a shell command, nor
should it imply that imp itself owns multi-unit orchestration policy.

Representative model:

```text
native mana tool -> start/run/inspect orchestration
mana run         -> coordinate units in parallel
imp run workers  -> execute one unit each
```

This is a messaging and UX requirement, not only an implementation one.

---

## Non-Goals

This proposal does **not** do the following:

1. Add a new native imp orchestration tool action for subagent spawning.
2. Move dependency scheduling or orchestration policy into imp.
3. Replace `mana run` with direct imp parallel execution.
4. Collapse the imp native mana tool and `imp run` into one undifferentiated layer.
5. Keep markdown filename scraping as the intended long-term worker contract.

---

## Migration Plan

### Phase 1 — write the contract
- Specify the architecture and ownership split.
- Specify machine-facing `imp run` contract.
- Specify verify/close handoff semantics.

### Phase 2 — extract the worker runtime in imp
- Move CLI-local headless worker logic into a reusable imp-core runtime.
- Keep imp-cli as the wrapper.
- Add machine-readable worker mode.

### Phase 3 — replace ad hoc unit loading
- Move from markdown scanning toward canonical mana execution loading.
- Ensure worker context assembly uses the contract rather than filename matching.

### Phase 4 — wire mana run onto the contract
- `mana run` continues parallel orchestration.
- It invokes the strengthened `imp run` worker contract.
- It retains run accounting, batch verify, and final outcomes.

### Phase 5 — align top-level UX
- Polish imp native mana tool wording and state surfaces.
- Keep direct `imp run <id>` polished as a secondary direct-entry path.
- Document the stack clearly.

---

## Acceptance Criteria for the Architecture

This proposal is successful if later implementation yields all of the following:

1. A human inside imp experiences the native `mana` tool as the primary
   orchestration surface.
2. `mana run` still owns durable orchestration and parallel dispatch.
3. `imp run` is the canonical worker/runtime for one unit.
4. Orchestrated worker mode no longer depends on ad hoc markdown scanning
   and shell side effects as the intended long-term contract.
5. Direct `imp run <id>` remains useful, but clearly secondary to the
   native mana tool for orchestration UX.

---

## Mapping to Current Work

- `28.1.1` — this specification
- `28.1.2` — extract imp worker runtime and strengthen `imp run`
- `28.1.3` — integrate `mana run` with the strengthened worker contract
- `28.1.4` — polish secondary direct `imp run` UX/docs
- `28.1.5` — make the native mana tool the clear first-class orchestration UX
