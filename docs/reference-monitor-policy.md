# Reference monitor policy design

`ReferenceMonitor` is the runtime boundary that should answer one question before
or around every tool action: **may this action proceed, and what audit record
should explain that answer?** Today imp has several correct-but-scattered checks.
This document maps those checks and defines the target architecture for the
394.5 migration.

## Current scattered checks

### `agent/tool_execution.rs`

`crates/imp-core/src/agent/tool_execution.rs` is the current enforcement hot
path. It performs policy checks in this order:

1. `RepeatedToolCallCheck` warns on the third identical call and blocks at four
   or more identical consecutive calls.
2. `BeforeToolCall` hooks run before built-in policy checks. Blocking hook
   results can deny execution.
3. `AgentMode::allows_tool` denies tools not available in the current mode.
4. `RunPolicy::check_tool` applies per-run allow/deny tool lists.
5. Blocking hook results are converted into tool errors.
6. Bash calls are checked for mana-equivalent commands with
   `mana_bash_equivalent_hint` and can be blocked with a native mana-tool hint.
7. Mana calls are checked with `evaluate_mana_policy`, which applies
   `AgentMode::allows_mana_action` and records mana action class details.
8. Tool argument schema validation rejects malformed tool calls before execute.
9. Missing tools return an `Unknown tool` error.
10. `AfterToolCall` hooks may modify results.
11. After successful `write`/`edit`/`multi_edit`, `AfterFileWrite` hooks run.
12. After-write guardrails can append advisory output or mark the result as an
    error when `GuardrailLevel::Enforce` and a check failed.

These checks produce user-visible tool errors and recovery checkpoint labels, but
most are not first-class policy records yet.

### `policy.rs` `RunPolicy`

`crates/imp-core/src/policy.rs` defines per-run constraints:

- tool allowlist/denylist via `check_tool`
- write allowlist/denylist via `check_write_path`

`RunPolicy` is narrower than `AgentMode`: it constrains a single automation run.
Today tool-level checks are wired in `tool_execution.rs`; path checks are passed
through `ToolContext` and enforced by individual tools that opt in.

### `config.rs` `AgentMode`

`AgentMode` is the coarse role policy:

- `Full`: all tools and mana actions
- `Worker`: implementation tools and limited progress-checkpoint mana actions
- `Orchestrator`: mana orchestration with no direct file write tool
- `Planner`: read/create/update planning graph operations
- `Reviewer`: read-only inspection; no mana actions
- `Auditor`: read/report-oriented code and mana inspection

`AgentMode::allows_tool` and `AgentMode::allows_mana_action` are currently called
from execution-time checks. The monitor should preserve these defaults exactly.

### Hooks

`hooks.rs` supports blocking `BeforeToolCall` hooks and result-modifying
`AfterToolCall` hooks. A blocking before-hook is currently an execution denial.
The monitor should normalize it as a policy decision with source `hook` while
keeping hook execution order compatible during migration.

### Mana loop policy

`agent/mana_loop.rs` classifies mana actions (`inspect`, `lifecycle`,
`orchestration`, `destructive`, etc.) and denies actions not available in the
current `AgentMode`. That decision is already structured as `ManaPolicyDecision`;
it should become a nested/reference-monitor policy record rather than a special
case in `tool_execution.rs`.

### Bash-equivalent blocking

`mana_bash_equivalent_hint` blocks shell commands that should use the native
mana tool. This is a policy check, not a bash implementation detail. The monitor
should represent it as `Denied { source: BashEquivalent, reason, suggestion }`.

### Repeated-call blocking

Repeated-call detection is an action-safety guard. The monitor should emit:

- `Warn` at the existing third identical call threshold
- `Deny` at the existing fourth-and-later threshold

It must not reset or reorder current repeated-call state semantics.

### Schema validation

JSON-schema validation rejects malformed tool args before execution. It is not a
permission decision, but it is still a reference-monitor-adjacent precondition.
The target monitor can report it as `InvalidInput` while preserving the current
self-correction behavior.

### After-write guardrails

