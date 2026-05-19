# TypeScript Extension Bridge

## Goal

Add TypeScript extension tools without moving authority out of Rust. Extensions
are discovered from manifests, registered as normal imp tools, executed
out-of-process, policy-checked by the same `ReferenceMonitor` path as native
and Lua tools, and surfaced through existing trace/evidence/TUI/runtime state.

Non-goal: no in-process JavaScript or TypeScript runtime embedded in imp. Rust
owns discovery, validation, policy, cancellation, timeouts, secrets mediation,
output limits, tracing, and evidence references.

## Current codebase baseline

Existing support is intentionally compatibility-oriented:

- `crates/imp-core/src/typescript_extensions/discovery.rs`
  - discovers `.ts` files and package directories under `.imp/extensions`
  - supports `package.json` `pi.extensions` compatibility
- `schema.rs`
  - currently models registered tools returned by the Bun compatibility bridge
  - normalizes parameters into JSON Schema-ish objects
- `bun_runner.rs`
  - writes a temporary bridge script and runs `bun`
  - supports `inspect` and `execute` actions
- `mod.rs`
  - registers discovered TS tools into `ToolRegistry`
  - wraps tools as `TypeScriptExtensionTool`

This is useful compatibility scaffolding but is not yet the target security
model. The new bridge should keep backwards-compatible Pi/Bun discovery where
possible while introducing a manifest-first authority boundary.

## Relationship to Lua and guest-runtime substrate

Lua extensions already demonstrate an in-process sandbox/bridge pattern:

- Lua tool metadata is normalized into imp tool schemas.
- Lua tool results are converted into native `ToolOutput`.
- Rust remains the registry/execution authority.

TypeScript should share the same principles but use a stricter process boundary:

- TS tools run out-of-process.
- TS metadata comes from a Rust-validated manifest.
- TS execution uses JSON messages over stdio or one-shot subprocess calls.
- TS tools never receive ambient secrets or broad environment access by default.

The guest-runtime substrate proposal remains the long-term common model:
manifest → capability declaration → policy check → isolated execution → typed
output → trace/evidence/runtime event.

## Authoring quickstart

Create a project-local extension package under `.imp/extensions/<name>`:

```text
.imp/extensions/my-tools/
  imp.extension.json
  package.json
  src/tool.mjs
```

Start with a read-only tool. Read-only tools are easiest to approve and should be
the default unless the tool truly needs to modify workspace files or external
state.

Minimal `imp.extension.json`:

```json
{
  "schemaVersion": 1,
  "id": "com.example.my-tools",
  "name": "My tools",
  "version": "0.1.0",
  "runtime": {
    "kind": "typescript-subprocess",
    "command": "node",
    "args": ["src/tool.mjs"],
    "protocol": "one-shot-json",
    "timeoutMs": 5000,
    "outputLimitBytes": 65536
  },
  "tools": [
    {
      "name": "my_echo",
      "label": "My echo",
      "description": "Echo text through a TypeScript extension.",
      "inputSchema": {
        "type": "object",
        "properties": { "text": { "type": "string" } },
        "required": ["text"],
        "additionalProperties": false
      },
      "sideEffect": "read-only",
      "resourceScope": { "kind": "none" },
      "network": "none",
      "secrets": [],
      "env": [],
      "policyTags": ["extension", "example"],
      "verifierTags": []
    }
  ]
}
```

Minimal one-shot handler:

```js
#!/usr/bin/env node
import { readFileSync } from "node:fs";

const request = JSON.parse(readFileSync(0, "utf8"));
const text = String(request.input?.text ?? "");

console.log(JSON.stringify({
  id: request.id,
  type: "tool_result",
  content: [{ type: "text", text }],
  details: { extension: request.context?.extensionId }
}));
```

Test it directly:

```sh
printf '%s\n' '{"id":"call-1","type":"tool_call","tool":"my_echo","input":{"text":"hello"}}' \
  | node .imp/extensions/my-tools/src/tool.mjs
```

## Discovery and install locations

Initial manifest discovery should search project-local extension roots only:

```text
.imp/extensions/*/imp.extension.json
.imp/extensions/*.extension.json
```

A package directory may also include `package.json`, `tsconfig.json`, source, and
lockfiles, but `imp.extension.json` is the authoritative imp manifest.

Pi compatibility may continue to discover `package.json` `pi.extensions`, but
those tools should be treated as legacy compatibility tools until they are
wrapped in a manifest.

Discovery rules:

1. ignore missing extension directories
2. discover manifests deterministically and sort by extension id/name
3. reject duplicate tool names unless namespaced explicitly
4. reject manifests outside project-local trusted roots unless installed by a
   future explicit extension install command
5. surface invalid manifests as extension status records, not panics

## Manifest schema

Proposed manifest file: `imp.extension.json`.

