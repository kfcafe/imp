# Guest-Runtime Extension Substrate for imp

> Proposal for `44.1` — April 2026
>
> Defines how imp should evolve from a runtime-specific Lua extension path
> into a language-agnostic guest-runtime extension substrate while keeping
> Rust host ownership, Rust-native worker execution, and the current shipped
> Lua path intact.

---

## Executive Summary

imp already has a working extension path, but it is expressed too directly
as "Lua support" instead of as a more general host-owned extension substrate.

Today:
- `imp-core` exposes a `lua_tool_loader` seam in `AgentBuilder`
- `imp-lua` discovers `.lua` files, boots a `LuaRuntime`, exposes host APIs,
  and registers tool/hook/command handles back into imp
- Lua extensions can call back into native imp tools through `imp.tool()`

That is enough to prove the architecture shape, but not enough to make the
extension system legible as a durable platform boundary.

The proposal is:
- keep **Rust as the host owner** of policy, runtime authority, tool registry,
  and worker execution
- treat **Lua as the first shipped guest runtime**, not as the whole design
- define a **guest-runtime substrate** that could later host other runtimes
  without changing the top-level product/runtime ownership model
- keep **`imp run` and worker execution Rust-native**; this proposal is about
  extensibility, not about routing worker/runtime execution through Lua

In short:

```text
Rust host owns runtime authority
  -> guest runtime substrate hosts extension code
     -> Lua is the first guest runtime
```

Not:

```text
worker execution -> Lua
```

---

## Problem

The current shipped path works, but it has three architectural weaknesses.

### 1. The extension boundary is named too narrowly

Current code makes the seam look like "Lua support" instead of a host-owned
extension substrate:
- `imp-core` has `lua_tool_loader`
- `imp-lua` owns discovery, sandbox/runtime state, host API bridging, and tool
  registration
- the authoring tool (`extend`) is explicitly about skills and Lua reference

That framing makes it harder to distinguish:
- the **Rust host/runtime boundary**
- the **guest runtime execution boundary**
- the **extension package/lifecycle boundary**

### 2. Host policy and guest capability are not yet modeled as first-class extension contracts

The current Lua path already has real capability surfaces:
- native tool access via `imp.tool()`
- shell execution via `imp.exec()`
- HTTP via `imp.http.*`
- secret access via `imp.secret()` / `imp.secret_fields()`
- env access via `imp.env()`
- hook/tool/command registration

But these are exposed as ad hoc host API functions rather than as a clearly
named guest-runtime capability model.

### 3. The current architecture can be misread as "replace Lua later"

That would be the wrong conclusion.

The right move is:
- preserve the working Lua path
- move the architecture language up a level
- make the Rust host contract language-agnostic
- treat future runtimes as optional later guests, not as the reason for the design

---

## Current State from Code Inspection

### `imp-core` host seam

`crates/imp-core/src/builder.rs` shows the current ownership split clearly:
- `AgentBuilder` owns native tool registration and overall agent assembly
- `extra_tools` runs after native tools are registered
- `lua_tool_loader` runs after native and extra tools, before mode filtering
- `agent.lua_tool_loader` is carried into the runtime for later use

This means the host already owns:
- the canonical tool registry
- the ordering of tool registration
- mode filtering and runtime visibility
- the system prompt and runtime policy context

### Current Lua bootstrap

`crates/imp-lua/src/lib.rs` currently:
- discovers extensions from user and project dirs
- creates `LuaRuntime`
- calls `setup_host_api()`
- loads extension files
- gives Lua access to native tools with `rt.set_native_tools(tools.tools_map())`
- registers Lua-defined tools back onto imp's tool registry

That is already a guest-runtime pattern in everything but name.

### Discovery model

`crates/imp-lua/src/loader.rs` discovers:
- direct `.lua` files
- directories containing `init.lua`
from:
- `<user-config>/lua`
- `<project>/.imp/lua`

