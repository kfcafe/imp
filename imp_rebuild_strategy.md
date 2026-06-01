# imp rebuild strategy

Status: draft
Audience: `imp` and Tower maintainers
Scope: `~/imp`

---

## Table of contents

- [Summary](#summary)
- [Primary recommendation](#primary-recommendation)
- [Why this is not a rewrite-from-scratch project](#why-this-is-not-a-rewrite-from-scratch-project)
- [What to preserve](#what-to-preserve)
- [What to change aggressively](#what-to-change-aggressively)
- [Target end-state for imp](#target-end-state-for-imp)
- [Key remaining decisions](#key-remaining-decisions)
- [Recommended work tracks](#recommended-work-tracks)
- [Suggested decomposition into smaller jobs](#suggested-decomposition-into-smaller-jobs)
- [Order of operations](#order-of-operations)
- [What success looks like](#what-success-looks-like)

---

## Summary

`imp` should remain a **pure Rust runtime**.

Tower as a whole may still become a Rust + TypeScript system if a future operator plane, evidence dashboard, or web-first control plane emerges. But `imp` itself is fundamentally a:

- terminal runtime
- worker engine
- provider streaming layer
- tool execution surface
- local session and context system
- policy and capability boundary

That is a Rust-shaped problem.

The right move is **not** to rewrite `imp` from scratch.
The right move is to **rebuild its internal seams** so the code more literally matches the architecture already described in Tower docs and `agent_design.md`.

---

## Primary recommendation

Keep `imp` pure Rust and refactor it in place.

More specifically:

1. keep the product/runtime identity of `imp`
2. split `imp-core` into smaller crates or sharply bounded modules
3. extract shared `mana ↔ imp` protocol types into a canonical contracts layer
4. introduce a clearer runner boundary for execution, sandboxing, and worktree lifecycle
5. make verification, review, and evidence first-class workflow outputs instead of runtime-adjacent behavior

---

## Why this is not a rewrite-from-scratch project

A full restart would throw away too much value.

### Valuable assets already exist

- `imp-llm` already solves real provider normalization and streaming problems
- the tool registry and built-in tool surface are broad and useful
- `imp` already understands modes, worker execution, and mana-backed orchestration
- the TUI is substantial enough that a rewrite would spend a long time regaining parity
- the docs and prior deep review already identify the real problems clearly

### The problem is not product identity

The problem is not that `imp` is conceptually wrong.
The problem is that some critical boundaries are still too implicit, and too much responsibility is concentrated inside a few large files.

---

## What to preserve

### 1. Keep `imp-llm`

Do not rebuild provider integration unless a provider-specific area truly needs cleanup.
This crate is already a strong independent asset.

### 2. Keep `imp` as the live runtime

`imp` should continue to own:

- context assembly
- model interaction
- turn loop
- tool invocation
- session-local state
- TUI interaction
- local adaptation while a run is active

### 3. Keep the current user-facing shape where possible

Preserve:

- `imp` CLI
- TUI workflows
- session model
- modes as a user-facing concept
- extension surface in principle, though capability policy should be tightened

### 4. Keep Rust as the implementation language

Do not add TypeScript to `imp` unless there is a genuinely separate web/control-plane product being created inside the same repo.

---

## What to change aggressively

### 1. Shrink `imp-core`

Current strongest hot spots:

- `crates/imp-core/src/agent.rs`
- `crates/imp-core/src/tools/mod.rs`
- `crates/imp-core/src/session.rs`
- `crates/imp-core/src/tools/mana.rs`
- `crates/imp-core/src/config.rs`

These should not continue to absorb more unrelated responsibilities.

### 2. Move cross-boundary types out of `imp-core`

Anything another worker, reviewer, or orchestrator must trust cold should not live only as ad hoc structs inside the runtime.

### 3. Harden capability enforcement

The deepest review already identified a serious mismatch between intended policy boundaries and actual runtime surfaces, especially around Lua and tool access.

### 4. Separate editing from verification more clearly

Even if both remain in Rust and inside the same repo, the workflow should treat them as distinct stages with distinct artifacts.

### 5. Introduce a runner boundary

Execution, sandboxing, worktrees, PTY mediation, and artifact capture should become a more explicit subsystem instead of being split informally between runtime and CLI helpers.

---

## Target end-state for imp

A good target shape is:

```text
imp/
  crates/
    imp-llm        # provider/model/auth/usage layer
    imp-runtime    # turn loop, events, orchestration
    imp-context    # retrieval, packing, compaction, prefill
    imp-tools      # tool contracts, registry, common helpers
    imp-policy     # deterministic capability and runtime policy
    imp-worker     # canonical mana worker runtime
    imp-ext        # guest runtime / extension boundary
    imp-tui        # terminal UI
    imp-cli        # entrypoint and operator commands
```

This does not mean all of these must be deployed or versioned separately right away.
It means the internal architecture should move toward those boundaries.

---

## Key remaining decisions

These are the most important decisions still to make explicitly.

### 1. Contracts location

Decision:
- Should shared workflow/protocol types live in a Tower-wide contracts crate, a `mana` contracts crate, or an `imp` crate that `mana` also depends on?

Recommendation:
- Prefer a Tower-level shared contracts crate so neither side conceptually owns the shared boundary.

### 2. Runner placement

Decision:
- Should the runner boundary live inside `imp` initially, or become a sibling `runner/` subtree in Tower immediately?

Recommendation:
- Start with an `imp` internal runner boundary if that reduces migration friction, but design the protocol as if it can move into a sibling `runner/` layer later.

### 3. Extension model

Decision:
- Should Lua remain the extension substrate long-term, or should it be wrapped in a stricter guest-runtime abstraction and treated as one possible backend?

Recommendation:
- Treat Lua as an implementation detail of a broader guest-runtime model. Tighten capabilities first; do not expand the extension model until capability boundaries are trustworthy.

### 4. Verification ownership

Decision:
- How much verification should happen inside `imp`, and how much should become explicit durable workflow state mediated by `mana`?

Recommendation:
- `imp` may execute verifiers, but `mana` should own the durable record of what was verified, what evidence exists, and whether completion is accepted.

### 5. Session memory vs durable memory

Decision:
- Which memories remain local/session-scoped in `imp` and which become durable substrate facts or artifact-linked memory in `mana`?

Recommendation:
- Keep `imp` memory ephemeral unless another worker must inherit it cold. Durable memory should be evidence-backed and externalized.

### 6. TUI refactor timing

Decision:
- Should the TUI be refactored early or only after runtime/protocol boundaries are cleaner?

Recommendation:
- Do the runtime/protocol refactor first. TUI should consume cleaner runtime/state boundaries rather than driving them.

---

## Recommended work tracks

### Track A — contracts and ownership boundary

Goal:
- formalize the `mana ↔ imp` contract as code, not just docs

Owns questions like:
- assignment types
- worker results
- verifier results
- evidence bundles
- review decisions
- durable artifact references

### Track B — `imp-core` decomposition

Goal:
- split the monolithic runtime into cleaner modules/crates without changing core behavior prematurely

Owns questions like:
- what belongs in `imp-runtime`
- what belongs in `imp-context`
- what belongs in `imp-tools`
- what belongs in `imp-policy`

### Track C — runner, worktree, and sandbox substrate

Goal:
- make execution boundaries explicit and attributable

Owns questions like:
- command execution protocol
- worktree lifecycle
- sandbox profile model
- PTY mediation
- artifact capture around execution

### Track D — verification, review, and evidence pipeline

Goal:
- make the workflow candidate → evidence → gate explicit

Owns questions like:
- verify stage outputs
- skeptic/review stage outputs
- what artifacts must exist before a unit can be considered done
- how `imp` and `mana` divide responsibility for verification lifecycle

### Track E — capability policy and extension hardening

Goal:
- ensure modes, tool boundaries, Lua/guest runtime capabilities, and execution policy actually line up

Owns questions like:
- runtime policy engine shape
- guest runtime restrictions
- environment access defaults
- command/network exposure rules

### Track F — migration and compatibility rollout

Goal:
- sequence the work without breaking the product or forcing a flag day rewrite

Owns questions like:
- temporary adapters
- crate migration order
- compatibility layers
- verification strategy during the refactor

---

## Suggested decomposition into smaller jobs

Below is the recommended way to break the rebuild into smaller jobs. These are not all meant to be created immediately, but they show the intended shape.

## Track A — contracts and ownership boundary

### Small jobs

1. Define the first canonical shared types
   - `TaskSpec`
   - `WorkerAssignment`
   - `WorkerOutcome`
   - `VerifierResult`
   - `EvidenceBundle`

2. Map existing runtime structs to the new contract types
   - especially `imp-core/src/mana_worker.rs`

3. Decide which durable objects are still runtime-local by mistake
   - examples: assignment fields, retry summaries, verify results, artifact refs

4. Add compatibility adapters so old code can keep compiling while migration happens

## Track B — `imp-core` decomposition

### Small jobs

1. Split `agent.rs` into:
   - turn loop
   - tool execution coordinator
   - event/timing emission
   - follow-up / continuation logic

2. Split `tools/mod.rs` into:
   - tool traits and registry
   - file helpers
   - output truncation and diff helpers
   - checkpoint/file-history helpers
   - validation helpers

3. Move `mana_worker.rs` into a dedicated worker-oriented module/crate

4. Decide whether `session.rs` should split into:
   - persistence
   - tree/branch model
   - title/summary heuristics
   - replay/export

## Track C — runner, worktree, and sandbox substrate

### Small jobs

1. Define a local runner protocol and data model
2. Wrap command execution in a typed request/response boundary
3. Model sandbox profiles explicitly
4. Model worktree allocation/cleanup as a durable or semi-durable lease concept
5. Bind execution artifacts to run IDs, worktree IDs, and sandbox IDs

## Track D — verification, review, and evidence pipeline

### Small jobs

1. Define the minimum evidence bundle for a coding run
2. Separate edit-stage outputs from verify-stage outputs
3. Define what the skeptic/review stage consumes and emits
4. Decide which verification outputs live only in `imp` vs which must be stored durably in `mana`
5. Add a completion gate that reasons from evidence rather than narration

## Track E — capability policy and extension hardening

### Small jobs

1. Audit mode/tool visibility against actual runtime capability
2. Introduce a deterministic runtime policy interface
3. Tighten Lua/guest runtime tool access
4. Tighten env/network/shell access defaults
5. Make cancellation propagate through active tool execution

## Track F — migration and compatibility rollout

### Small jobs

1. Decide the migration order of new crates/modules
2. Add temporary re-exports and shims
3. Define which user-visible behavior must remain stable throughout
4. Write focused verification commands for each migration slice
5. Add a rollback plan for each high-risk refactor seam

---

## Order of operations

Recommended order:

1. contracts and ownership boundary
2. `imp-core` decomposition plan
3. runner protocol and local execution boundary
4. verification/evidence model
5. capability policy hardening
6. migration and compatibility rollout
7. only then deeper TUI-facing cleanup based on the cleaner runtime boundaries

This order minimizes wasted motion.

---

## What success looks like

The rebuild is successful when:

- `imp` is still a Rust runtime and feels like the same product externally
- the `mana ↔ imp` boundary is represented by explicit shared types
- `imp-core` is no longer the place where every new responsibility lands
- execution happens through a clearer runner/worktree/sandbox boundary
- verification and review produce explicit artifacts instead of mostly prose and exit codes
- modes and capability boundaries are enforced consistently across native tools and extensions
- another agent can pick up the work cold from structured artifacts and durable state instead of reading long transcripts or ad hoc notes

---

## Final recommendation

Do not restart `imp`.

Refactor it in place around these principles:

- pure Rust runtime
- protocol-first boundary with `mana`
- smaller runtime modules
- explicit runner boundary
- evidence-driven workflow stages
- deterministic capability enforcement

That is the rebuild strategy most aligned with the current codebase, the existing docs, and Tower's long-term architecture.