```json
{
  "schemaVersion": 1,
  "id": "example.echo",
  "name": "Example Echo",
  "version": "0.1.0",
  "runtime": {
    "kind": "typescript-subprocess",
    "command": "bun",
    "args": ["run", "src/tool.ts"],
    "protocol": "json-lines"
  },
  "tools": [
    {
      "name": "example_echo",
      "label": "Echo",
      "description": "Echo text for extension bridge tests.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "text": { "type": "string" }
        },
        "required": ["text"],
        "additionalProperties": false
      },
      "sideEffect": "read-only",
      "resourceScope": { "kind": "none" },
      "network": "none",
      "secrets": [],
      "timeoutMs": 5000,
      "outputLimitBytes": 65536,
      "policyTags": ["extension", "example"],
      "verifierTags": []
    }
  ]
}
```

### Manifest fields

Extension-level:

- `schemaVersion`: integer, starts at `1`
- `id`: stable reverse-DNS-ish id, unique in project
- `name`: human label
- `version`: semver-ish string
- `runtime.kind`: initially `typescript-subprocess`
- `runtime.command`: executable, usually `bun`, `node`, or package script wrapper
- `runtime.args`: fixed arguments; Rust appends protocol payload or opens stdio
- `runtime.protocol`: `json-lines` initially; `one-shot-json` allowed for simpler
  implementation compatibility

Tool-level:

- `name`: imp tool name; must pass normal tool-name validation
- `label` / `description`: UI and model-facing text
- `inputSchema`: JSON Schema object validated by Rust
- `sideEffect`: `read-only`, `workspace-write`, `external-write`, `destructive`
- `resourceScope`: `none`, `workspace`, `path`, `command`, `network`, or
  structured future scopes aligned with `ReferenceMonitor`
- `network`: `none`, `declared-hosts`, or `unrestricted` (discouraged)
- `networkHosts`: optional allowlist when `network=declared-hosts`
- `secrets`: list of secret env names/capabilities requested
- `timeoutMs`: execution timeout
- `outputLimitBytes`: stdout/stderr/result output cap
- `policyTags`: labels included in policy trace records
- `verifierTags`: labels useful for evidence/verifier routing

## Execution protocols

### `json-lines`

Long-lived subprocess, one JSON message per line:

Rust passes a JSON request on stdin:

```json
{
  "id": "call-123",
  "type": "tool_call",
  "tool": "example_echo",
  "input": { "text": "hello" },
  "context": {
    "cwd": "/repo",
    "runId": "run-1",
    "timeoutMs": 5000
  }
}
```

The extension must write exactly one JSON response to stdout. Logs should go to
stderr so stdout remains parseable:

```json
{
  "id": "call-123",
  "type": "tool_result",
  "content": [{ "type": "text", "text": "hello" }],
  "details": { "extension": "example.echo" },
  "is_error": false
}
```

Error response:

```json
{
  "id": "call-123",
  "type": "error",
  "message": "bad input",
  "code": "invalid_input"
}
```

Supported content blocks are currently text blocks:

```json
[{ "type": "text", "text": "hello" }]
```

Unknown content block types are rejected by Rust.

### `one-shot-json`

Initial easier implementation option: spawn the command per call, pass request
JSON on stdin or as an argument, parse one JSON response from stdout, then exit.
This is slower but simpler and safer for v1. The manifest should allow both so
we can start boring and later add a server pool.

## Declaring capabilities

Manifest declarations are intentionally explicit:

- `sideEffect: "read-only"` for pure inspection/transformation tools.
- `sideEffect: "workspace-write"` only when writing files under declared globs.
- `sideEffect: "external-write"` for APIs or state outside the workspace.
- `sideEffect: "destructive"` for irreversible or high-risk operations.

`resourceScope` must match the declared side effect. For example,
`workspace-write` requires writable workspace globs:

```json
{
  "sideEffect": "workspace-write",
  "resourceScope": {
    "kind": "workspace",
    "read": [],
    "write": ["tmp/my-extension/**"]
  }
}
```

Network and secrets are separate declarations:

```json
{
  "network": { "kind": "declared-hosts", "hosts": ["api.example.com"] },
  "secrets": ["EXAMPLE_API_KEY"],
  "env": ["EXAMPLE_REGION"]
}
```

Current behavior:

- manifest-declared env keys are mediated; only declared keys present in the
  host environment are passed to the subprocess
- ambient process environment is cleared before subprocess launch
- `PATH`, `IMP_EXTENSION_ID`, `IMP_EXTENSION_TOOL`, and `IMP_EXTENSION_ROOT` are
  host-provided
- declared secrets are not injected yet; ReferenceMonitor denies secret-capable
  extension tools until explicit grants are implemented
- network declarations feed ReferenceMonitor policy and TUI warnings; strong OS
  network sandboxing is future work

## Debugging authoring errors

Common failures:

- `invalid TypeScript extension manifest`: schema validation failed; check
  required fields, duplicate tool names, object `inputSchema`, side-effect scope,
  and timeout/output limits
- `returned invalid JSON`: the subprocess wrote logs or non-JSON text to stdout;
  write logs to stderr instead
- `unsupported content block type`: only `{ "type": "text" }` blocks are
  accepted today
- `timed out after ...ms`: increase manifest `timeoutMs` or make the tool return
  faster
