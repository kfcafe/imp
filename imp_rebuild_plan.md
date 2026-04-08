# imp rebuild plan

Status: active planning draft
Audience: `imp` maintainers
Scope: `~/tower/imp`

This file converts `imp_rebuild_strategy.md` into an execution plan with a strict order of operations.

The primary recommendation remains:

> do not rewrite `imp`; refactor it in place around clearer seams

---

## Mission

`imp` should remain the live worker runtime for Tower.

It should own:
- context assembly
- model interaction
- turn loop behavior
- tool invocation
- local planning and adaptation during a run
- operator-facing execution UX
- runtime policy enforcement

It should stop being the accidental long-term home for durable workflow truth that another worker must inherit cold.

---

## Decisions already made

### 1. `imp` stays pure Rust

No TypeScript-first runtime split for the active rebuild.

### 2. Shared workflow types move out of `imp-core`

The `mana ↔ imp ↔ runner` boundary should be expressed through the Tower shared contracts crate.

### 3. Runner concerns are real architecture, not glue

Execution, worktrees, sandboxing, PTY mediation, and artifact capture must converge on an explicit runner boundary.

### 4. `imp-core` should shrink in authority

Large central files should stop accumulating unrelated concerns.

### 5. TUI refactor trails runtime refactor

The UI should consume cleaner seams, not define them.

---

## Required end-state inside `imp`

`imp` should become legible as a set of bounded runtime concerns:
- runtime / turn loop
- context assembly and compaction
- tool contracts and registry
- capability and runtime policy
- worker runtime for mana assignments
- extension / guest runtime boundary
- TUI and CLI adapters

The exact crate count can evolve, but these seams should become visible in code.

---

## Order of operations

## Phase I0 — freeze boundaries and hot spots

Goal:
- stop `imp-core` from absorbing more architectural drift during the rebuild

Deliverables:
- identify current hot spots and their target seams
- document which types should migrate to shared contracts
- document current runner-adjacent behavior that is transitional

Hot spots already visible from repo docs and search:
- `imp/crates/imp-core/src/agent.rs`
- `imp/crates/imp-core/src/tools/mod.rs`
- `imp/crates/imp-core/src/session.rs`
- `imp/crates/imp-core/src/tools/mana.rs`
- `imp/crates/imp-core/src/config.rs`
- `imp/crates/imp-core/src/mana_worker.rs`

## Phase I1 — adopt shared contracts from Tower

Goal:
- stop defining trusted cross-boundary shapes only inside the runtime

Deliverables:
- `tower-contracts` dependency wired into `imp`
- worker assignment/outcome mappings
- verifier/evidence/result mappings
- temporary adapters so behavior stays stable during migration

Important rule:
- land the shared type adoption before large file splits that depend on those types

## Phase I2 — isolate the canonical worker runtime boundary

Goal:
- make the mana-assigned worker path a first-class seam

Deliverables:
- move worker-facing types and orchestration helpers away from ad hoc placement
- define a clearer worker runtime module/crate boundary
- ensure worker runtime consumes shared assignment/result types

Why early:
- this is where the `mana ↔ imp` contract becomes concrete in live execution

## Phase I3 — split `agent.rs` by responsibility

Goal:
- reduce authority concentration in the main runtime file

Target seams:
- turn loop
- tool execution coordinator
- event/timing emission
- follow-up / continuation logic
- run completion and result assembly

Deliverables:
- smaller modules with preserved behavior
- focused tests for extracted logic

## Phase I4 — split tool system into clearer layers

Goal:
- stop `tools/mod.rs` from being a grab-bag

Target seams:
- tool traits and registry
- file/edit helpers
- output truncation / rendering helpers
- validation helpers
- checkpoint / rollback helpers
- mana-specific tool integration

Deliverables:
- clearer internal module boundaries
- tool contract types separated from helper internals
- easier policy enforcement per tool family

## Phase I5 — separate context/session concerns from runtime loop concerns

Goal:
- make context assembly and session persistence independently legible

Target seams:
- context packing / compaction / retrieval
- session persistence and branching
- replay/export
- summary/title heuristics

Deliverables:
- clearer context and session modules or crates
- reduced coupling between long-session behavior and turn execution

## Phase I6 — introduce the runner-facing execution boundary

