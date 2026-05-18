# mana-next Workflow Ledger Examples

Status: examples/templates for imp-next  
Parent: mana `394.3` / child `394.3.7`

These examples demonstrate the streamlined mana-next vocabulary while preserving compatibility with current mana concepts.

## Example: code-change workflow

```yaml
kind: workflow
id: 'wf_auth_001'
title: Fix failing auth session test
status: executing
workflow_type: code_change
risk_level: medium
autonomy_mode: local-auto
workflow_contract_ref:
  run_id: run_auth_001
  artifact:
    kind: workflow_contract
    path: .imp/runs/run_auth_001/workflow-contract.json
acceptance:
  - auth session test passes
  - no unrelated diff
  - evidence packet written
closeout_criteria:
  require_summary: true
  require_evidence_packet: true
  require_required_verification: true
verification_refs:
  - verify_auth_tests
evidence_refs:
  - evidence_run_packet
decision_refs:
  - decision_dependency
note_refs:
  - note_initial_plan
child_run_refs: []
blockers: []
final_status: null
```

## Example: task

```yaml
kind: task
id: task_auth_fix
workflow_id: wf_auth_001
title: Update session expiry comparison
status: executing
role: coder
assignee: imp
dependencies: []
requires:
  - src/auth/session.ts
produces:
  - src/auth/session.ts
  - tests/auth/session.test.ts
verification_refs:
  - verify_auth_tests
evidence_refs: []
blockers: []
closeout_status: null
```

## Example: verification

```yaml
kind: verification
id: verify_auth_tests
workflow_id: wf_auth_001
task_id: task_auth_fix
name: Auth session targeted tests
gate_type: command
required: true
status: passed
command: npm test -- tests/auth/session.test.ts
exit_code: 0
artifact_refs:
  - evidence_verify_log
```

## Example: evidence

```yaml
kind: evidence
id: evidence_run_packet
workflow_id: wf_auth_001
task_id: task_auth_fix
run_id: run_auth_001
evidence_type: evidence_packet
trust_label: generated_summary
summary: Evidence packet for auth session fix.
artifact:
  kind: evidence_packet
  path: .imp/runs/run_auth_001/evidence.md
  media_type: text/markdown
produced_by: run_auth_001
```

```yaml
kind: evidence
id: evidence_verify_log
workflow_id: wf_auth_001
task_id: task_auth_fix
run_id: run_auth_001
evidence_type: test_output
trust_label: verifier_output
summary: Targeted auth session tests passed.
artifact:
  kind: verify_log
  path: .imp/runs/run_auth_001/verify.log
  media_type: text/plain
produced_by: verify_auth_tests
```

## Example: decision

```yaml
kind: decision
id: decision_dependency
workflow_id: wf_auth_001
question: Add a date/time dependency for expiry comparison?
status: resolved
options:
  - add dependency
  - use standard library Date APIs
outcome: use standard library Date APIs
rationale: Existing project has enough date handling for the fix; dependency is unnecessary.
blocks: []
```

## Example: note

```yaml
kind: note
id: note_initial_plan
workflow_id: wf_auth_001
task_id: task_auth_fix
source: agent
trust_label: generated_summary
body: Inspect failing test, find session expiry comparison, patch minimal logic, run targeted test.
```

## Current mana-compatible markdown sketch

A current mana task can remain markdown/frontmatter and gain workflow-ledger refs later.

```markdown
---
id: wf_auth_001
kind: task
title: Fix failing auth session test
status: executing
acceptance:
  - auth session test passes
  - no unrelated diff
verify: npm test -- tests/auth/session.test.ts
labels:
  - workflow
  - code-change
workflow:
  workflow_type: code_change
  risk_level: medium
  autonomy_mode: local-auto
  contract_ref:
    run_id: run_auth_001
    path: .imp/runs/run_auth_001/workflow-contract.json
  verification_refs:
    - verify_auth_tests
  evidence_refs:
    - evidence_run_packet
---

## Notes

- Initial plan recorded in note_initial_plan.
```

## Sidecar layout sketch

```text
~/.mana/ledger/
  workflows/wf_auth_001.json
  verifications/verify_auth_tests.json
  evidence/evidence_run_packet.json
  evidence/evidence_verify_log.json
```

The current mana markdown unit remains readable without these sidecars.

## Artifact layout sketch

```text
repo/.imp/runs/run_auth_001/
  workflow-contract.json
  trace.jsonl
  evidence.md
  diff.patch
  verify.log
  policy.jsonl
```

## Minimal template: workflow

```yaml
kind: workflow
id: '<workflow-id>'
title: '<title>'
status: open
workflow_type: ad-hoc
risk_level: unknown
autonomy_mode: safe
acceptance: []
verification_refs: []
evidence_refs: []
blockers: []
final_status: null
```

## Minimal template: verification

```yaml
kind: verification
id: '<verify-id>'
workflow_id: '<workflow-id>'
gate_type: command
required: true
status: pending
command: '<command>'
artifact_refs: []
```

## Minimal template: evidence

```yaml
kind: evidence
id: '<evidence-id>'
workflow_id: '<workflow-id>'
run_id: '<run-id>'
evidence_type: evidence_packet
summary: '<short summary>'
artifact:
  kind: evidence_packet
  path: .imp/runs/<run-id>/evidence.md
```
