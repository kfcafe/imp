# mana-next Storage and Artifact Reference Strategy

Status: design decision for imp-next  
Parent: mana `394.3` / child `394.3.2`

## Decision

Use a **hybrid file-first strategy**:

1. Keep existing mana units as markdown files under `~/.mana` as the source of truth for workflow/task/decision/note compatibility.
2. Add workflow-ledger metadata as compatible frontmatter fields or sidecar records, but do not require migration before value is delivered.
3. Store bulky run artifacts in imp run directories, not mana files.
4. Treat SQLite/indexing as a derived cache or later optimization, not the v1 authoritative store.

This preserves current mana behavior while giving imp-next stable references for evidence, verification, workflow contracts, and child runs.

## Current constraints

Current mana uses file-backed markdown units such as:

```text
~/.mana/394.3.1-specify-mana-next-workflow-ledger-schema-and-compa.md
~/.mana/394.7.8-accept-user-and-mana-provided-verification-gates.md
```

These files contain YAML frontmatter and markdown body. Existing commands expect this format for list/show/create/update/verify/close/notes/decisions/dependencies.

Therefore v1 mana-next must not require:

- rewriting all unit files
- moving units out of `~/.mana`
- replacing markdown with SQLite as source of truth
- deleting current `epic`, `task`, or `fact` kinds

## Storage layers

### Layer 1: mana unit markdown

Authoritative for:

- unit identity
- title
- status
- parent/dependencies
- acceptance
- verify command
- labels
- notes/decisions in current mana format
- compatibility with existing commands

Use for workflow-ledger summaries:

```yaml
workflow:
  workflow_type: code_change
  risk_level: medium
  autonomy_mode: safe
  contract_ref:
    run_id: run_abc
    path: .imp/runs/run_abc/workflow-contract.json
  verification_refs:
    - verify_1
  evidence_refs:
    - evidence_1
  closeout_status: DONE
```

This can be added as optional frontmatter later. Old mana ignores unknown fields.

### Layer 2: sidecar workflow ledger records

Use sidecars when frontmatter would become too large or when multiple child records belong to one unit.

Suggested directory:

```text
~/.mana/ledger/
  workflows/
    394.3.json
  verifications/
    verify_1.json
  evidence/
    evidence_1.json
  child-runs/
    child_1.json
```

Rationale:

- keeps markdown readable
- avoids bloating frontmatter with repeated gate/evidence records
- allows structured records without changing old file format
- can be indexed later

Sidecars should be additive and rebuildable when possible from markdown + run artifacts.

### Layer 3: imp run artifacts

Bulk artifacts live with imp runs.

For repo-scoped workflows:

```text
<repo>/.imp/runs/<run-id>/
  workflow-contract.json
  trace.jsonl
  evidence.md
  diff.patch
  verify.log
  policy.jsonl
```

For non-repo/global workflows:

```text
~/.local/share/imp/runs/<run-id>/
  workflow-contract.json
  trace.jsonl
  evidence.md
  verify.log
  policy.jsonl
```

Exact user data dir can follow the existing imp storage conventions; this document only fixes the separation of responsibilities.

### Layer 4: derived index / SQLite later

SQLite may be useful for fast queries:

- list workflows by status
- find failed verification gates
- search evidence metadata
- track child run status
- dashboard/TUI summaries

But v1 should treat SQLite as derived/cache only. The authoritative records remain markdown + sidecars + artifacts.

## Artifact reference format

Use stable refs with enough metadata to validate and render them.

```yaml
artifact_ref:
  id: evidence_1
  run_id: run_2026_05_12_abc
  kind: evidence_packet
  path: .imp/runs/run_2026_05_12_abc/evidence.md
  media_type: text/markdown
  sha256: optional
  bytes: optional
  created_at: '2026-05-12T04:00:00Z'
```

Path rules:

- Prefer repo-relative paths for repo-scoped runs.
- Use absolute or user-data-relative paths for global runs.
- Do not inline large outputs in mana records.
- Store optional hash/size when cheap.
- Missing artifacts should degrade gracefully: show broken ref, do not corrupt unit.

## Workflow record placement

For a workflow backed by an existing mana unit:

```text
~/.mana/394.3-streamline-mana-into-workflow-and-evidence-ledger.md
```

The markdown unit remains the workflow record. Extra structured fields may be written as frontmatter or sidecar.

For a workflow not explicitly backed by mana:

- trivial TUI sessions may remain artifact-only
- meaningful runs can create a lightweight workflow unit or sidecar depending on user/config policy

Default recommendation:

| Run type | mana record? | artifacts? |
|---|---:|---:|
| trivial chat | no | optional session log only |
| meaningful code change | yes, if workflow-first enabled | yes |
| mana task run | yes, existing unit | yes |
| CI/headless run | yes or artifact-only by config | yes |
| child workflow | yes, child task/workflow sidecar | yes |

## Verification record placement

A verification gate should be represented by:

1. summary in workflow/task frontmatter or sidecar
2. detailed logs in run artifacts

Example sidecar:

```json
{
  "kind": "verification",
  "id": "verify_1",
  "workflow_id": "394.2",
  "task_id": "394.2.1",
  "required": true,
  "status": "passed",
  "command": "cargo test -p imp-core workflow_contract --lib",
  "artifact_refs": ["evidence_verify_1"]
}
```

The command may also remain in the current mana `verify` field for compatibility.

## Evidence record placement

Evidence records should be structured sidecars or frontmatter summaries pointing to artifacts.

Do not write large evidence content into `.mana/*.md` frontmatter.

Good:

```yaml
evidence_refs:
  - id: evidence_1
    kind: evidence_packet
    path: .imp/runs/run_abc/evidence.md
```

Bad:

```yaml
evidence: |
  thousands of lines of test output...
```

## Compatibility behavior

Existing commands should behave as they do today:

- `mana list`
- `mana show`
- `mana create`
- `mana update`
- `mana verify`
- `mana close`
- `mana notes_append`
- `mana decision_*`
- `mana dep_*`

Unknown future frontmatter fields must be preserved where possible. If current tooling rewrites frontmatter destructively, sidecars should be preferred until preservation is guaranteed.

## Migration strategy

Phase 1: document + examples

- Add schema docs.
- Add examples/templates.
- No migration.

Phase 2: adapter types

- imp can read current unit as Workflow/Task view.
- imp can write artifact refs to sidecars or notes without breaking old commands.

Phase 3: additive metadata

- Add optional workflow/evidence frontmatter fields only when current mana preserves unknown fields safely.
- Keep sidecars as fallback.

Phase 4: derived index

- Optional SQLite or index for fast dashboard/TUI queries.
- Rebuildable from markdown + sidecars + artifacts.

## Rollback strategy

Because v1 is additive:

- Old mana can ignore `~/.mana/ledger` sidecars.
- Removing sidecars should not destroy old task/epic status.
- Run artifacts can remain as historical evidence even if mana-next is disabled.
- If frontmatter fields cause issues, stop writing them and keep sidecars only.

## Tradeoffs

### Why not SQLite as source of truth now?

Pros:

- faster queries
- easier relational model
- better dashboard support

Cons:

- migration risk
- breaks the current inspectable-file model
- harder to hand-edit/recover
- premature before schema settles

Decision: defer SQLite to a derived index.

### Why not only frontmatter?

Pros:

- one file per unit
- easy to inspect

Cons:

- frontmatter bloat
- repeated verification/evidence records become awkward
- old tools may rewrite unknown fields

Decision: frontmatter for summaries, sidecars for structured repeated records.

### Why not only sidecars?

Pros:

- avoids touching old files
- structured records are clean

Cons:

- split-brain risk
- harder to inspect from `mana show`
- old commands cannot see workflow status summaries

Decision: hybrid; sidecars for detail, markdown/frontmatter for summary when safe.

## Chosen initial layout

```text
~/.mana/
  394.3-streamline-mana-into-workflow-and-evidence-ledger.md
  ledger/
    workflows/
    verifications/
    evidence/
    child-runs/

<repo>/.imp/runs/<run-id>/
  workflow-contract.json
  trace.jsonl
  evidence.md
  diff.patch
  verify.log
  policy.jsonl
```

The exact sidecar schema will be implemented in `394.3.4`; this task only chooses the strategy.
