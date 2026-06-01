# Rust parser and validator plan

This is the next implementation slice after the schema definition workflow.

## Goal

Add the smallest Rust implementation that can parse and validate `imp.workflow/v1` workflow files, using the dogfood workflow artifacts as fixtures.

## Likely module location

Inspect existing workflow modules first, then add a focused module such as:

```text
crates/imp-core/src/workflow/schema.rs
crates/imp-core/src/workflow/validate.rs
```

If the current module layout makes that awkward, use the nearest existing workflow contract/runtime module and keep public API churn minimal.

## DTO sketch

```rust
struct WorkflowDocument {
    schema: String,
    id: String,
    title: String,
    status: WorkflowStatus,
    kind: String,
    parent: Option<WorkflowParent>,
    settings: WorkflowSettings,
    spec: WorkflowSpec,
    context: BTreeMap<String, ContextRequirement>,
    steps: BTreeMap<String, WorkflowStep>,
    prototypes: BTreeMap<String, WorkflowPrototype>,
    checks: BTreeMap<String, WorkflowCheck>,
    workers: BTreeMap<String, WorkflowWorker>,
    results: WorkflowResults,
    closeout: WorkflowCloseout,
}
```

Use `BTreeMap` for stable ordering in diagnostics/tests.

## Validation API sketch

```rust
fn load_workflow(path: &Path) -> Result<WorkflowDocument, WorkflowError>;
fn validate_workflow(doc: &WorkflowDocument, root: &Path) -> Result<(), Vec<WorkflowDiagnostic>>;
fn next_runnable_steps(doc: &WorkflowDocument) -> Vec<String>;
```

## First tests

Use fixtures:

```text
.imp/workflows/prototype-imp-workflow-engine/workflow.yaml
.imp/workflows/define-workflow-schema/workflow.yaml
```

Test cases:

- valid dogfood workflows parse and validate;
- missing top-level field fails;
- invalid status fails;
- missing step dependency fails;
- missing check reference fails;
- missing prototype reference fails;
- missing worker reference fails;
- missing acceptance status fails;
- missing closeout required check fails;
- parent/child workflow mismatch fails;
- next runnable steps respects dependencies.

## Non-goals for first implementation

- No agent-loop integration.
- No worker dispatch.
- No worktree creation.
- No event log replay.
- No TUI rendering.
- No replacement of mana/work/prototype yet.

## Verification command

The eventual narrow verification should be something like:

```sh
cargo test -p imp-core workflow_schema
```
