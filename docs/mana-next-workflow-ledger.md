# mana-next Workflow Ledger Schema

Status: design draft for imp-next  
Parent: mana `394.3` / child `394.3.1`

## Purpose

mana-next is the durable workflow ledger for imp-next. It records the stable state and reviewable evidence of agent work without becoming a project-management UI or a transcript dump.

The user-facing vocabulary should be small:

- **Workflow** — a meaningful unit of work with a contract, status, acceptance, verification, evidence, and closeout.
- **Task** — a decomposed execution unit inside a workflow.
- **Decision** — a blocking or historical choice with an outcome and rationale.
- **Verification** — a required or optional proof gate and its result.
- **Evidence** — an artifact reference or durable observation supporting a claim/status.
- **Note** — scoped progress/context that is useful but not a verified fact.

Existing mana units remain compatible. This document defines how the new vocabulary maps onto the current file-backed mana concepts: epic, task, fact, decision, notes, dependencies, verify commands, acceptance criteria, and artifacts.

## Non-goals

- No storage migration in this task.
- No removal of existing mana primitives.
- No automatic conversion of old facts/notes into workflow evidence.
- No transcript storage in mana records.
- No implementation of new commands in this task.
- No requirement that trivial imp TUI interactions visibly create workflows.

## Design principles

1. **Workflow ledger, not Jira.** mana records execution truth and evidence, not a noisy planning bureaucracy.
2. **Summaries in mana, bulk in artifacts.** Raw traces, logs, diffs, and transcripts live under imp run artifacts; mana stores refs and durable summaries.
3. **Compatibility first.** Existing `mana list/show/create/update/verify/close/decision/notes/deps` should continue to work.
4. **Verification controls completion.** Required verification gates must be represented explicitly.
5. **Facts stay rare.** Stable verified claims can remain facts; transient generated observations should become evidence or notes.
6. **Child work is explicit.** Delegated role runs should have parent/child refs, status, and evidence handoff.

## Entity model

### Workflow

A Workflow is the top-level durable ledger record for meaningful work.

Suggested fields:

```yaml
kind: workflow
id: '394.2'
title: Add workflow contract model above imp agent loop
status: executing
workflow_type: code_change
risk_level: medium
autonomy_mode: safe
workflow_contract_ref:
  run_id: run_2026_05_12_abc
  artifact: .imp/runs/run_2026_05_12_abc/workflow-contract.json
parent: '394'
acceptance:
  - WorkflowContract data model exists
  - Agent run carries the contract without behavior changes
closeout_criteria:
  require_summary: true
  require_evidence_packet: true
  require_required_verification: true
verification_refs:
  - verify_1
artifact_refs:
  - evidence_1
child_run_refs: []
blockers: []
final_status: null
```

Status taxonomy:

- `open`
- `claimed`
- `planned`
- `executing`
- `waiting_for_approval`
- `verifying`
- `blocked`
- `done`
- `done_with_concerns`
- `needs_context`
- `cancelled`
- `archived`

Compatibility mapping:

| mana-next Workflow | Current mana |
|---|---|
| workflow record | `kind: epic` or significant parent `kind: task` |
| workflow status | existing `status` plus closeout convention |
| workflow parent | existing `parent` |
| acceptance | existing `acceptance` |
| verification refs | existing `verify` command plus future sidecar/metadata refs |
| artifact refs | existing `produces`/notes plus future evidence refs |
| child workflows | existing child tasks / dependencies |

### Task

A Task is a decomposed execution unit within a Workflow. Current mana `task` maps naturally here.

Suggested fields:

```yaml
kind: task
id: '394.2.3'
workflow_id: '394.2'
title: Thread workflow contract through agent loop state
status: done
role: coder
assignee: imp
requires:
  - '394.2.2'
produces:
  - crates/imp-core/src/agent/mod.rs
verification_refs:
  - verify_2
evidence_refs:
  - evidence_2
closeout_status: DONE
```

Compatibility mapping:

| mana-next Task | Current mana |
|---|---|
| task | `kind: task` |
| workflow_id | `parent` chain to workflow/epic |
| role | label or future metadata field |
| dependencies | existing `dependencies` |
| requires/produces | existing fields |
| verification | existing `verify` plus future Verification entity |
| evidence | notes/artifacts today; Evidence refs later |

### Decision

A Decision records a choice. It can block workflow progress until resolved.

Suggested fields:

```yaml
kind: decision
id: dec_1
workflow_id: '394.3'
question: Where should workflow run artifacts live?
status: open
options:
  - repo .imp/runs
  - user config dir
  - hybrid
outcome: null
rationale: null
blocks:
  - '394.3.2'
```

Compatibility mapping:

| mana-next Decision | Current mana |
|---|---|
| decision | existing decision records |
| status | existing unresolved/resolved state |
| blocks | existing blocking decisions / dependency notes |
| rationale | existing decision resolution text |

### Verification

A Verification is a proof gate. It may be required or optional.

Suggested fields:

```yaml
kind: verification
id: verify_1
workflow_id: '394.2'
task_id: '394.2.1'
name: Workflow contract tests
gate_type: command
required: true
status: passed
command: cargo test -p imp-core workflow_contract --lib
started_at: '2026-05-12T04:00:00Z'
completed_at: '2026-05-12T04:00:10Z'
exit_code: 0
artifact_refs:
  - evidence_3
```

Status taxonomy:

- `pending`
- `running`
- `passed`
- `failed`
- `skipped`
- `blocked`

Compatibility mapping:

| mana-next Verification | Current mana |
|---|---|
| command gate | existing `verify` field |
| status/result | current `mana verify` outcome plus notes/logs |
| artifact refs | future Evidence refs; currently notes/produces |
| required/optional | acceptance text today; explicit field later |

