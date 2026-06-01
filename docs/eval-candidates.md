# Eval Candidates

## Goal

Capture useful failures and corrections as durable eval candidates so imp can
learn from regressions later. This phase records candidate artifacts only; it does
not implement an eval runner.

An eval candidate is a structured record that links a run, workflow context,
trace/evidence artifacts, failure mode, expected behavior, verifier commands, and
human notes. It should be safe to review, redact, and later convert into a real
eval case.

## Non-goals

- No automated eval runner in this phase.
- No automatic model grading.
- No upload/export to external services.
- No storing secrets or sensitive raw outputs without redaction.
- No replacing normal evidence packets; candidates reference evidence rather
  than duplicating large artifacts.

## Capture triggers

Create or offer an eval candidate when a run exposes behavior that should become
a future regression test.

Automatic or suggested triggers:

- final status `BLOCKED`
- final status `DONE_WITH_CONCERNS`
- final status `FAILED`
- required verification gate failed
- required verification gate blocked
- required verification gate skipped
- ReferenceMonitor policy denial
- repeated tool-call loop or recovery checkpoint exhaustion
- worktree-auto apply conflict or discarded useful worktree
- evidence packet records unresolved risks
- user gives negative feedback
- user corrects the result
- user manually chooses “save as eval candidate”

Manual trigger:

- CLI/TUI action: save the current run, selected evidence packet, or selected
  correction as an eval candidate.

Triggers should be conservative: missing confidence is a reason to offer capture,
not silently create noisy artifacts.

## Failure classification

Use a small explicit classification first:

- `blocked`
- `done-with-concerns`
- `verification-failed`
- `verification-blocked`
- `verification-skipped-required`
- `policy-denied`
- `tool-loop`
- `tool-error`
- `user-correction`
- `negative-feedback`
- `worktree-apply-conflict`
- `manual`
- `unknown`

A candidate may include multiple labels, but `failure_mode` should identify the
primary reason.

## Current implementation status

The current implementation records closeout candidates as run sidecars after the
run evidence packet is written:

```text
.imp/runs/<run-id>/eval-candidates/<run-id>-closeout/candidate.json
```

These candidates reference sibling run artifacts:

```text
.imp/runs/<run-id>/trace.jsonl
.imp/runs/<run-id>/evidence.md
.imp/runs/<run-id>/workflow-contract.json
.imp/runs/<run-id>/verification/<gate-id>/...
```

The older project-wide layout below remains the intended index/listing location
for manual and promoted candidates:

```text
.imp/eval-candidates/
  <candidate-id>/
    candidate.json
    notes.md              # optional human notes/correction
    redactions.json       # optional redaction metadata
```

Candidate ids should be stable enough for references and unique enough for local
storage, for example:

```text
<UTC timestamp>-<short-run-id>-<slug>
```

Candidate artifacts should reference existing run artifacts by path when
possible:

```text
.imp/runs/<run-id>/evidence.md
.imp/runs/<run-id>/trace.jsonl
.imp/runs/<run-id>/worktree/diff.patch
```

Avoid copying large trace/tool output files unless a later export command needs a
self-contained bundle.

## EvalCandidate schema

Initial JSON shape:

```json
{
  "schema_version": 1,
  "id": "2026-05-14-run-abc-verification-failed",
  "created_at": "2026-05-14T20:00:00Z",
  "source": {
    "run_id": "run-abc",
    "workflow_id": "394.12",
    "session_id": null,
    "parent_candidate_id": null
  },
  "trigger": "verification-failed",
  "failure_mode": "verification-failed",
  "labels": ["eval-candidate", "verification"],
  "prompt": "Fix the failing parser test",
  "task": {
    "title": "Fix parser regression",
    "description": "...",
    "acceptance": "cargo test -p imp-core parser"
  },
  "workflow_contract_ref": {
    "kind": "mana-unit",
    "id": "394.12.3",
    "path": ".mana"
  },
  "expected_behavior": {
    "summary": "Parser handles empty input without panic",
    "assertions": ["cargo test -p imp-core parser_empty_input passes"]
  },
  "actual_behavior": {
    "summary": "Verification failed with parser_empty_input panic",
    "error_excerpt": "thread panicked at ..."
  },
  "verifiers": [
    {
      "name": "parser tests",
      "command": "cargo test -p imp-core parser_empty_input",
      "required": true,
      "last_status": "failed",
      "output_ref": ".imp/runs/run-abc/verification/parser-tests.txt"
    }
  ],
  "artifact_refs": [
    { "kind": "evidence", "path": ".imp/runs/run-abc/evidence.md" },
    { "kind": "trace", "path": ".imp/runs/run-abc/trace.jsonl" },
    { "kind": "worktree-patch", "path": ".imp/runs/run-abc/worktree/diff.patch" }
  ],
  "policy_refs": [
    {
      "tool_name": "bash",
      "decision": "deny",
      "reason_code": "extension_network_denied"
    }
  ],
  "privacy": {
    "redaction_status": "redacted",
    "redaction_rules": ["secrets", "absolute-home-paths"],
    "contains_sensitive_data": false
  },
  "human_notes_ref": ".imp/eval-candidates/.../notes.md",
  "correction": {
    "kind": "patch",
    "summary": "Human changed parser empty-input branch",
    "artifact_ref": null
  }
}
```

## Core fields

Required:

- `schema_version`
- `id`
- `created_at`
- `source.run_id` when available
- `trigger`
- `failure_mode`
- `prompt` or `task`
- `expected_behavior.summary`
- `artifact_refs`
- `privacy.redaction_status`

Optional but recommended:

- workflow id / mana unit id
- workflow contract reference
- verification metadata
- trace/evidence refs
- policy refs
- actual behavior summary
- human notes
- correction artifact
- worktree patch/diff refs

## Artifact references

Use typed references instead of embedding large content:

- `evidence`
- `trace`
- `tool-output`
- `verification-output`
- `worktree-status`
- `worktree-diff-stat`
- `worktree-patch`
- `metadata`
- `human-notes`
- `correction-patch`

Each ref should include:

- `kind`
- `path`
- optional `summary`
- optional `sha256` when cheap

## Verification metadata

Eval candidates should preserve verifier context:

- verifier name/label
- command
- required vs optional
- last status
- exit code
- output artifact path
- failure excerpt

A failed required verifier is one of the strongest capture triggers.

## Policy metadata

Policy-denied candidates should preserve reviewable policy context:

- tool name
- action kind
- extension id/version if any
- decision kind
- reason code/message
- autonomy mode
- resource scopes
- trust labels

Do not store secret values. Secret names may be stored if they are already part
of a manifest/policy declaration and are not sensitive in the project context.

## Privacy and redaction

Before writing or exporting a candidate:

- redact known secret values
- avoid embedding full tool output by default
- prefer artifact refs over copied content
- trim long error excerpts
- mark whether sensitive data may remain
- record applied redaction rules

Suggested redaction status values:

- `unreviewed`
- `redacted`
- `contains-sensitive-data`
- `safe-to-export`

Project-local storage can keep unreviewed candidates, but export should require
`redacted` or `safe-to-export`.

## Trust metadata

Candidates may include a compact trust/provenance summary:

```json
{
  "trust": {
    "sources": ["source=ToolResult; trust=ExternalUntrusted; origin=https://example.test"],
    "low_trust_influences": ["low-trust source observed: https://example.test"],
    "warnings": ["external/untrusted content cannot authorize policy or tool escalation"]
  }
}
```

This summary is for review and filtering. It should help a future eval runner or
export command distinguish failures caused by trusted project context from
failures influenced by untrusted tool output.

## User correction flow

TUI/CLI should support saving a corrected run as an eval candidate:

1. User sees a wrong or incomplete result.
2. User provides correction or notes.
3. imp writes/updates `notes.md` and links correction artifacts.
4. Candidate records original prompt, actual behavior, expected behavior, and
   correction summary.

A correction can be:

- freeform notes
- a patch/diff
- a command/verifier that would catch the issue
- a better expected answer

## Automatic closeout examples

### `BLOCKED`

If a run cannot proceed because execution is blocked, imp records a candidate
with `failure_mode: "blocked"` and an `actual_behavior.summary` such as:

