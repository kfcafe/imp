# Trace and Evidence Format

Status: design draft for imp-next  
Parent: mana `394.4` / child `394.4.1`

## Purpose

imp-next should emit a structured raw trace for every meaningful workflow run. The trace is a replay/debug/audit artifact. It is not the human-facing summary.

Two artifacts are intentionally separate:

- **`trace.jsonl`** — raw, structured, append-only runtime events.
- **`evidence.md`** — human-readable review packet derived from trace, workflow contract, verification gates, diffs, policy decisions, and closeout.

This document specifies the versioned TraceEvent JSONL schema and maps existing `AgentEvent` variants to trace records. Evidence packet structure is specified by later children in `394.4`.

## File format

`trace.jsonl` is newline-delimited JSON:

```text
{"schema_version":1,"kind":"agent.start",...}
{"schema_version":1,"kind":"turn.start",...}
{"schema_version":1,"kind":"tool.execution.start",...}
```

Rules:

- one complete JSON object per line
- append-only during a run
- flush at significant boundaries where practical
- stable `schema_version`
- monotonic `sequence` number per run
- timestamps in Unix milliseconds or RFC3339; implementation should choose one and document it in the Rust type
- no raw secrets
- large fields are truncated or stored by artifact ref

## TraceEvent envelope

Every event should share a common envelope.

```json
{
  "schema_version": 1,
  "sequence": 42,
  "timestamp": 1778568000000,
  "run_id": "run_2026_05_12_abc",
  "workflow_id": "394.2",
  "session_id": "session_abc",
  "turn": 3,
  "kind": "tool.execution.start",
  "correlation": {
    "message_id": "msg_1",
    "tool_call_id": "call_1",
    "parent_event_id": "event_41"
  },
  "redaction": {
    "contains_redactions": false,
    "truncated_fields": [],
    "content_hash": null
  },
  "payload": {}
}
```

Required fields:

| Field | Required | Notes |
|---|---:|---|
| `schema_version` | yes | starts at `1` |
| `sequence` | yes | monotonic per run |
| `timestamp` | yes | event time |
| `run_id` | yes | imp run artifact id |
| `kind` | yes | stable event kind string |
| `payload` | yes | kind-specific object |

Optional fields:

| Field | Notes |
|---|---|
| `workflow_id` | mana/workflow contract id when available |
| `session_id` | imp session id when available |
| `turn` | current turn index when available |
| `correlation` | message/tool/parent ids |
| `redaction` | redaction/truncation metadata |

## Redaction model

Trace events must be useful without becoming a secret leak.

### Redaction metadata

```json
{
  "contains_redactions": true,
  "truncated_fields": ["payload.args.command"],
  "content_hash": "sha256:..."
}
```

### Rules

- Do not store full secret values.
- Hash or omit sensitive args where practical.
- Truncate large deltas/output.
- Store large outputs in artifact files when needed.
- Tool args should be redacted according to tool metadata and policy.
- External low-trust content should carry provenance/trust metadata once 394.8 exists.

Suggested defaults:

| Content | Trace behavior |
|---|---|
| assistant text deltas | truncate long deltas |
| tool args | include redacted JSON |
| bash command | include command unless policy marks sensitive; truncate long commands |
| tool stdout/stderr | deltas truncated; full output goes to artifact when needed |
| secrets/env | omit or hash only |
| file contents | avoid full contents unless already in model context; prefer path/hash |

## Event kind taxonomy

Use dot-separated stable names:

```text
agent.start
agent.end
turn.start
turn.assessment
turn.end
message.start
message.delta
message.end
tool.execution.start
tool.output.delta
tool.execution.end
warning
error
timing
recovery.checkpoint
policy.checked
verification.started
verification.completed
evidence.written
workflow.phase_changed
```

Some kinds are future-facing. The initial mapping should cover existing `AgentEvent`; later children add policy/verification/evidence events.

## Mapping from existing AgentEvent

Existing source: `crates/imp-core/src/agent/events.rs`.