Goal:
- make command execution, worktree use, sandboxing, and artifact capture explicit

Deliverables:
- runner request/result consumption via shared contracts
- local execution adapter integration
- command/tool execution artifacts bound to run context
- fewer stringly shell/orchestration assumptions in runtime-critical paths

Important rule:
- this phase should target the stable runner protocol established at Tower level

## Phase I7 — harden capability and extension policy

Goal:
- align visible modes with real runtime authority

Deliverables:
- deterministic policy interfaces
- tighter tool visibility and execution gating
- stricter guest runtime / Lua capability boundaries
- clearer environment/network/shell defaults
- better cancellation propagation through active operations

## Phase I8 — make evidence-oriented workflow outputs visible in runtime and UX

Goal:
- treat candidate, verification, and review as explicit workflow outputs

Deliverables:
- structured result assembly for candidate work
- normalized verifier output handling
- clearer handoff of evidence refs back to `mana`
- only then TUI/CLI polish to surface the cleaner runtime state

---

## Dependency rules for imp work

### Must happen first
1. shared contracts adoption
2. worker runtime boundary cleanup
3. `agent.rs` decomposition
4. tools decomposition
5. context/session separation
6. runner-boundary integration
7. policy hardening
8. evidence-oriented output and UX cleanup

### Can proceed in parallel later
After I2, limited parallel work is reasonable across:
- `agent.rs` extraction
- tools helper extraction
- session/context cleanup

But do not fork competing designs for:
- worker assignment/result types
- runner request/result handling
- verifier/evidence output schemas
- policy authority model

---

## Suggested imp epics

### Epic A — adopt shared contracts in imp

Jobs should cover:
- inspect current worker structs and handoff paths
- map runtime types to shared contracts
- land temporary adapters and focused tests

### Epic B — canonical worker runtime seam

Jobs should cover:
- isolate mana worker execution path
- move worker-oriented code out of ambiguous placement
- verify headless worker behavior still works

### Epic C — runtime loop decomposition

Jobs should cover:
- extract turn loop pieces from `agent.rs`
- isolate follow-up / continuation behavior
- preserve current tests or add focused ones where missing

### Epic D — tool system decomposition

Jobs should cover:
- split registry/contracts from helpers
- isolate truncation/validation/checkpoint logic
- make mana tool integration less coupled to unrelated helpers

### Epic E — context and session separation

Jobs should cover:
- isolate compaction/retrieval logic
- isolate persistence/branching/replay logic
- reduce cross-coupling with agent loop

### Epic F — runner-boundary integration

Jobs should cover:
- consume runner requests/results
- wire local adapter expectations
- bind execution artifacts and IDs into runtime flow

### Epic G — capability and extension hardening

Jobs should cover:
- audit visible mode vs actual capability
- tighten Lua / guest runtime access
- improve cancellation and policy enforcement

### Epic H — evidence-oriented runtime outputs and operator surfaces

Jobs should cover:
- structure candidate/result assembly
- normalize verification output handling
- then update TUI/CLI surfaces to present the cleaner artifacts

---

## Verify philosophy for imp rebuild work

Use narrow verification.

Prefer:
- `cargo check -p imp-core`
- focused `cargo test -p imp-core <target>`
- targeted `cargo check -p imp-cli` or `cargo check -p imp-tui` only when a slice touches them
- existence checks paired with focused compile/test checks

Avoid broad verify gates like:
- whole-workspace builds for every slice
- large unscoped test runs unless the change is truly cross-cutting

---

## What not to do

Do not:
- rewrite `imp` from scratch
- let TUI cleanup lead the runtime architecture
- keep cross-project trusted types inside `imp-core`
- treat runner concerns as temporary glue forever
- mix policy hardening with unrelated broad refactors when focused slices would do
- expand extension power before capability boundaries are trustworthy

---

## Success condition

The `imp` rebuild is successful when:
- `imp` still feels like the same runtime externally
- shared workflow truth no longer lives primarily in `imp-core`
- the worker path consumes explicit shared contracts
- runtime, context, tools, policy, and worker concerns are visibly separated
- execution uses a clearer runner/worktree/sandbox boundary
- evidence and verification outputs are structured enough for `mana` to record and another worker to inherit cold