Guardrails run after a successful write/edit. They are not pre-execution
permission checks; they are post-action policy observations. The monitor should
record advisory/enforced outcomes and whether enforcement changed the tool result
to an error. Later verification-gate work can consume these records.

## Target architecture

`ReferenceMonitor` lives in `imp-core` and is called from the tool execution
runtime, not from individual UI surfaces. It should be deterministic, explicit,
and auditable.

```text
Agent::execute_one_tool
  -> ReferenceMonitor::preflight_tool(input)
       AgentMode tool permission
       RunPolicy tool permission
       bash-equivalent blocking
       mana action policy
       repeated-call warn/deny
       schema-valid precondition metadata
  -> tool.execute(...)
  -> ReferenceMonitor::post_tool(input, result)
       after-write guardrail outcomes
       hook outcomes normalized as policy records
       verification-required hints
  -> trace/evidence policy records
```

Hook execution can remain physically outside the monitor at first, but hook
outcomes should be converted to the same `PolicyDecision` record format.

## Inputs

A monitor decision needs enough context to be replayable without depending on UI
state:

- run id / workflow id when available
- turn number and tool call id
- tool name and normalized tool metadata
- raw args or redacted args hash, depending trace privacy policy
- cwd and extracted path for file tools
- `AgentMode`
- `RunPolicy`
- workflow autonomy mode, risk level, and trust labels from the workflow contract
- current repeated-call state
- hook outcome summary
- mana action and action class for mana calls
- guardrail profile/level and after-write command results for post-action checks

## Tool metadata

Every registered tool should expose or derive:

- stable name
- readonly/mutable/external-side-effect classification
- filesystem read/write/network/process/secrets capabilities when known
- whether it supports dry-run
- whether it can require user approval
- whether it can produce verification evidence

Initial migration can derive this from existing `Tool::is_readonly`, known names
(`bash`, `git`, `mana`), and schema/path extraction helpers. Later extension work
should require manifests to declare these capabilities.

## Decision model

The first Rust model should be small and additive:

```rust
enum PolicyDecisionKind {
    Allow,
    Warn,
    Deny,
    InvalidInput,
    RequireApproval,
    RequireVerification,
    Observe,
}

struct PolicyDecision {
    id: String,
    kind: PolicyDecisionKind,
    source: PolicySource,
    tool_call_id: Option<String>,
    tool_name: Option<String>,
    reason: Option<String>,
    suggestion: Option<String>,
    details: serde_json::Value,
}

enum PolicySource {
    AgentMode,
    RunPolicy,
    ManaLoop,
    BashEquivalent,
    RepeatedCall,
    Hook,
    Schema,
    Guardrail,
    WorkflowAutonomy,
    TrustLabel,
}
```

`Deny` maps to the existing tool-error result. `Warn` is appended/surfaced but
allows execution. `RequireApproval` should feed the existing UI confirmation path
when available and deny or pause when no UI can answer. `RequireVerification` is a
workflow obligation, not an immediate tool denial unless a later policy says so.

## Logging hooks

Every non-trivial decision should emit a policy record to the 394.4 trace and be
summarized in `evidence.md` when it matters:

- all denies
- all approval requests/resolutions
- all enforced guardrail failures
- repeated-call warnings/denials
- high-risk allows in allow-all/high-autonomy modes
- verification-required outcomes

Raw trace can keep more detail; evidence should summarize objective facts and
artifact refs. Mana should receive durable summaries/refs, not raw policy logs.

## Compatibility strategy

The first migration must be behavior-preserving under default safe/full mode:

- preserve current `tool_execution.rs` ordering unless a task explicitly changes
  it
- preserve current error strings where tests or user behavior depend on them
- keep `AgentMode` and `RunPolicy` public APIs initially
- keep hook execution behavior, including blocking before-hooks and result
  modification after-hooks
- keep mana bash-equivalent blocking and mana action class details
- keep after-write guardrail advisory/enforce semantics
- fail closed for monitor errors that affect permission decisions; fail open only
  for logging/evidence write failures

## Phased migration

1. Add Rust types for monitor inputs, outputs, sources, and decision kind.
2. Add tool metadata classification with compatibility defaults from existing
   tool registry behavior.