```json
{
  "trigger": "blocked",
  "failure_mode": "blocked",
  "actual_behavior": {
    "summary": "Run blocked: dependency unavailable",
    "error_excerpt": "dependency unavailable"
  },
  "artifact_refs": [
    { "kind": "trace", "path": ".imp/runs/run_123/trace.jsonl" },
    { "kind": "evidence", "path": ".imp/runs/run_123/evidence.md" }
  ]
}
```

A repeated-action blocker is classified as `tool-loop` so future regression tests
can target autonomy loops separately from ordinary blocked work.

### `DONE_WITH_CONCERNS`

If a run finishes but cannot honestly report clean completion, imp records a
candidate with `failure_mode: "done-with-concerns"`. The candidate preserves the
concerns as the actual behavior excerpt, while expected behavior remains the
workflow contract or human-provided expected result.

```json
{
  "failure_mode": "done-with-concerns",
  "expected_behavior": {
    "summary": "Agent run should satisfy its workflow contract and required verification gates"
  },
  "actual_behavior": {
    "summary": "Run completed with concerns: verification was not available",
    "error_excerpt": "verification was not available"
  }
}
```

### Failed verification gate

Required verification gates take precedence over a nominal `DONE` status. A
failed command gate records verifier metadata separately from natural-language
expected behavior:

```json
{
  "failure_mode": "verification-failed",
  "expected_behavior": {
    "assertions": ["cargo test -p imp-core parser_empty_input passes"]
  },
  "verifiers": [
    {
      "name": "parser tests",
      "command": "cargo test -p imp-core parser_empty_input",
      "required": true,
      "last_status": "failed",
      "exit_code": 101,
      "output_ref": ".imp/runs/run_123/verification/parser-tests.txt",
      "failure_excerpt": "parser_empty_input failed"
    }
  ]
}
```

## User-corrected run example

When the user corrects a result, the candidate should describe both what imp did
and what should happen next time. A correction can be notes, a patch, a verifier
command, or a better expected answer.

```json
{
  "trigger": "user-correction",
  "failure_mode": "user-correction",
  "prompt": "Update the parser to handle empty input",
  "expected_behavior": {
    "summary": "Parser returns an empty AST for empty input instead of panicking",
    "assertions": ["cargo test -p parser empty_input passes"]
  },
  "actual_behavior": {
    "summary": "imp changed whitespace handling but missed empty input"
  },
  "human_notes_ref": ".imp/eval-candidates/2026-05-14-run_123-empty-input/notes.md",
  "correction": {
    "kind": "patch",
    "summary": "Human added an explicit empty-input branch",
    "artifact_ref": {
      "kind": "correction-patch",
      "path": ".imp/eval-candidates/2026-05-14-run_123-empty-input/correction.patch"
    }
  }
}
```

Manual save flows may also attach an optional verifier command. That command is
stored under `verifiers[]` and mirrored as an expected assertion, but it is not
run by the eval-candidate capture path.

## Closeout integration

At workflow closeout:

- `DONE`: no eval candidate by default
- `DONE_WITH_CONCERNS`: offer or auto-record candidate depending on policy
- `BLOCKED`: offer or auto-record candidate
- `NEEDS_CONTEXT`: offer candidate if missing context revealed product gap
- `FAILED`: auto-record candidate when trace/evidence exists

Verification and policy triggers can create candidates even if final status is
not failed, especially for recovered failures.

## CLI/TUI surfacing

Suggested CLI commands:

```sh
imp eval save --run <run-id> --reason verification-failed
imp eval list
imp eval show <candidate-id>
```

Suggested TUI actions:

- “Save as eval candidate” on final answer
- “Save correction as eval” on user correction
- “Save failed verification as eval” on verification panel
- “Save policy denial as eval” on policy warning

The UI should show candidate id and path after saving.

## Future eval runner boundary

This phase captures candidate artifacts only. A later eval runner can:

- load candidates
- generate test fixtures
- replay prompts/tools under controlled conditions
- run verifier commands
- compare expected vs actual behavior
- report regression status

Keeping capture separate from execution makes the learning loop useful before the
runner exists.
