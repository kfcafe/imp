# Workflow evidence contract

Implementation workflows need checks that prove both that work changed and that the result verifies.

## Evidence classes

- Implementation evidence: proves the requested implementation was actually attempted or completed.
  - changed files
  - created/updated artifacts
  - required text added or removed
  - targeted regression test added or updated
  - generated migration/config/schema file exists
- Verification evidence: proves the current tree still passes relevant checks.
  - tests
  - builds
  - linters
  - broad validation commands

## Guardrail

For `kind: implementation` workflows, a build step must not be auto-completed by broad command checks alone. At least one check attached to the step should be change-sensitive implementation evidence.

Broad commands like `cargo test -p imp-core --lib` are useful for verify steps, but they do not prove an implementation step changed anything.

## Initial check kinds to support

- `changed_files`: one or more paths/globs expected to be modified relative to the workflow baseline.
- `presence`: a file contains a required string/pattern.
- `absence`: a file does not contain a forbidden string/pattern.

## Dogfood target

`simplify-agent-runtime-policy` should require evidence such as:

- text classifier symbols removed or disabled
- generated prompt operating-rule strings removed
- targeted tests added/updated
- broad tests passing only at verify/closeout