3. Implement a `ReferenceMonitor` facade that delegates to existing
   `AgentMode`, `RunPolicy`, mana policy, bash-equivalent, repeated-call, schema,
   hooks, and guardrail helpers.
4. Route `tool_execution.rs` through the facade one check at a time, preserving
   ordering and tests.
5. Emit policy decision events into trace/evidence.
6. Thread workflow autonomy/trust context into monitor inputs.
7. Add approval/dry-run/require-verification outcomes once there is a clear UI
   and workflow closeout contract.
8. Document tool-author requirements for core, Lua, and future TypeScript
   extensions.

## Implemented 394.5 slice

The first implementation lives in `crates/imp-core/src/reference_monitor.rs` and
is intentionally additive. It does not remove the older policy APIs yet; it wraps
and records them so later tasks can migrate checks one at a time.

Implemented pieces:

- `ReferenceMonitor`, `ToolPolicyContext`, `ToolPolicyDecision`,
  `PolicyReason`, `PolicySource`, `PolicyTraceRecord`, `ToolMetadata`,
  `ToolActionKind`, and `ResourceScope`.
- `Tool::policy_metadata()` and `ToolRegistry::policy_metadata(...)`.
- Name-based compatibility metadata for native tools including `read`, `write`,
  `edit`, `multi_edit`, `bash`, `git`, `worktree`, `mana`, `web`, `ask`,
  `extend`, and `lua:*`/`extension:*` placeholders.
- Resource-scope extraction for common args: file paths, bash command program,
  mana action, and network host.
- `ReferenceMonitor::check_tool_action(...)` wrapping existing
  `AgentMode::allows_tool`, `RunPolicy::check_tool`, and
  `RunPolicy::check_write_path` behavior.
- Adapter records for hook blocking, mana policy decisions, bash-equivalent
  blocking, repeated-call warnings/blocks, schema validation, and after-write
  guardrail outcomes.
- Workflow context threading from `WorkflowContract`, including autonomy mode,
  workflow type, risk level, workspace scope, trust scope, and generated trust
  labels.
- Safe default handling for future non-allow decisions: `ask_user`,
  `dry_run_only`, `sandbox_only`, and `require_verification` are represented as
  data and blocked by tool execution until a real approval/sandbox/verification
  runtime exists.
- `AgentEvent::PolicyChecked` and `policy.checked` trace events.

The first routed checks in `agent/tool_execution.rs` are the mode/run-policy
preflight checks. Legacy user-facing deny messages and recovery checkpoint labels
are preserved for compatibility.

## Tool author requirements

Every tool must have policy metadata. For built-in tools the default implementation
is usually enough:

```rust
fn policy_metadata(&self) -> ToolMetadata {
    ToolMetadata::for_tool_name(self.name(), self.is_readonly())
}
```

Override `policy_metadata` when the name-based default is too vague. A good tool
metadata record should answer:

- Is this tool read-only?
- Can it write to the workspace?
- Can it execute processes or produce external side effects?
- Can it access the network?
- Can it access secrets?
- Is it an extension-provided tool?
- Does it support dry-run or sandbox execution?
- Does it require approval by default?
- What resource scope can be derived from its arguments?

Tool authors should keep argument names conventional where possible:

- `path`, `file`, or `directory` for filesystem resources
- `command` for process execution
- `action` for mana actions
- `url` for network resources

If a tool mutates state but reports `is_readonly() == true`, the reference monitor
will classify it incorrectly. Treat this as a policy bug, not a UI detail.

## Extension tool requirements

Lua tools currently inherit compatibility metadata from their registered name;
future manifest-driven extension tools must declare equivalent metadata in their
manifest. The Rust runtime must not trust extension code to self-enforce policy.
The monitor should receive a host-owned metadata record before the extension
subprocess/interpreter is invoked.

Future TypeScript extension manifests from 394.10 should include at least:

- stable tool name and extension id
- action kind
- readonly/workspace-write/network/process/secrets capabilities
- dry-run/sandbox support
- approval requirement
- resource argument mapping
- verification evidence capability

Extensions cannot bypass `policy.checked` tracing or non-allow blocking.

## Policy reason codes

Current stable reason codes include:

- `agent_mode_tool_denied`
- `run_policy_tool_denied`
- `run_policy_write_path_denied`
- `hook_blocked`
- `mana_policy_allowed`
- `mana_policy_blocked`
- `policy_blocked` for bash-equivalent blocks
- `repeated_tool_call_warned`
- `repeated_tool_call_blocked`
- `validation_error`
- `guardrail_passed`
- `guardrail_advisory_failed`
- `guardrail_enforced`
- `ask_user_required`
- `dry_run_required`
- `sandbox_required`
- `require_verification`

Compatibility checkpoint labels currently map selected policy codes to legacy
classes such as `mode_blocked`, `run_policy_blocked`, `approval_required`,
`dry_run_required`, `sandbox_required`, and `verification_required`.

## `policy.checked` trace events

Tool execution emits `AgentEvent::PolicyChecked` for the routed monitor decision.
That converts to a `policy.checked` trace event and is serialized by CLI/RPC as
`policy_checked`. The TUI intentionally treats it as a no-op display event for
now.

The trace payload includes:

- tool name
- action kind
- decision and reason
- resource scope summary
- autonomy mode, workflow type, and risk level
- trust scope and trust labels
- details supplied by adapter records
- argument hash when available

Raw tool arguments are not emitted in `policy.checked`; use hashes and summarized
resource scopes to keep policy traces useful without turning them into another
transcript sink. This complements the trace/evidence artifacts from 394.4.

## Current compatibility limitations and non-goals

- Only the AgentMode and RunPolicy preflight checks are physically routed through
  `ReferenceMonitor::check_tool_action` today.
- Hook, mana, bash-equivalent, repeated-call, schema, and guardrail outcomes have
  normalized policy-record adapters, but most execution flow remains in its
  existing location.
- `ask_user`, `dry_run_only`, `sandbox_only`, and `require_verification` do not
  yet perform approval UX, dry-run execution, sandboxing, or verification-gate
  scheduling. They fail closed as blocked tool results.
- Full autonomy-mode semantics belong to 394.6.
- Rich context provenance and low-trust source labeling belongs to 394.8.
- Manifest-driven TypeScript extension metadata belongs to 394.10.
- Verification gate closeout belongs to 394.7.

The intended migration path is to keep moving checks into the monitor facade
without changing user-visible behavior unless a task explicitly calls for it.

## Implemented 394.5 slice

The first implementation lives in `crates/imp-core/src/reference_monitor.rs` and
is intentionally additive. It does not remove the older policy APIs yet; it wraps
and records them so later tasks can migrate checks one at a time.

Implemented pieces:

- `ReferenceMonitor`, `ToolPolicyContext`, `ToolPolicyDecision`,
  `PolicyReason`, `PolicySource`, `PolicyTraceRecord`, `ToolMetadata`,
  `ToolActionKind`, and `ResourceScope`.
- `Tool::policy_metadata()` and `ToolRegistry::policy_metadata(...)`.
- Name-based compatibility metadata for native tools including `read`, `write`,
  `edit`, `multi_edit`, `bash`, `git`, `worktree`, `mana`, `web`, `ask`,
  `extend`, and `lua:*`/`extension:*` placeholders.
- Resource-scope extraction for common args: file paths, bash command program,
  mana action, and network host.
- `ReferenceMonitor::check_tool_action(...)` wrapping existing
  `AgentMode::allows_tool`, `RunPolicy::check_tool`, and
  `RunPolicy::check_write_path` behavior.
- Adapter records for hook blocking, mana policy decisions, bash-equivalent
  blocking, repeated-call warnings/blocks, schema validation, and after-write
  guardrail outcomes.
- Workflow context threading from `WorkflowContract`, including autonomy mode,
  workflow type, risk level, workspace scope, trust scope, and generated trust
  labels.
- Safe default handling for future non-allow decisions: `ask_user`,
  `dry_run_only`, `sandbox_only`, and `require_verification` are represented as
  data and blocked by tool execution until a real approval/sandbox/verification
  runtime exists.
- `AgentEvent::PolicyChecked` and `policy.checked` trace events.

