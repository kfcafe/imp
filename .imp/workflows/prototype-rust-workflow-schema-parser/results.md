# Prototype Rust workflow schema parser results

## Summary

Implemented the first Rust parser/validator prototype for `imp.workflow/v1` compact YAML workflow artifacts.

## Code changes made

- Added `crates/imp-core/src/workflow/schema.rs`.
- Exported the schema module from `crates/imp-core/src/workflow/mod.rs`.
- Moved `serde_yaml` to root workspace dependencies.
- Added `serde_yaml.workspace = true` to `imp-core`.
- Removed the unused direct `serde_yaml = "0.9"` dependency from `imp-cli`.
- Gated two existing builder tests that use `mana_core` behind `#[cfg(feature = "mana-api")]`, matching the existing optional mana dependency.

## Parser/validator behavior implemented

- Loads workflow YAML into typed Rust DTOs.
- Uses map-based IDs for steps, checks, workers, prototypes, and acceptance.
- Denies unknown fields in core schema structs.
- Validates schema version.
- Validates workflow id matches directory name in strict mode.
- Validates step dependencies.
- Validates step check references.
- Validates prototype references.
- Validates worker references.
- Validates workflow-call child files exist in strict mode.
- Validates parent/child workflow links in strict mode.
- Validates check `requires` references.
- Validates acceptance entries and linked checks.
- Validates closeout required checks and built-in predicates.
- Validates passed artifact checks resolve against project root.
- Detects cycles in step and check dependency graphs.
- Provides `next_runnable_steps` as a conservative pure helper.

## Tests added

- Dogfood workflows parse and validate.
- Missing step/check/prototype/worker/closeout references are rejected.
- Invalid status and malformed acceptance fail parsing.
- Next runnable steps respects dependencies.
- Strict parent/child workflow mismatch is rejected.

## Verification command

```sh
cargo test -p imp-core workflow_schema
```

Result:

```text
5 passed; 0 failed; 0 ignored; 0 measured; 882 filtered out
```

## Limitations and next steps

- No agent-loop integration yet.
- No native `workflow` tool yet.
- No workflow event-log replay yet.
- No TUI rendering yet.
- No Rust writeback/formatting of `workflow.yaml` yet.
- Source line/column diagnostics are deferred; diagnostics currently use object paths.
- `.imp/workflows/**` remains ignored by git, so dogfood artifacts are local-only unless tracking policy changes.