- `exceeded output limit`: reduce stdout/result size or raise
  `outputLimitBytes`
- `extension_network_denied`: a network-capable extension ran under an autonomy
  mode that does not allow extension network access
- `extension_secret_denied`: secret-capable extensions are blocked until explicit
  secret grants land

## Capability and security model

Rust treats manifest declarations as claims, not grants. Every tool call still
flows through policy:

1. Load and validate manifest.
2. Register tool metadata and declared capability requirements.
3. On call, validate input against `inputSchema`.
4. Convert manifest declarations + call context into a `ReferenceMonitor` policy
   request.
5. If denied, return normal policy denial without spawning TS.
6. If allowed, spawn subprocess with a constrained env/cwd and timeout.
7. Enforce output size and response schema.
8. Emit trace/runtime/evidence events as with native tools.

Secrets are never passed by default. A tool must declare secret names/capability
labels in `secrets`, and Rust must explicitly decide whether to expose each as an
environment variable or structured stdin field. Secret values must not be logged
or surfaced in evidence packets.

Network access is declared but not fully sandboxed by Node/Bun itself. For v1,
network declarations feed policy and user-facing warnings. Strong network
sandboxing is future work unless executed under an OS/container sandbox.

## Rust-owned lifecycle

1. **Discover** manifests from trusted local roots.
2. **Validate** schema, tool names, JSON schemas, paths, command/args, timeouts,
   and duplicate names.
3. **Register** each tool with `ToolRegistry` using normal `Tool` trait objects.
4. **Policy-check** each call through `ReferenceMonitor` before execution.
5. **Execute** via one-shot or json-lines subprocess protocol.
6. **Decode** result into native `ToolOutput`.
7. **Trace** tool start/end, policy decision, runtime event/state updates, and
   evidence refs where applicable.
8. **Surface** extension status in TUI/CLI with manifest load errors and
   compatibility warnings.

## Compatibility with current TypeScriptExtensionToolStatus

`TypeScriptExtensionToolStatus` should grow from compatibility-only state into a
manifest-aware status surface:

- extension id/name/version
- source path
- load state (`executable`, `needs dependencies`, `invalid manifest`,
  `policy-disabled`, `legacy compatibility`)
- tool names and declared side effects
- compatibility debt from legacy Pi APIs where applicable
- validation or dependency error messages

Existing `TypeScriptExtensionCompatibility` can remain for Pi/Bun compatibility
but should not be the security authority.

## Evidence, tracing, and runtime state

Extension tools flow through the same tool execution path as native tools:

- `AgentEvent::PolicyChecked` records ReferenceMonitor decisions
- TUI surfaces trust warnings and extension manifest policy warnings
- runtime events/snapshots track tool start/output/completion like native tools
- evidence packets can link artifacts produced by extension tools through the
  normal evidence path

Tool results have policy provenance attached in `details.policy`, so reviews can
trace a tool result back to the manifest-derived policy decision that allowed,
denied, prompted, or constrained it.

## TUI and runtime surfacing

TS extension tools should look like normal tools in the TUI:

- tool start/output/end events use existing `AgentEvent`/`RuntimeEvent` paths
- policy decisions appear through trust/policy warning surfaces
- extension load failures appear in status/diagnostics, not as panics
- tool details should include extension id and manifest path when helpful

No TUI-specific execution logic should be added; the TUI observes runtime/tool
events emitted by core.

## Example extension

A runnable example package lives at `extensions/example`:

```text
extensions/example/
  imp.extension.json
  package.json
  src/tool.mjs
```

It intentionally uses Node built-ins only. The manifest declares:

- `example_echo` — read-only one-shot JSON tool used by Rust integration tests.
- `example_write_demo` — workspace-write demo restricted to
  `tmp/example-extension/**` to demonstrate policy-gated side effects.

Run its self-test with:

```sh
node extensions/example/src/tool.mjs --self-test
```

The core test `loads_example_manifest_extension_fixture` copies this package into
`.imp/extensions/example`, loads it through the real manifest discovery and tool
registration path, then executes `example_echo` through the one-shot JSON bridge.

## Non-goals for 394.10

- No embedded JS runtime.
- No automatic npm install.
- No unrestricted global extension marketplace.
- No ambient secret/environment inheritance.
- No automatic network sandbox beyond manifest/policy warnings unless a separate
  sandbox substrate lands.
- No custom TS-rendered UI components in the TUI/GUI.

## Implementation sequence

1. Add manifest structs and validation in `typescript_extensions::schema`.
2. Extend discovery to prefer `imp.extension.json` manifests and keep legacy Pi
   discovery as compatibility status.
3. Register manifest tools with declared capabilities and normalized JSON Schema.
4. Implement one-shot JSON subprocess execution first; add json-lines server pool
   later if needed.
5. Route each call through `ReferenceMonitor` before spawn.
6. Mediate secrets/env/cwd and enforce timeout/output limits.
7. Add a minimal example TS extension package under `extensions/example`.
8. Surface extension status in TUI/CLI.
9. Document authoring/security model with manifest examples.