This is a working discovery mechanism, but it is runtime-specific and does not
express a general extension package manifest or lifecycle.

### Runtime and capability model

`crates/imp-lua/src/sandbox.rs` and `bridge.rs` show that Lua runtime state already tracks:
- registered tools, hooks, commands
- native host tools available to `imp.tool()`
- active execution context for guest-to-host calls
- allowed env vars
- an `allow_native_tool_calls` gate

`setup_host_api()` exposes host functions including:
- `imp.on(...)`
- `imp.register_tool(...)`
- `imp.register_command(...)`
- `imp.exec(...)`
- `imp.tool(...)`
- `imp.secret(...)`
- `imp.secret_fields(...)`
- `imp.env(...)`
- `imp.http.get/post(...)`
- a small inter-extension event bus

This is already a meaningful capability surface. The missing piece is not raw power.
It is architecture naming, packaging, and policy framing.

---

## Design Goals

1. **Preserve Rust host ownership**
   - policy enforcement stays in Rust
   - runtime authority stays in Rust
   - tool registration and visibility stay in Rust
   - worker execution stays in Rust

2. **Preserve Lua as the first shipped guest runtime**
   - do not break the current Lua path
   - do not describe TS/JS as shipped

3. **Promote the seam from "Lua support" to "guest runtime substrate"**
   - the host contract should not be named around one guest language

4. **Separate extension substrate from worker/runtime execution**
   - `imp run` and tool/worker execution remain native Rust
   - guest runtimes are for extension code, not for the worker runtime path itself

5. **Make capabilities explicit**
   - host APIs should be described as capabilities under Rust host control
   - future runtimes should fit the same capability model

6. **Make lifecycle and packaging legible**
   - extensions should become packaged/durable units with metadata
   - runtime discovery should evolve toward manifest-driven loading

---

## Non-Goals

This proposal does **not**:
- replace Lua with TypeScript
- choose a JS engine now
- move worker execution into guest runtimes
- define a marketplace or installation UX
- redesign imp's entire tool system
- require immediate code extraction into many crates

---

## Core Architecture

### 1. Rust host remains the authority boundary

Rust owns:
- agent/runtime execution
- worker execution (`imp run`, native tools, runtime loop)
- policy enforcement
- tool registry and filtering
- host-side capability mediation
- extension loading policy
- packaging/lifecycle decisions

This is the critical constraint.

The guest runtime is **not** the authority boundary.
It is a sandboxed extension execution environment behind the Rust host.

### 2. Guest runtime is the execution environment for extension code

A guest runtime is a host-managed execution environment that can:
- load extension code
- register tools/hooks/commands through host-owned APIs
- execute extension callbacks under a host-supplied call context
- access only the capabilities explicitly exposed by the Rust host

Today, Lua is the first such runtime.

In future, another runtime could exist, but only if it fits the same host-owned model.

### 3. Extension is the packaged durable unit

An extension should be thought of as a durable package that can contribute:
- tools
- hooks
- commands
- metadata
- configuration defaults or declared capability needs

Today, the package boundary is effectively:
- a `.lua` file, or
- a directory with `init.lua`

That is good enough for the current shipped path, but the long-term substrate
should evolve toward an explicit manifest-based package contract.

---

## Proposed Vocabulary

Use these terms in architecture and follow-on implementation work:

- **extension** — the packaged durable unit
- **guest runtime** — the language/runtime execution environment for extension code
- **host capability** — a Rust-owned operation exposed to guest code
- **extension manifest** — extension metadata and declared capability needs
- **host policy** — Rust-side enforcement of what an extension may do

Avoid using **Lua** as the name for the whole substrate.
Lua is one guest runtime.

---

## Proposed Host / Guest Split

### Host-owned responsibilities (Rust)

The host should own:
- extension discovery entrypoints
- manifest parsing and validation
- capability grants and denials
- tool registration into the canonical registry
- runtime mode filtering (`full`, `worker`, `orchestrator`, etc.)
- context injection for guest callbacks
- logging/telemetry/error normalization
- cancellation handling
- worker/runtime execution

