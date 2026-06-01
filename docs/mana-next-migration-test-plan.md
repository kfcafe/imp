# mana-next Compatibility and Migration Test Plan

Status: test plan for imp-next  
Parent: mana `394.3` / child `394.3.9`

## Purpose

Before changing mana internals, we need a compatibility test plan that protects current mana behavior while adding workflow-ledger views, sidecars, evidence refs, and imp adapters.

## Test environments

Use temporary mana roots whenever possible:

```text
/tmp/imp-mana-test/.mana
```

Avoid mutating the developer's real `~/.mana` in automated tests.

Tests should cover:

- clean empty mana root
- existing mana root with old epic/task/fact/decision files
- mana root with new `ledger/` sidecars
- repo with `.imp/runs/<run-id>/` artifacts
- missing/broken artifact refs

## Existing command compatibility

These commands must continue to work:

```bash
mana template kind=task
mana list --count 1
mana show <id>
mana create --kind task --title "..."
mana update <id> --notes "..."
mana verify <id>
mana close <id>
mana notes_append <id> --notes "..."
mana decision_add <id> --title "..."
mana decision_resolve <id> --resolve_decisions "..."
mana dep_add --from-id <id> --dep-id <dep>
mana dep_remove --from-id <id> --dep-id <dep>
```

If exact CLI syntax differs, use native mana tool equivalents in integration tests.

## Old unit fixture

Create fixtures for current format:

```text
fixtures/mana-old/
  100-root-epic.md
  100.1-child-task.md
  100.2-task-with-verify.md
  100.3-fact.md
```

Fixture assertions:

- list/show reads all units
- tree parent/child relation works
- verify command is visible
- notes append preserves existing frontmatter
- unknown future fields are preserved or sidecar path is used instead

## New ledger fixture

Create fixtures for sidecars:

```text
fixtures/mana-next/
  200-workflow.md
  ledger/
    workflows/200.json
    verifications/verify_200.json
    evidence/evidence_200.json
    child-runs/child_200.json
```

Fixture assertions:

- existing mana list/show still sees `200-workflow.md`
- adapter reads WorkflowView from markdown + sidecar
- VerificationView reads status/command/artifacts
- EvidenceView reads artifact refs
- child run refs do not break old mana commands

## Artifact fixtures

Create repo artifact fixture:

```text
fixtures/repo/.imp/runs/run_200/
  workflow-contract.json
  trace.jsonl
  evidence.md
  diff.patch
  verify.log
  policy.jsonl
```

Assertions:

- adapter validates existing artifact refs
- missing artifact is reported as broken ref, not panic
- artifact content is not inlined into mana record
- relative paths resolve from repo root

## Adapter tests

### Read mapping

Input: current `kind: task` with parent, acceptance, verify, produces.

Expected:

- TaskView has workflow_id from parent
- verify field maps to required command VerificationView
- produces maps to expected artifact hints
- acceptance maps to closeout criteria

### Workflow mapping

Input: `kind: epic` or task-with-children.

Expected:

- WorkflowView created
- child tasks linked
- status preserved

### Evidence mapping

Input: sidecar evidence ref.

Expected:

- EvidenceView includes kind/path/run_id/summary
- path resolves or reports missing

### Decision mapping

Input: unresolved current mana decision.

Expected:

- DecisionView status open
- workflow/task shows blocker if decision blocks it

## Write adapter tests

### Record workflow started

Action:

- create/update workflow ledger record from WorkflowContract and run_id

Assert:

- existing markdown remains readable
- sidecar or frontmatter summary exists
- no raw trace content written to mana markdown

### Record verification result

Action:

- record passed/failed verification with artifact ref

Assert:

- VerificationRecord status updated
- evidence ref attached
- verify.log path stored, not content

### Record evidence ref

Action:

- record evidence.md ref

Assert:

- EvidenceRecord exists
- WorkflowRecord evidence_refs includes id

### Record closeout

Action:

- update final status DONE/DONE_WITH_CONCERNS/BLOCKED

Assert:

- current mana status maps consistently
- final closeout status preserved
- required verification failure prevents DONE in higher-level tests

## Migration tests

### No migration required

Given old mana files only:

- adapter can read views
- existing commands work
- no sidecars are created until a write occurs

### Additive write

Given old mana file, after imp writes evidence ref:

- old file still readable
- new sidecar/frontmatter is additive
- rollback by deleting sidecar does not destroy old status

### Unknown field preservation

If frontmatter writes are used, assert unknown fields survive roundtrip. If this cannot be guaranteed, prefer sidecars for v1.

## Rollback tests

- Delete `~/.mana/ledger/` sidecars: old mana list/show still works.
- Delete `.imp/runs/<run-id>/` artifact: mana shows broken evidence ref gracefully.
- Disable mana-next adapter: current mana commands remain usable.

## Privacy and trust tests

When trust/redaction support exists:

- low-trust external content cannot become verified fact automatically
- secret-like content is not written into evidence sidecar summaries
- artifact refs may point to redacted logs
- stale facts do not map to high-trust workflow evidence without verification

## CI gates for mana-next implementation

Minimum future CI command set:

```bash
cargo test -p imp-core mana_workflow_ledger
cargo test -p imp-core mana_workflow_ledger_adapter
cargo test -p imp-core workflow_contract
```

When integration fixtures exist:

```bash
cargo test -p imp-core mana_next_fixtures
```

Manual smoke with native mana tool:

```text
mana template kind=task
mana list --count 1
mana show <known-id>
```

## Acceptance for migration readiness

The mana-next implementation is migration-ready only when:

- existing commands pass compatibility tests
- old fixtures load
- new sidecars load
- artifact refs degrade gracefully
- deleting sidecars rolls back safely
- evidence/log content is not inlined into mana records
- low-trust facts are not promoted silently