### Evidence

Evidence is a reference to durable proof. It should usually point to an artifact, not inline large content.

Suggested fields:

```yaml
kind: evidence
id: evidence_1
workflow_id: '394.2'
task_id: '394.2.1'
run_id: run_2026_05_12_abc
evidence_type: test_output
trust_label: verifier_output
summary: imp-core workflow contract tests passed
artifact:
  path: .imp/runs/run_2026_05_12_abc/verify.log
  media_type: text/plain
produced_by: verify_1
```

Evidence types:

- `trace`
- `evidence_packet`
- `diff`
- `test_output`
- `policy_decision`
- `tool_observation`
- `manual_review`
- `child_result`
- `eval_candidate`

Compatibility mapping:

| mana-next Evidence | Current mana |
|---|---|
| evidence packet ref | produced artifact / note |
| test output ref | verify output / artifact note |
| diff ref | produced artifact |
| stable verified claim | may become current `fact` only if durable and verified |
| transient observation | note or evidence, not fact |

### Note

A Note is scoped context or progress that is useful but not necessarily verified.

Suggested fields:

```yaml
kind: note
id: note_1
workflow_id: '394.3'
task_id: '394.3.1'
source: agent
trust_label: generated_summary
body: Schema design completed; storage decision deferred to 394.3.2.
```

Compatibility mapping:

| mana-next Note | Current mana |
|---|---|
| note | existing notes_append |
| progress note | existing notes |
| unverified generated observation | note, not fact |

## Relationship model

Core relationships:

```text
Workflow
  ├─ has many Tasks
  ├─ has many Decisions
  ├─ has many Verifications
  ├─ has many Evidence refs
  ├─ has many Notes
  └─ may have child Workflows / child Runs

Task
  ├─ belongs to Workflow
  ├─ depends on Tasks
  ├─ may require Decisions
  ├─ may produce Evidence
  └─ may have Verification gates

Verification
  ├─ belongs to Workflow and optionally Task
  ├─ produces Evidence
  └─ affects closeout

Evidence
  ├─ belongs to Workflow and optionally Task/Verification
  ├─ references artifact path
  └─ may be summarized in mana
```

## Artifact reference format

Artifact refs should be stable, relative when possible, and not inline bulk data.

Suggested shape:

```yaml
artifact:
  path: .imp/runs/run_2026_05_12_abc/evidence.md
  kind: evidence_packet
  media_type: text/markdown
  sha256: optional
  bytes: optional
```

Common imp artifacts:

```text
.imp/runs/<run-id>/workflow-contract.json
.imp/runs/<run-id>/trace.jsonl
.imp/runs/<run-id>/evidence.md
.imp/runs/<run-id>/diff.patch
.imp/runs/<run-id>/verify.log
.imp/runs/<run-id>/policy.jsonl
```

Implementation note: `crates/imp-core/src/storage.rs` provides `RunArtifacts`,
`project_run_artifacts`, `global_run_artifacts`, and `run_artifacts_under` helpers.
Run ids are restricted to ASCII letters, numbers, `_`, and `-` so artifact roots
cannot escape the selected runs directory.

## Workflow contract refs

The workflow ledger should not duplicate the whole runtime contract in many places. It should store a summary and a reference.

Suggested shape:

```yaml
workflow_contract_ref:
  run_id: run_2026_05_12_abc
  artifact: .imp/runs/run_2026_05_12_abc/workflow-contract.json
summary:
  workflow_type: code_change
  risk_level: medium
  autonomy_mode: local-auto
  workspace_scope: repo
```

## Child run refs

Child workflow delegation should be represented explicitly.

```yaml
child_run_refs:
  - child_id: child_1
    role: verifier
    status: done
    workflow_id: '394.13.child.1'
    evidence_refs:
      - evidence_child_1
```

Existing mana compatibility can model child runs as child tasks initially.

## Compatibility constraints

1. Existing file-backed mana units under `~/.mana` must remain readable.
2. Existing `kind: epic`, `kind: task`, and `kind: fact` are not removed.
3. Existing `verify` command remains the basic Verification input.
4. Existing `acceptance`, `requires`, `produces`, `dependencies`, `decisions`, and `notes` remain meaningful.
5. New workflow-ledger fields should be additive metadata, sidecars, or compatible frontmatter fields until a storage strategy is chosen.
6. Old facts should not automatically become trusted workflow facts; migration must preserve trust boundaries.

## Mapping summary

<table>
<thead><tr><th>mana-next</th><th>Current mana</th><th>Compatibility behavior</th></tr></thead>
<tbody>
<tr><td>Workflow</td><td>epic or significant task</td><td>Additive fields; parent/child tree still works.</td></tr>
<tr><td>Task</td><td>task</td><td>Direct mapping.</td></tr>
<tr><td>Decision</td><td>decision</td><td>Direct mapping with clearer blocking semantics.</td></tr>
<tr><td>Verification</td><td>verify command</td><td>Existing command becomes first gate; richer gate metadata later.</td></tr>
<tr><td>Evidence</td><td>produces/notes/facts</td><td>Prefer artifact refs; facts only for stable verified claims.</td></tr>
<tr><td>Note</td><td>notes_append</td><td>Direct mapping; scoped provenance later.</td></tr>
</tbody>
</table>

## Open questions for child tasks

- Storage strategy: frontmatter fields, sidecar JSON, SQLite index, or hybrid? See `394.3.2`.
- Compatibility adapter details: how old units are read as workflow-ledger records? See `394.3.3`.
- Which commands should expose the new vocabulary first?
- Should workflow IDs always equal mana unit IDs, or should imp run workflows have separate run IDs linked to mana units?
- How much workflow metadata belongs in `~/.mana` versus repo `.imp/runs` artifacts?