### Guest-owned responsibilities

Guest code may:
- define tools/hooks/commands
- implement callback logic
- transform data inside its granted capability set
- call back into host capabilities that are explicitly allowed

Guest code should not own:
- authority over the host runtime
- direct unsandboxed filesystem/network/process access by default
- scheduling or worker orchestration policy
- tool visibility policy

---

## Capability Model

The current Lua host APIs map naturally into capability families.

### Capability families

1. **Registration capabilities**
   - register tool
   - register hook
   - register command

2. **Runtime-call capabilities**
   - call host/native tools
   - emit progress updates
   - participate in event bus patterns

3. **Execution capabilities**
   - shell/process execution (`imp.exec`)

4. **Network capabilities**
   - HTTP GET/POST

5. **Secret/config capabilities**
   - read saved secrets
   - read selected environment variables

### Policy direction

These capabilities should be treated as Rust-side grants, not as implicit language features.

That means the future substrate should allow the host to answer questions like:
- may this extension call native tools at all?
- may it call only a subset of tools?
- may it read env vars, and which ones?
- may it use HTTP?
- may it use process execution?
- may it register hooks in all modes, or only some?

The current Lua runtime already hints at this with:
- `allowed_env`
- `allow_native_tool_calls`

The next step is to raise this into an explicit extension capability model rather than leaving it as runtime-local implementation detail.

---

## Extension Manifest Direction

The substrate should move toward an explicit manifest, even if Lua remains the only shipped runtime for now.

A future manifest should describe things like:
- extension name / version
- runtime kind (`lua` for phase 1)
- entrypoint
- declared tools / hooks / commands (optional summary)
- requested capabilities
- optional config schema

For example, conceptually:

```text
extension:
  name: example-extension
  runtime: lua
  entrypoint: init.lua
  capabilities:
    native_tools: limited
    env: ["TAVILY_API_KEY"]
    http: true
    exec: false
```

This proposal does not require implementing the manifest now.
It defines the target substrate language so future work is coherent.

---

## Discovery and Loading Direction

### Today

Discovery is runtime-specific:
- `<user>/lua/*.lua`
- `<user>/lua/<dir>/init.lua`
- `<project>/.imp/lua/*.lua`
- `<project>/.imp/lua/<dir>/init.lua`

### Direction

Keep the current Lua discovery path working, but evolve loading toward:
- extension-package discovery
- manifest validation
- runtime-specific loader dispatch behind a host-neutral entrypoint

Conceptually:

```text
host discovers extension package
  -> reads manifest
  -> chooses guest runtime loader
  -> initializes runtime with granted capabilities
  -> loads extension code
  -> registers exposed tools/hooks/commands into host registry
```

This keeps loading language-neutral at the host boundary.

---

## Worker Execution Boundary

This needs to be explicit because the user has already clarified it.

### Settled constraint

**`imp` tool/worker execution remains a native Rust path.**

That means:
- `imp run` is not an extension runtime
- headless worker execution is not a Lua concern
- runtime bugs in worker execution should be fixed in Rust worker/runtime code, not by widening Lua's role

### Practical implication for this proposal

The guest-runtime substrate must clearly stop at extension code.
It must not become the substrate for:
- worker execution
- direct task runtime routing
- orchestration or handoff semantics

Those stay in the Rust runtime / mana worker boundary.

---

## What Not to Do Yet

1. **Do not frame this as "replace Lua with TS"**
   - that skips the actual architectural move
   - the move is host-owned guest-runtime design

2. **Do not move worker execution behind guest runtimes**
   - this conflicts with the settled native Rust worker boundary

3. **Do not let guest runtimes bypass host policy**
   - raw access should not outrun Rust-side capability enforcement

