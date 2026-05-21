# Guest Runtime Implementation Plan

This plan translates the guest-runtime substrate and script-tool policy proposals into implementation phases. It is sequencing guidance, not an implementation patch.

## Guardrails

- Rust host authority stays central: policy, tool registry, capability checks, and worker execution remain native Rust.
- Lua remains the first shipped guest runtime and must keep working during migration.
- Durable extensions and the ephemeral script tool are separate surfaces.
- Do not implement the script tool before host/runtime policy and capability contracts exist.
- Do not treat JavaScript or TypeScript as Phase 1; they are optional future guests after the substrate is proven.

## Phase 1 — Host/runtime abstraction in `imp-core`

Goal: rename and isolate the existing Lua-shaped extension seam into a language-neutral guest-runtime host contract without changing behavior.

Likely files:
- `crates/imp-core/src/builder.rs`
- `crates/imp-core/src/tools/extend.rs`
- `crates/imp-lua/src/lib.rs`
- `crates/imp-lua/src/loader.rs`
- `crates/imp-lua/src/sandbox.rs`

Expected artifacts:
- a host-owned trait or adapter boundary for guest runtimes that can register tools/hooks/commands;
- a compatibility adapter that wraps the current Lua loader;
- no change to native worker execution;
- existing Lua extension discovery still works.

Verify strategy:
- `cargo check -p imp-core -p imp-lua`
- existing Lua extension/tool registration tests, plus one compatibility test proving Lua tools still appear through the new host abstraction.

## Phase 2 — Manifest and permission model

Goal: make extension packages and capabilities explicit before adding more runtime power.

Likely files:
- `crates/imp-lua/src/loader.rs`
- `crates/imp-lua/src/sandbox.rs`
- `crates/imp-lua/src/bridge.rs`
- new shared extension policy module in `imp-core` or a future small crate if needed

Expected artifacts:
- manifest schema for durable extensions;
- capability names for host APIs such as native tool calls, shell, HTTP, env, and secrets;
- loader output that reports extension identity, runtime, requested capabilities, and granted capabilities;
- deny-by-default behavior for new capabilities, preserving existing Lua compatibility behind an explicit legacy/default profile if needed.

Verify strategy:
- schema/manifest unit tests;
- loader tests for direct Lua files and manifest-backed packages;
- bridge tests proving denied capabilities fail through Rust-side policy, not guest-side convention.

## Phase 3 — Lua migration and adaptation

Goal: move current Lua behavior onto the substrate without breaking current users.

Likely files:
- `crates/imp-lua/src/lib.rs`
- `crates/imp-lua/src/loader.rs`
- `crates/imp-lua/src/bridge.rs`
- `crates/imp-core/src/builder.rs`
- docs for extension authors

Expected artifacts:
- Lua runtime implements the guest-runtime adapter contract;
- current `<user-config>/lua` and `<project>/.imp/lua` discovery remains supported;
- manifest-based Lua packages are accepted alongside legacy files/directories;
- compatibility docs explain how existing Lua users migrate to manifests over time.

Verify strategy:
- compatibility fixtures for legacy `.lua` files and `init.lua` directories;
- manifest-backed Lua fixture tests;
- `cargo check -p imp-core -p imp-lua` and focused extension registration tests.

## Phase 4 — Evolve `extend` into extension authoring/management

Goal: keep `extend` as an authoring and management helper, not a script execution tool.

Likely files:
- `crates/imp-core/src/tools/extend.rs`
- extension author docs under `docs/` or skill resources

Expected artifacts:
- `extend` can explain/package/inspect durable extensions using the manifest vocabulary;
- existing skill-authoring behavior is preserved unless deliberately split later;
- generated extension templates default to least privilege and Lua-first examples.

Verify strategy:
- `extend` tool unit tests for reference output and generated manifest templates;
- no script execution behavior in `extend` tests or schema.

## Phase 5 — Optional script tool spike after policy approval

Goal: only after host policy is explicit, prototype the ephemeral script tool as a distinct agent-facing tool.

Likely files:
- `crates/imp-core/src/tools/script.rs` or equivalent
- shared policy/capability module from Phase 2
- docs/proposals follow-up if runtime choice changes assumptions

Expected artifacts:
- script tool contract with runtime selector, timeout/output limits, resource policy, and structured result envelope;
- no durable tool/hook/command registration;
- no raw filesystem/network/process/native-tool access by default;
- clear errors when requested capabilities exceed policy.

Verify strategy:
- unit tests for bounded execution, timeout, output truncation, denied capabilities, and structured errors;
- security review before enabling beyond local experimental profiles.

## Phase 6 — Optional future guest runtimes

Goal: consider JS/TS or other guests only after Lua proves the substrate.

Expected artifacts:
- runtime adapter implementation behind the same host contract;
- no changes to worker execution semantics;
- compatibility tests showing host policy applies identically across runtimes.

Verify strategy:
- adapter-specific tests plus shared conformance tests from the substrate.

## Sequencing Decisions

1. Start with the host/runtime abstraction because it reduces naming and ownership confusion without adding runtime power.
2. Add manifest and permission contracts before new guest runtimes or script execution.
3. Migrate Lua before adding JavaScript/TypeScript so current shipped users are not stranded.
4. Keep `extend` focused on durable extension authoring/management.
5. Treat the script tool as an optional later spike, gated by policy approval and separate from durable extensions.