<table>
<thead><tr><th>AgentEvent</th><th>Trace kind</th><th>Payload summary</th><th>Notes</th></tr></thead>
<tbody>
<tr><td><code>AgentStart { model, timestamp }</code></td><td><code>agent.start</code></td><td>model, source timestamp</td><td>Start of run/agent execution.</td></tr>
<tr><td><code>AgentEnd { usage, cost, status }</code></td><td><code>agent.end</code></td><td>usage, cost, final status</td><td>Status serialized from <code>RunFinalStatus</code>.</td></tr>
<tr><td><code>TurnStart { index }</code></td><td><code>turn.start</code></td><td>turn index</td><td>Also sets envelope <code>turn</code>.</td></tr>
<tr><td><code>TurnAssessment { index, assessment }</code></td><td><code>turn.assessment</code></td><td>assessment summary</td><td>May need compact serialization of <code>NextActionAssessment</code>.</td></tr>
<tr><td><code>TurnEnd { index, message, mana_review }</code></td><td><code>turn.end</code></td><td>assistant message summary, mana review summary</td><td>Do not inline huge message content in future if artifact exists.</td></tr>
<tr><td><code>MessageStart { message }</code></td><td><code>message.start</code></td><td>role/type/message id when available</td><td>Payload may include redacted message summary.</td></tr>
<tr><td><code>MessageDelta { delta }</code></td><td><code>message.delta</code></td><td>stream delta</td><td>Truncate text deltas; preserve tool-call delta ids.</td></tr>
<tr><td><code>MessageEnd { message }</code></td><td><code>message.end</code></td><td>message summary</td><td>May include token/message metadata.</td></tr>
<tr><td><code>ToolExecutionStart { tool_call_id, tool_name, args }</code></td><td><code>tool.execution.start</code></td><td>tool id/name/redacted args</td><td>Set correlation.tool_call_id.</td></tr>
<tr><td><code>ToolOutputDelta { tool_call_id, text }</code></td><td><code>tool.output.delta</code></td><td>truncated text delta</td><td>Set correlation.tool_call_id.</td></tr>
<tr><td><code>ToolExecutionEnd { tool_call_id, result }</code></td><td><code>tool.execution.end</code></td><td>status/result summary</td><td>Do not inline very large results.</td></tr>
<tr><td><code>Warning { message }</code></td><td><code>warning</code></td><td>message</td><td>Warnings may also appear in evidence summary.</td></tr>
<tr><td><code>Timing { timing }</code></td><td><code>timing</code></td><td>turn, stage, durations, label, success</td><td>Uses <code>TimingEvent</code>/<code>TimingStage</code>.</td></tr>
<tr><td><code>RecoveryCheckpoint { checkpoint }</code></td><td><code>recovery.checkpoint</code></td><td>checkpoint</td><td>Already versioned; preserve args_hash.</td></tr>
<tr><td><code>Error { error }</code></td><td><code>error</code></td><td>error class/message</td><td>Redact paths/secrets if needed.</td></tr>
</tbody>
</table>

## TimingEvent mapping

Existing `TimingEvent` fields map directly:

```json
{
  "kind": "timing",
  "payload": {
    "stage": "llm_request_start",
    "since_turn_start_ms": 120,
    "since_llm_request_start_ms": null,
    "duration_ms": null,
    "label": null,
    "success": null
  }
}
```

`TimingStage::as_str()` provides stable stage names.

## RecoveryCheckpoint mapping

Existing `RecoveryCheckpoint` is already versioned.

Trace payload should include:

- checkpoint version
- turn
- kind
- tool_call_id
- tool_name
- args_hash
- success
- error_class
- checkpoint timestamp

The trace envelope timestamp may differ from checkpoint timestamp; keep both if useful.

## Correlation rules

- Tool events set `correlation.tool_call_id`.
- Message events set `correlation.message_id` if known.
- Timing/recovery events include turn and optional tool id.
- Future policy events should correlate to tool_call_id.
- Future verification events should correlate to gate id.
- Future evidence events should correlate to artifact/evidence id.

## Relationship to TurnTracker

TUI `TurnTracker` currently classifies files read/written, commands, searches, and tool errors. This logic is useful for evidence summaries but should not be the trace schema.

Trace should store raw structured tool lifecycle events. Evidence packet builders can derive TurnTracker-like summaries from trace events in core rather than depending on TUI state.

## Evidence Packet data model

An evidence packet is the human-readable summary produced for meaningful workflow runs. It is derived from runtime trace, workflow contract, verification gates, policy decisions, artifact refs, and final closeout.

The initial data model should be small and renderable to Markdown.

```text
EvidencePacket
  run_id
  workflow_id?
  session_id?
  objective
  workflow_type
  risk_level
  autonomy_mode
  final_status?
  summary
  plan[]
  actions
    files_inspected[]
    files_changed[]
    commands_run[]
    searches[]
    tools[]
  policy
    decisions[]
    denials[]
    approvals[]
  verification
    gates[]
  artifacts[]
  concerns[]
  next_steps[]
```

Suggested Rust shape:

```rust
struct EvidencePacket {
    run_id: String,
    workflow_id: Option<String>,
    session_id: Option<String>,
    objective: String,
    workflow_summary: WorkflowSummary,
    final_status: Option<String>,
    summary: Vec<String>,
    plan: Vec<String>,
    actions: EvidenceActions,
    policy: EvidencePolicy,
    verification: Vec<EvidenceVerificationGate>,
    artifacts: Vec<EvidenceArtifact>,
    concerns: Vec<String>,
    next_steps: Vec<String>,
}
```

## Evidence Packet Markdown sections

