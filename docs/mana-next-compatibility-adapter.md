# mana-next Compatibility Adapter

Status: design draft for imp-next  
Parent: mana `394.3` / child `394.3.3`

## Purpose

The compatibility adapter lets imp-next view existing mana units as workflow-ledger records without breaking current mana commands or requiring a migration.

The adapter is a translation layer:

```text
current mana markdown/frontmatter + optional sidecars + imp run artifacts
  -> WorkflowLedger view
```

It should be additive, reversible, and conservative about trust.

## Inputs

The adapter reads:

- existing `.mana/*.md` unit files
- frontmatter fields: `id`, `title`, `status`, `kind`, `parent`, `dependencies`, `acceptance`, `verify`, `requires`, `produces`, `labels`, paths
- current mana decisions/notes where available
- optional sidecars under `~/.mana/ledger/`
- imp run artifact refs when linked from sidecars/frontmatter/notes

## Outputs

The adapter exposes normalized views:

- WorkflowView
- TaskView
- DecisionView
- VerificationView
- EvidenceView
- NoteView

These views are not necessarily new storage records. They are the compatibility interface used by imp workflow runtime, evidence, TUI summaries, and future mana-next commands.

## Mapping rules

### epic/task to Workflow/Task

Current `kind: epic` maps to Workflow by default.

Current `kind: task` maps to Task by default, except significant parent tasks may also be viewed as Workflow when:

- they have child tasks, or
- they have a workflow sidecar, or
- they are explicitly marked with workflow metadata, or
- imp starts a workflow from that unit.

```text
kind: epic -> Workflow
kind: task + children -> Workflow + Task-compatible view
kind: task + no children -> Task
```

The adapter should support dual views when needed. This avoids forcing users to distinguish epic/task/workflow perfectly.

### verify command to Verification

Current `verify` frontmatter maps to a required command Verification gate.

```yaml
verify: cargo test -p imp-core workflow_contract --lib
```

becomes:

```yaml
kind: verification
required: true
gate_type: command
command: cargo test -p imp-core workflow_contract --lib
status: unknown_or_pending
```

If `mana verify` has a recorded result in future metadata/sidecars, that result should populate status. Without result metadata, status is `pending` or `unknown` depending on context.

### acceptance to closeout criteria

Current `acceptance` maps to workflow/task closeout criteria.

Acceptance remains human-readable text until richer structured criteria exist.

### dependencies to requires/ordering

Current `dependencies` maps to task/workflow dependencies.

The adapter should preserve direction exactly as existing mana commands interpret it.

### requires/produces to artifacts/evidence hints

Current `requires` remains input requirements.

Current `produces` maps to expected outputs and possible Evidence refs when the produced path exists or appears in an evidence sidecar.

### decisions to Decision

Current mana decisions map directly to DecisionView.

Decision fields:

- question/title
- status open/resolved
- outcome
- rationale
- blocking units

Unresolved blocking decisions should appear as workflow blockers.

### notes to Note

Current notes map to NoteView.

Notes are not automatically Evidence unless they include an artifact ref or are explicitly marked as evidence.

### facts to Evidence or durable Fact

Current facts need conservative handling.

- Stable verified claims may remain facts.
- Agent-generated transient observations should be viewed as Note or Evidence, not high-trust fact.
- Facts with stale TTL should not be promoted to workflow truth without verification.

The adapter should not automatically treat all current facts as trusted workflow evidence.

## Write behavior

The adapter should initially write minimal additive records.

Preferred write targets:

1. Existing unit frontmatter only for small summary fields when preservation is safe.
2. Sidecars under `~/.mana/ledger/` for structured repeated records.
3. Notes for human-readable progress summaries when no structured write path exists.
4. Run artifact directories for bulk trace/evidence/log/diff data.

Do not write:

- raw transcripts
- full tool logs
- huge test output
- unredacted secrets
- low-trust content as global fact

## Compatibility shims

### Workflow from current task

If imp is asked to run mana unit `394.2.1`:

```text
unit 394.2.1 -> TaskView
parent chain 394.2 -> WorkflowView
verify field -> VerificationView
acceptance -> closeout criteria
```

If no parent workflow exists, create an implicit WorkflowView around the task without writing new storage unless meaningful artifacts are produced.

### Evidence from produces

If a unit has:

```yaml
produces:
  - crates/imp-core/src/workflow/contract.rs
```

The adapter treats that as an expected artifact, not proof. Proof requires an Evidence record or verification result.

### Existing close

Current `mana close` remains authoritative for old status updates. imp-next closeout should call compatible update APIs rather than bypass mana status semantics.

## Deprecated/noisy concepts

The adapter should discourage these patterns for imp-next-generated records:

- storing every model claim as a fact
- appending verbose trace output to notes
- using notes as a substitute for verification results
- marking DONE without verification status when a required gate exists

But it should not break old records using these patterns.

## Initial adapter API sketch

```rust
struct ManaLedgerAdapter;

impl ManaLedgerAdapter {
    fn workflow_view(unit_id: &str) -> Result<WorkflowView>;
    fn task_view(unit_id: &str) -> Result<TaskView>;
    fn verification_views(unit_id: &str) -> Result<Vec<VerificationView>>;
    fn evidence_refs(unit_id: &str) -> Result<Vec<EvidenceRef>>;

    fn record_workflow_started(update: WorkflowStartedUpdate) -> Result<()>;
    fn record_verification_result(update: VerificationResultUpdate) -> Result<()>;
    fn record_evidence_ref(update: EvidenceRefUpdate) -> Result<()>;
    fn record_closeout(update: WorkflowCloseoutUpdate) -> Result<()>;
}
```

This API can live in imp-core initially and use current mana tool/CLI/file APIs under the hood.

## Read examples

### Current mana task

```yaml
id: '394.2.1'
kind: task
title: Define workflow contract Rust types in imp-core
parent: '394.2'
acceptance: Rust module for workflow contract types exists...
verify: cd ~/imp && cargo test -p imp-core workflow_contract
produces:
  - crates/imp-core/src/workflow
```

Adapter view:

```yaml
TaskView:
  id: '394.2.1'
  workflow_id: '394.2'
  title: Define workflow contract Rust types in imp-core
  closeout_criteria:
    - Rust module for workflow contract types exists...
  verifications:
    - type: command
      required: true
      command: cd ~/imp && cargo test -p imp-core workflow_contract
  expected_outputs:
    - crates/imp-core/src/workflow
```

### Current mana epic

```yaml
id: '394'
kind: epic
title: Evolve imp into workflow-first agent runtime...
```

Adapter view:

```yaml
WorkflowView:
  id: '394'
  title: Evolve imp into workflow-first agent runtime...
  child_tasks:
    - '394.1'
    - '394.2'
```

## Trust behavior

The adapter should attach provenance to mapped records later:

| Source | Suggested provenance |
|---|---|
| user-created mana unit | durable_mana_record |
| agent-created note | generated_summary |
| verify command result | verifier_output |
| evidence artifact | artifact_ref |
| old fact with TTL | mana_fact_staleable |

Low-trust or stale content should not become a high-trust workflow authorization source.

## Open questions

- Should task-with-children always be a WorkflowView, or only if explicitly marked?
- Should sidecars be addressed by mana unit ID, run ID, or both?
- How should old `fact` records with pass/fail verification map into Evidence?
- How should current `mana verify` results be persisted so VerificationView can show status?

## Implementation order

1. Define view structs and sidecar refs.
2. Implement read-only mapping from current unit frontmatter.
3. Implement write path for evidence refs and verification results as sidecars.
4. Add closeout update path.
5. Add provenance/trust metadata when 394.8 types exist.