The first routed checks in `agent/tool_execution.rs` are the mode/run-policy
preflight checks. Legacy user-facing deny messages and recovery checkpoint labels
are preserved for compatibility.

## Tool author requirements

Every tool must have policy metadata. For built-in tools the default implementation
is usually enough:

```rust
fn policy_metadata(&self) -> ToolMetadata {
    ToolMetadata::for_tool_name(self.name(), self.is_readonly())
}
```

Override `policy_metadata` when the name-based default is too vague. A good tool
metadata record should answer:

- Is this tool read-only?
- Can it write to the workspace?
- Can it execute processes or produce external side effects?
- Can it access the network?
- Can it access secrets?
- Is it an extension-provided tool?
- Does it support dry-run or sandbox execution?
- Does it require approval by default?
- What resource scope can be derived from its arguments?

Tool authors should keep argument names conventional where possible:

- `path`, `file`, or `directory` for filesystem resources
- `command` for process execution
- `action` for mana actions
- `url` for network resources

If a tool mutates state but reports `is_readonly() == true`, the reference monitor
will classify it incorrectly. Treat this as a policy bug, not a UI detail.

## Extension tool requirements

Lua tools currently inherit compatibility metadata from their registered name;
future manifest-driven extension tools must declare equivalent metadata in their
manifest. The Rust runtime must not trust extension code to self-enforce policy.
The monitor should receive a host-owned metadata record before the extension
subprocess/interpreter is invoked.

Future TypeScript extension manifests from 394.10 should include at least:

- stable tool name and extension id
- action kind
- readonly/workspace-write/network/process/secrets capabilities
- dry-run/sandbox support
- approval requirement
- resource argument mapping
- verification evidence capability

Extensions cannot bypass `policy.checked` tracing or non-allow blocking.

## Policy reason codes

Current stable reason codes include:

- `agent_mode_tool_denied`
- `run_policy_tool_denied`
- `run_policy_write_path_denied`
- `hook_blocked`
- `mana_policy_allowed`
- `mana_policy_blocked`
- `policy_blocked` for bash-equivalent blocks
- `repeated_tool_call_warned`
- `repeated_tool_call_blocked`
- `validation_error`
- `guardrail_passed`
- `guardrail_advisory_failed`
- `guardrail_enforced`
- `ask_user_required`
- `dry_run_required`
- `sandbox_required`
- `require_verification`

Compatibility checkpoint labels currently map selected policy codes to legacy
classes such as `mode_blocked`, `run_policy_blocked`, `approval_required`,
`dry_run_required`, `sandbox_required`, and `verification_required`.

## `policy.checked` trace events

Tool execution emits `AgentEvent::PolicyChecked` for the routed monitor decision.
That converts to a `policy.checked` trace event and is serialized by CLI/RPC as
`policy_checked`. The TUI intentionally treats it as a no-op display event for
now.

The trace payload includes:

- tool name
- action kind
- decision and reason
- resource scope summary
- autonomy mode, workflow type, and risk level
- trust scope and trust labels
- details supplied by adapter records
- argument hash when available

Raw tool arguments are not emitted in `policy.checked`; use hashes and summarized
resource scopes to keep policy traces useful without turning them into another
transcript sink. This complements the trace/evidence artifacts from 394.4.

## Current compatibility limitations and non-goals

- Only the AgentMode and RunPolicy preflight checks are physically routed through
  `ReferenceMonitor::check_tool_action` today.
- Hook, mana, bash-equivalent, repeated-call, schema, and guardrail outcomes have
  normalized policy-record adapters, but most execution flow remains in its
  existing location.
- `ask_user`, `dry_run_only`, `sandbox_only`, and `require_verification` do not
  yet perform approval UX, dry-run execution, sandboxing, or verification-gate
  scheduling. They fail closed as blocked tool results.
- Full autonomy-mode semantics belong to 394.6.
- Rich context provenance and low-trust source labeling belongs to 394.8.
- Manifest-driven TypeScript extension metadata belongs to 394.10.
- Verification gate closeout belongs to 394.7.

The intended migration path is to keep moving checks into the monitor facade
without changing user-visible behavior unless a task explicitly calls for it.