`evidence.md` should render stable sections in this order:

```markdown
# Evidence Packet

## Workflow

## Summary

## Plan

## Actions

## Policy

## Verification

## Artifacts

## Closeout
```

### Workflow

Include:

- objective
- workflow id / mana id when present
- run id
- workflow type
- risk level
- autonomy mode
- workspace scope when useful

### Summary

Include:

- final status
- concise outcome
- changed files count/list
- verification summary
- concerns count/list

### Plan

Include the plan when the agent produced one. For trivial runs this may say:

```text
No explicit plan was required for this small change.
```

### Actions

Summarize observed work:

- files inspected
- files changed
- commands run
- searches performed
- tools called
- notable warnings/errors

Do not inline giant tool output.

### Policy

Summarize:

- autonomy mode
- policy denials
- approval requests/resolutions
- high-risk actions allowed by mode
- hard-rail warnings

`allow-all` mode must be visible here.

### Verification

For each gate:

- name/id
- required/optional
- command or gate type
- status
- exit code if command-backed
- artifact/log ref

Required failed/skipped/blocked gates should be obvious.

### Artifacts

List artifact refs:

- `trace.jsonl`
- `evidence.md`
- `diff.patch`
- `verify.log`
- `policy.jsonl`
- child workflow evidence later

### Closeout

Include:

- final status: `DONE`, `DONE_WITH_CONCERNS`, `BLOCKED`, `NEEDS_CONTEXT`, or `CANCELLED`
- rationale
- concerns
- next useful action

## Data sources

Evidence packet fields map to these sources:

| Evidence field | Source |
|---|---|
| objective/workflow | `WorkflowContract` |
| tool actions | `TraceEvent` / `AgentEvent` summary |
| policy | `policy.checked` trace events |
| verification | `VerificationGateResult` |
| artifacts | run artifact helpers |
| final status | `RunFinalStatus` / closeout evaluator |
| mana refs | mana workflow ledger adapter |

## Omission rules

For trivial or incomplete runs:

- Empty sections can render as `None recorded.`
- Missing workflow id is allowed.
- Missing verification gates means `No verification gates were declared.`
- Missing evidence due to crash should render partial evidence when possible.

## Redaction and truncation

Evidence is human-facing and should be safer than raw traces:

- never print known secrets
- summarize large outputs
- link logs instead of inlining them
- display args only after trace/policy redaction
- include trust/provenance warnings when available

`trace.jsonl` answers:

- what happened, in order?
- what events did the runtime emit?
- what tool calls ran?
- what warnings/errors/timing/recovery checkpoints occurred?

`evidence.md` answers:

- what changed?
- what was verified?
- what evidence supports the final status?
- what concerns remain?
- where are the artifacts?

Trace is machine/debug oriented. Evidence is human/review oriented.

## Implemented first slice

The current workflow-runtime slice writes run artifacts for agent runs under the
project-local run directory:

```text
.imp/runs/run_<uuid>/
  workflow-contract.json
  trace.jsonl
  evidence.md
```

`trace.jsonl` contains newline-delimited `TraceEvent` records derived from the
agent event stream, including lifecycle, turn, message, tool, timing, recovery,
warning, error, and `evidence.written` events. Large string payloads are
truncated by the trace writer and marked in the event redaction metadata; traces
are still local debug artifacts and should be treated as potentially sensitive.

`evidence.md` is a concise human-review packet. The first implementation records
workflow metadata, final status, basic action summaries derived from tool calls,
and artifact references back to `trace.jsonl` and `workflow-contract.json`.
Policy decisions, verification gates, diffs, and richer provenance are added by
later workflow-runtime slices.

The TUI surfaces the latest evidence path after closeout as a compact system
message:

```text
Evidence: .imp/runs/run_<uuid>/evidence.md
```

It also stores the path in the status item map as `evidence` for existing status
surfaces. If evidence is absent, normal chat behavior is unchanged.

Even high-autonomy modes such as future `allow-all` must keep these artifacts
enabled. Autonomy can reduce prompts; it must not remove auditability. Mana
should store durable summaries and artifact refs rather than inline raw trace or
evidence contents. Future eval-candidate extraction should point at these run
artifacts and apply privacy/redaction policy before promotion.

## Schema evolution

- Increment `schema_version` for breaking changes.
- Additive payload fields do not require a version bump.
- Consumers should ignore unknown fields.
- Event kind names should be stable once released.
- Experimental events can use an `experimental.` prefix if needed.

## Non-goals for this slice

- No reference-monitor policy log beyond the existing trace event stream.
- No verification gate runner or `verify.log` population yet.
- No diff artifact capture yet.
- No mana ledger write path for evidence refs yet.
- No GUI/TUI redesign beyond compact evidence path surfacing.

Those are child tasks under `394.4`.
