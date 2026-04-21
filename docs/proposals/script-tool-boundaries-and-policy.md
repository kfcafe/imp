# Script Tool Boundaries and Policy for imp

> Proposal for `44.2` — April 2026
>
> Defines the script tool as a distinct agent-facing execution surface,
> separate from durable extensions and separate from imp's native Rust
> worker/runtime path.

---

## Executive Summary

imp needs a way to let the agent perform one-off programmable transforms
without turning every transient script into a durable extension and without
confusing that surface with worker execution.

This proposal defines a **script tool** as:
- an explicit agent-facing tool for **ephemeral, run-scoped scripting**
- owned and policy-enforced by the **Rust host**
- distinct from the durable **extension substrate**
- distinct from **`extend`**, which is an authoring/management surface
- distinct from **`imp run`**, which remains the native Rust worker/runtime path

The high-level split should be:

```text
imp run / worker execution      -> native Rust runtime path
script tool                     -> ephemeral agent-facing programmable tool
extension substrate             -> durable packaged tools/hooks/commands
extend tool                     -> authoring/management helper
```

This keeps the product legible:
- the agent uses the **script tool** for one-off transforms during a run
- durable packaged behavior lives in **extensions**
- worker execution remains part of the **Rust runtime**, not scripting

---

## Problem

Today there is no dedicated script tool in imp.

The closest nearby surface is `extend`, but `crates/imp-core/src/tools/extend.rs`
shows that it currently does something else:
- returns Lua reference docs
- returns skill-writing reference docs
- creates/patches/deletes skill files

That is an authoring helper, not a runtime scripting tool.

At the same time, the current Lua bridge (`crates/imp-lua/src/bridge.rs`) proves
that guest code can register tools and call back into host capabilities, but that
surface belongs to the durable extension substrate, not to one-off agent actions.

Without an explicit boundary, three different concepts blur together:

1. **Worker/runtime execution**
   - `imp run`
   - native Rust worker path

2. **Ephemeral programmable actions during a run**
   - "run a small script to transform some data"
   - one-off scripting

3. **Durable packaged behavior**
   - extension-defined tools/hooks/commands
   - reusable long-lived functionality

They should not be the same thing.

---

## Current State from Code Inspection

### `extend` is not the script tool

`crates/imp-core/src/tools/extend.rs` is currently scoped to:
- Lua reference lookup
- skill reference lookup
- skill file create/patch/delete

This makes it an authoring and management helper.
It does **not**:
- execute one-off scripts during a run
- present a transient programmable surface to the agent
- define runtime policy for sandboxed scripting

So improving `extend` is not the same as designing a script tool.

### Lua host surface is broader than what a script tool should expose by default

`crates/imp-lua/src/bridge.rs` exposes host APIs such as:
- `imp.register_tool`
- `imp.register_command`
- `imp.exec`
- `imp.tool`
- `imp.http.get/post`
- `imp.secret`
- `imp.env`

That is appropriate for a durable extension substrate where the host can reason
about packaged code and lifecycle. It is too broad to use as the default mental
model for a one-off script tool.

### The worker/runtime boundary is already settled elsewhere

From the user and root planning:
- **worker execution remains a native Rust path**
- `imp run` is not part of the script or Lua surface

This proposal must preserve that boundary.

---

## Design Goals

1. **Create a clear agent-facing surface for one-off scripting**
   - useful during a run
   - explicit in the tool list
   - easy to reason about

2. **Keep durable extensions separate**
   - extensions register ordinary tools/hooks/commands
   - the agent usually sees those registered tools, not the extension runtime

3. **Keep worker execution separate**
   - `imp run` stays native Rust
   - script execution is not a worker/runtime routing mechanism

4. **Keep the script tool host-owned and policy-first**
   - Rust chooses runtime, limits, and capabilities
   - guest code does not bypass host authority

5. **Avoid locking in a JS engine now**
   - engine choice is intentionally deferred
   - policy and contract come first

---

## Non-Goals

This proposal does **not**:
- implement the script tool
- choose a specific JS/TS engine now
- redefine the durable extension substrate
- move worker execution into a guest runtime
- grant raw filesystem/network/process access by default
- merge script execution into the `extend` tool

---

## Core Product Boundary

### 1. Script tool

The script tool is for **ephemeral, run-scoped programming**.

Use it when the agent needs to do something like:
- transform structured text or JSON
- compute a small derived result
- parse or reshape tool output
- perform a bounded local algorithm better expressed as code than prose
- create a temporary helper for the current run only

The key properties are:
- explicit tool call
- ephemeral lifetime
- no durable registration by default
- policy-controlled runtime

### 2. Durable extensions

Durable extensions are packaged units that can:
- register tools
- register hooks
- register commands
- persist beyond the current run
- be reloaded/discovered later

The agent typically should not need to think in terms of the extension runtime.
It should mostly see the resulting tools or commands.

### 3. `extend`

`extend` should remain an authoring/management helper.

It may eventually help author or package durable extensions, but it should not
become the same thing as the script tool.

### 4. Worker/runtime execution

`imp run` / worker execution remains a native Rust runtime path.

The script tool must not be used to model, replace, or route that path.

---

## Agent-Facing UX

The agent should see the script tool as an explicit action-oriented surface.

Conceptually, the tool should answer a request like:

```text
Run this short script in a bounded sandbox and return structured output.
```

Not:

```text
Create a durable extension.
```

and not:

```text
Run worker logic.
```

That means the script tool should be presented as a normal runtime tool,
with obvious parameters like:
- script source
- runtime selector (future-facing, host-controlled)
- input payload
- timeout/resource options (host-bounded)

The exact schema can evolve, but the product shape should remain explicit.

---

## Proposed Contract Shape

The final wire format can vary, but conceptually the script tool should look like:

```text
script(
  runtime,
  source,
  input,
  timeout,
  memory_limit,
  output_mode
)
```

### Minimum contract concepts

1. **Runtime selector**
   - identifies the guest runtime family
   - host may ignore or constrain it initially
   - useful for future extension without redesigning the tool

2. **Script source**
   - inline source for a one-off script

3. **Input payload**
   - structured input from the agent or prior tools

4. **Bounded execution policy**
   - timeout
   - memory/runtime ceiling
   - output size limits

5. **Structured result**
   - success output
   - stderr/diagnostics when allowed
   - normalized host error messages on failure

---

## Capability Model

The script tool should be more restrictive by default than durable extensions.

### Default posture

Default should be **deny by default, grant narrowly**.

That means the initial script tool should assume:
- no raw filesystem access
- no raw network access
- no arbitrary subprocess access
- no unconstrained access to host/native tools

### Why

A one-off script is a transient agent action. It should not automatically inherit
all the authority of a packaged extension or of the host runtime.

If it did, the script tool would become an unbounded escape hatch.

### Capability families to reason about

1. **Data-only execution**
   - pure transform over provided input
   - safest starting posture

2. **Host-mediated file access**
   - only through explicit Rust-owned APIs
   - not raw host FS calls by default

3. **Host-mediated network access**
   - only if the host deliberately grants it

4. **Host-mediated subprocess access**
   - likely disallowed initially

5. **Host-mediated native tool access**
   - carefully controlled if allowed at all

---

## Native Tool Access Policy

This is the most important policy choice in the near term.

### Recommendation

The script tool should **not** get unrestricted direct access to native imp tools.

Instead, it should use **safe host-exposed APIs** chosen for the script-tool contract.

Why:
- it keeps the tool understandable
- it prevents the script tool from becoming a generic policy bypass
- it preserves Rust-side mediation of authority
- it keeps the distinction between script execution and ordinary tool use clear

### Possible future posture

If native tool access is ever allowed from scripts, it should be:
- explicit
- host-mediated
- allowlisted
- visible in policy/config

Not automatic.

---

## Runtime Choice

This proposal is intentionally **runtime-agnostic**.

That means:
- do not commit to a specific JS engine now
- do not say TS/JS is already shipped
- do not treat Lua as the automatic answer just because it exists today

The right sequence is:
1. define the script-tool policy and contract
2. choose a runtime that fits that contract later

That runtime could be:
- a small JS/TS runtime in the future
- a different embedded guest runtime
- or something else host-chosen

The key is that the **tool contract and host policy come first**.

---

## Relationship to the Guest-Runtime Substrate

The new `44.1` proposal defines the durable extension substrate like this:
- Rust host owns policy and authority
- guest runtimes host extension code
- Lua is the first shipped guest runtime
- worker execution remains Rust-native

The script tool should align with that architecture, but it is **not the same layer**.

### Shared principles

Both script tool and durable extensions should:
- be host-owned and policy-enforced by Rust
- avoid bypassing host authority
- keep runtime choice below the policy boundary

### Different product roles

- **script tool** = ephemeral run-scoped programmable action
- **extension substrate** = durable package system for reusable behavior

So the script tool may eventually run on top of a guest runtime implementation,
but product-wise it remains a distinct surface.

---

## What Not to Do

1. **Do not merge script execution and extension authoring**
   - `extend` is not the script tool

2. **Do not let the script tool imply worker/runtime execution**
   - `imp run` remains Rust-native

3. **Do not grant raw filesystem/network/process access by default**
   - keep the initial tool policy-first and narrow

4. **Do not let the script tool bypass Rust-side policy enforcement**
   - all authority should still flow through host mediation

5. **Do not choose a JS engine just to make the design feel concrete**
   - engine choice is a later implementation decision

---

## Suggested Implementation Phases

### Phase 0 — document the boundary

Goal:
- land this proposal and make the product/runtime split explicit

Outcome:
- script tool and durable extension substrate are no longer conflated

### Phase 1 — define the host-side script contract

Goal:
- add a Rust-owned internal contract for ephemeral script execution

Suggested work:
- define a script request/result type in Rust
- define timeout/output/resource limits
- define the initial capability model (very narrow by default)
- define normalized host-side errors

### Phase 2 — add the first bounded runtime implementation

Goal:
- implement the script tool with one bounded runtime

Requirements:
- no raw authority by default
- structured input/output
- explicit sandbox limits
- no worker/runtime boundary leakage

### Phase 3 — evaluate selective capability expansion

Goal:
- only after the bounded tool exists, consider narrow opt-in capability additions

Examples:
- data-only script remains default
- optional allowlisted host APIs for selected file/network/tool calls

### Phase 4 — align with durable extension substrate where useful

Goal:
- share guest-runtime machinery where it helps implementation
- keep product semantics separate

This is implementation sharing, not surface unification.

---

## File-Level Follow-On Guidance

Grounded in the currently inspected files:

### `crates/imp-core/src/tools/extend.rs`

Implication:
- keep `extend` positioned as authoring/management
- do not overload it with ephemeral script execution semantics

### `crates/imp-lua/src/bridge.rs`

Implication:
- current host APIs show the kinds of capabilities the host can expose
- use this as a cautionary map, not as the default script-tool grant set

### `docs/proposals/guest-runtime-extension-substrate.md`

Implication:
- script tool should align with the host-owned guest-runtime architecture
- but remain a distinct product/runtime surface from durable extensions

---

## Success Criteria

This proposal succeeds if future work produces a script tool with these properties:
- the agent sees it as an explicit one-off scripting tool
- it is distinct from `extend`
- it is distinct from durable extensions
- it is distinct from native Rust worker execution
- it stays host-owned and policy-first
- runtime choice remains a later implementation detail, not the design center
