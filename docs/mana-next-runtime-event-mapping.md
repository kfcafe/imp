# mana-next Runtime Event Mapping

Status: design draft for imp-next  
Parent: mana `394.3` / child `394.3.6`

## Purpose

This document defines which imp runtime events should create durable mana-next workflow ledger updates.

Rule of thumb:

- **mana stores durable summaries and artifact refs**
- **run artifacts store raw traces/logs/tool output**
- **TUI shows live state**

Do not write every event to mana. Most runtime events belong only in `trace.jsonl`.

## Mapping table

<table>
<thead><tr><th>Runtime event</th><th>mana update</th><th>Write policy</th><th>Notes</th></tr></thead>
<tbody>
<tr><td><code>workflow.started</code></td><td>Create/update WorkflowRecord status = executing; attach workflow contract summary and run_id.</td><td>Automatic for mana-backed or meaningful workflows.</td><td>Trivial chat may remain artifact-only.</td></tr>
<tr><td><code>workflow.phase_changed</code></td><td>Update status only for durable phases: executing, waiting_for_approval, verifying, blocked.</td><td>Throttled/summary only.</td><td>Do not write every minor phase transition.</td></tr>
<tr><td><code>workflow.blocker_set</code></td><td>Add blocker string/ref to WorkflowRecord or TaskRecord.</td><td>Automatic.</td><td>Blockers should be concise and actionable.</td></tr>
<tr><td><code>workflow.blocker_cleared</code></td><td>Mark blocker resolved/removed or append note.</td><td>Automatic when blocker id exists.</td><td>Preserve history in notes or sidecar if needed.</td></tr>
<tr><td><code>workflow.completed</code></td><td>Set final closeout status, evidence refs, verification summary.</td><td>Automatic.</td><td>DONE requires required gates passed.</td></tr>
<tr><td><code>contract.created</code></td><td>Attach workflow_contract_ref and summary fields.</td><td>Automatic for workflows with mana record.</td><td>Full contract lives in run artifacts.</td></tr>
<tr><td><code>tool.started</code></td><td>No mana write.</td><td>Never by default.</td><td>Trace only.</td></tr>
<tr><td><code>tool.completed</code></td><td>No mana write unless it produced an artifact/evidence ref.</td><td>Artifact summary only.</td><td>Example: diff.patch, evidence.md, verify.log.</td></tr>
<tr><td><code>policy.checked</code></td><td>Only write denials, approval-needed, or high-risk allowed decisions as Evidence/Note summary.</td><td>Selective.</td><td>All policy events remain in trace/policy log.</td></tr>
<tr><td><code>approval.requested</code></td><td>Create/update DecisionRecord or blocker.</td><td>Automatic when approval blocks work.</td><td>May map to a Decision if user choice is durable.</td></tr>
<tr><td><code>approval.resolved</code></td><td>Resolve DecisionRecord and clear blocker.</td><td>Automatic.</td><td>Record outcome/rationale.</td></tr>
<tr><td><code>verification.started</code></td><td>Create/update VerificationRecord status = running.</td><td>Automatic for declared gates.</td><td>Do not inline output.</td></tr>
<tr><td><code>verification.completed</code></td><td>Update VerificationRecord status; attach artifact refs and exit code.</td><td>Automatic.</td><td>Required failed/skipped/blocked gates affect closeout.</td></tr>
<tr><td><code>evidence.written</code></td><td>Create EvidenceRecord ref and attach to Workflow/Task.</td><td>Automatic.</td><td>Prefer artifact path + summary.</td></tr>
<tr><td><code>mana.updated</code></td><td>No recursive write.</td><td>Never.</td><td>Trace only to avoid loops.</td></tr>
<tr><td><code>child.started</code></td><td>Add ChildRunRef and/or child Task/Workflow record.</td><td>Automatic when child workflows enabled.</td><td>Role/status only, no transcript.</td></tr>
<tr><td><code>child.completed</code></td><td>Update ChildRunRef status and attach evidence refs.</td><td>Automatic.</td><td>Parent workflow gets summary.</td></tr>
<tr><td><code>eval_candidate.created</code></td><td>Create EvidenceRecord or artifact ref to eval candidate.</td><td>Automatic or user-confirmed depending privacy policy.</td><td>No eval runner in this phase.</td></tr>
</tbody>
</table>

## Automatic vs confirmation-required writes

Automatic writes:

- workflow started/completed summaries
- verification status and artifact refs
- evidence packet refs
- concise blockers
- child run status refs
- closeout status

Confirmation or policy review recommended:

- durable decisions involving user/business choices
- global/project memory writes from low-trust sources
- eval candidates containing sensitive artifacts
- high-risk policy allow records that may expose secrets or external systems

No mana writes:

- raw message deltas
- every tool start/end
- raw stdout/stderr
- full trace JSONL content
- transient token usage updates
- TUI-only view state

## Event-to-record examples

### verification.completed

Event summary:

```json
{
  "type": "verification.completed",
  "workflow_id": "394.2",
  "gate_id": "verify_1",
  "status": "passed",
  "exit_code": 0,
  "artifact": ".imp/runs/run_1/verify.log"
}
```

mana update:

```yaml
VerificationRecord:
  id: verify_1
  workflow_id: '394.2'
  status: passed
  exit_code: 0
  artifact_refs:
    - evidence_verify_1

EvidenceRecord:
  id: evidence_verify_1
  evidence_type: test_output
  artifact:
    path: .imp/runs/run_1/verify.log
```

### policy.checked denial

Event summary:

```json
{
  "type": "policy.checked",
  "tool": "bash",
  "decision": "deny",
  "reason": "outside-workspace-delete"
}
```

mana update:

```yaml
EvidenceRecord:
  evidence_type: policy_decision
  summary: "Denied bash action: outside-workspace-delete"
```

If the denial blocks completion, also add a Workflow blocker.

### evidence.written

Event summary:

```json
{
  "type": "evidence.written",
  "run_id": "run_1",
  "path": ".imp/runs/run_1/evidence.md"
}
```

mana update:

```yaml
EvidenceRecord:
  id: evidence_1
  evidence_type: evidence_packet
  run_id: run_1
  artifact:
    path: .imp/runs/run_1/evidence.md
```

Attach `evidence_1` to the WorkflowRecord.

## Write coalescing

Runtime should batch/coalesce mana updates where possible:

- During a run, keep live details in RuntimeStateSnapshot and trace.
- At significant boundaries, write durable summary updates.
- At closeout, write final status/evidence/verification summary in one update if possible.

This prevents `.mana` churn.

## Failure behavior

If mana update fails:

1. Do not fail the agent run solely because of ledger write failure unless the workflow requires durable ledger persistence.
2. Emit a warning event.
3. Record ledger failure in evidence packet if available.
4. Keep run artifacts intact so a later repair can reconstruct mana refs.

## Open questions

- Should policy denials always become Evidence, or only when they affect closeout?
- Should approval requests always become Decisions, or only if long-lived/blocking?
- How aggressively should closeout coalesce update records?
- Should trivial TUI sessions ever create mana records automatically?
