---
id: '48'
title: Audit exported tool schemas for provider-compatible top-level JSON Schema restrictions
slug: audit-exported-tool-schemas-for-provider-compatibl
status: open
priority: 1
created_at: '2026-04-14T08:56:59.186969Z'
updated_at: '2026-04-14T08:56:59.186969Z'
acceptance: All provider-facing tool schemas emitted by imp are top-level JSON objects without top-level `oneOf`/`anyOf`/`allOf`/`enum`/`not`, and any conditional requirements are enforced by runtime checks or nested schema structure instead. Focused tests cover the flattened schema contract and the runtime-required arguments for affected tools.
labels:
- bugfix
- schema
- tools
- anthropic
- imp-core
verify: cd /Users/asher/imp && rg -n '^\s*"(oneOf|anyOf|allOf|enum|not)"\s*:' crates/imp-core/src/tools && cargo test -p imp-core schema_ -- --nocapture
verify_timeout: 120
kind: job
paths:
- crates/imp-core/src/tools
- crates/imp-core/src/tools/mod.rs
- crates/imp-core/src/tools/lua.rs
- crates/imp-llm/src/providers/anthropic.rs
---

Current state: the `imp` tool in `crates/imp-core/src/tools/imp.rs` was exporting a top-level `allOf`, which Anthropic/tool registration rejected with `schema must have type 'object' and not have 'oneOf'/'anyOf'/'allOf'/'enum'/'not' at the top level`. The immediate fix was applied by removing top-level `allOf`, adding the missing `prompt` property, and moving conditional requirements to runtime checks. Follow-up work should audit the rest of imp's exported tool schemas so other tools do not fail at runtime for the same provider-level restriction.

Steps:
1. Inspect all native tool `parameters()` schemas under `crates/imp-core/src/tools/` and any Lua/extension schema export paths that can reach Anthropic tool definitions.
2. Find any schemas with top-level `oneOf`/`anyOf`/`allOf`/`enum`/`not` or other provider-incompatible top-level constructs.
3. For each hit, decide whether to flatten the exported schema to a plain top-level object and move conditional enforcement into runtime validation, following the `imp` tool fix pattern.
4. Add or update focused tests so exported schemas remain provider-compatible and required runtime checks still fail clearly.
5. Verify with targeted tests and a repo search that no remaining exported tool schemas use the forbidden top-level constructs where provider-facing schemas are emitted.

Files:
- crates/imp-core/src/tools/ (inspect/modify exported native tool schemas)
- crates/imp-core/src/tools/mod.rs (inspect validation helpers/patterns)
- crates/imp-core/src/tools/lua.rs (inspect extension schema handling and provider-facing behavior)
- crates/imp-llm/src/providers/anthropic.rs (confirm provider-facing tool definition expectations)

In scope:
- Provider-facing tool schema compatibility for imp
- Small, reversible schema/test changes

Out of scope:
- Broad redesign of the tool system
- Non-provider local JSON Schema capabilities unless they affect exported tool definitions