4. **Do not merge script execution and durable extension packaging into one concept**
   - one-off script execution is a separate tool/policy problem
   - durable extensions are packaged substrate units

5. **Do not overfit the manifest to Lua-only assumptions**
   - phase 1 can be Lua-only in practice
   - the substrate language should remain guest-runtime oriented

---

## Migration Framing

### Phase 0 — name the actual architecture

Goal:
- document that the current Lua path is the first guest runtime under a Rust host-owned extension substrate

Concrete outcome:
- this proposal lands
- future docs stop treating "Lua support" as the whole extension architecture

### Phase 1 — preserve current Lua path, clarify interfaces

Goal:
- keep current behavior while making the host/guest split explicit in code and docs

Suggested follow-on work:
- rename or wrap Lua-specific entry seams so host-neutral extension concepts are visible
- document capability families and current grants
- isolate runtime-specific loading behind a host-neutral extension loading entrypoint

Possible concrete slices:
1. introduce host-neutral naming around extension loading in `imp-core`
2. add a small extension metadata struct owned by Rust
3. document current capability surfaces in code and docs
4. make the current Lua loader consume host-owned metadata rather than being the entire concept

### Phase 2 — introduce manifest-backed extension packages

Goal:
- make durable extensions explicit packages rather than only discovered source files

Suggested work:
- add manifest schema owned by Rust
- support manifest validation and capability declaration
- keep backward compatibility with existing Lua file discovery during migration

### Phase 3 — formalize capability enforcement

Goal:
- turn current ad hoc host API exposure into explicit host policy enforcement

Suggested work:
- define capability grant model in Rust
- make `imp.tool`, env access, HTTP, secrets, and exec individually controllable
- carry capability decisions into runtime diagnostics and extension errors

### Phase 4 — optionally add another guest runtime

Goal:
- only after the host contract is stable, consider a second guest runtime

Requirements before doing this:
- manifest exists
- host capability model exists
- loading entrypoint is host-neutral
- Lua remains supported during transition

This is where a TS/JS runtime could be evaluated, but it is intentionally not part of phase 1.

---

## Implementation Seams to Target Later

Grounded in the current inspected files:

### `crates/imp-core/src/builder.rs`

Current role:
- host-owned agent assembly
- native tool registration
- runtime mode filtering
- Lua loader injection point

Follow-on direction:
- evolve `lua_tool_loader` toward a guest-runtime-neutral extension loader seam
- keep worker/runtime assembly separate from extension loading

### `crates/imp-core/src/tools/extend.rs`

Current role:
- skills + Lua reference authoring helper

Follow-on direction:
- do not confuse this with the extension substrate itself
- in future, treat it as authoring/management UX, not as the architecture boundary

### `crates/imp-lua/src/lib.rs`

Current role:
- runtime bootstrap for the Lua guest runtime

Follow-on direction:
- keep this as the Lua-specific adapter implementing the guest-runtime substrate

### `crates/imp-lua/src/loader.rs`

Current role:
- Lua-specific discovery and loading

Follow-on direction:
- eventually sit behind a host-owned manifest/discovery model

### `crates/imp-lua/src/sandbox.rs`

Current role:
- runtime state, capability-ish gates, native tool bridge context

Follow-on direction:
- treat this as the concrete phase-1 implementation of a guest runtime state container
- lift capability concepts into host-owned abstractions over time

### `crates/imp-lua/src/bridge.rs`

Current role:
- host API surface

Follow-on direction:
- reorganize conceptually around capability families
- keep Rust-side validation and gating authoritative

---

## Success Criteria

This proposal is successful if future work treats the system as:

```text
Rust host runtime
  + host-owned capability model
  + extension package lifecycle
  + guest runtime adapters
```

with these constraints remaining true:
- Lua remains the current shipped guest runtime
- Rust remains the worker/runtime execution owner
- the extension substrate is bigger than a Lua convenience layer
- future runtimes are optional adapters to the same host contract, not a reason to replace the current system wholesale
