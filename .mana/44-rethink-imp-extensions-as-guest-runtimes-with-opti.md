---
id: '44'
title: Rethink imp extensions as guest runtimes with optional sandbox script execution
slug: rethink-imp-extensions-as-guest-runtimes-with-opti
status: open
priority: 1
created_at: '2026-04-07T16:18:29.762507Z'
updated_at: '2026-04-07T16:19:33.173189Z'
acceptance: 'There is a coherent design captured in mana for: (1) guest-runtime extension substrate, (2) script tool boundaries and policy, and (3) a phased implementation path for imp.'
notes: |-
  ---
  2026-04-07T16:19:33.173183+00:00
  Loaded and followed the `mana` skill before authoring units. Created a parent epic and decomposed it into three concrete child jobs: 44.1 substrate proposal, 44.2 script-tool boundaries/policy, and 44.3 phased implementation plan depending on the first two. Each unit includes inspected file context, scope boundaries, and targeted verify gates so later workers can execute cold without redoing the design framing.
labels:
- architecture
- imp
- extensions
- guest-runtime
- lua
- script-tool
verify: test -n "guest-runtime-design-tracked-in-mana"
kind: epic
---

Goal: define a new extension architecture for imp where Rust remains the host/source of truth, guest runtimes (starting with Lua, later possibly JS/TS) plug into a common capability substrate, and ephemeral sandbox scripting is treated as a separate product/runtime concern from durable extensions.

Current state inspected in this session:
- `crates/imp-core/src/builder.rs` wires a `lua_tool_loader`, so the builder is currently runtime-specific rather than guest-runtime-generic.
- `crates/imp-lua/src/{lib.rs,loader.rs,sandbox.rs,bridge.rs}` provides the existing extension runtime, host API bridge, and runtime-managed tool registration.
- `crates/imp-core/src/tools/extend.rs` is currently an authoring helper for skills + Lua reference, not a general extension-system manager.
- Existing proposal docs in `docs/proposals/` use markdown design-note format and are a good precedent.

Desired outcome:
- Capture a clear architecture for guest runtimes and extension manifests.
- Decide how a future script tool relates to durable extensions.
- Leave a concrete, phased implementation plan that can be decomposed into coding jobs later.

Scope:
- imp-only, project-local `.mana/`
- design/proposal work first; no implementation required yet
- preserve current Lua support while opening a path to JS/TS later

Out of scope:
- implementing a JS runtime now
- replacing Lua immediately
- shipping a script tool before policy/UX is defined
