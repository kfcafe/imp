# Workflow parser considerations

## Review worker artifacts

Review workers should be read-only for production code, but they should still be able to write disposable workflow artifacts such as:

```text
.imp/workflows/<workflow-id>/artifacts/reviews/<review-id>.md
.imp/workflows/<workflow-id>/artifacts/reviews/<review-id>.json
```

This keeps reviewer output durable enough for workflow closeout without letting reviewer workers mutate production code/tests. The schema can represent this through worker write capabilities:

```yaml
workers:
  reviewer:
    role: reviewer
    writes: [review_artifacts]
```

## serde_yaml dependency placement

Current evidence:

- `crates/imp-cli/Cargo.toml` declares `serde_yaml = "0.9"`.
- `rg serde_yaml crates/imp-cli crates/imp-core` found no Rust source usage.
- `cargo tree -p imp-cli -i serde_yaml --depth 2` shows `serde_yaml` is currently only a direct `imp-cli` dependency.
- `imp-core` does not currently depend on `serde_yaml`.

Recommendation for the Rust parser prototype:

1. Move `serde_yaml` to `[workspace.dependencies]` in the root `Cargo.toml`.
2. Add `serde_yaml.workspace = true` to `imp-core` when the parser lands.
3. Remove `serde_yaml` from `imp-cli` unless a real CLI usage appears.

This gives one workspace-managed dependency version while avoiding duplicate direct dependencies. The crate will still be compiled wherever needed, but version declaration and ownership are centralized.

## Parser features to consider before implementation

### Source spans / diagnostics

Serde YAML will parse shape, but it does not give great source spans. For v1, diagnostics can report workflow id and object path:

```text
steps.build_runtime.checks[0]: unknown check `schema_selected`
```

Source line/column diagnostics can be deferred unless the UX feels too poor.

### Draft vs strict validation

We need two validation modes:

- `draft`: allows referenced child workflows/artifacts to be missing when still planning.
- `strict`: requires workflow-call targets and passed artifact checks to exist.

This prevents early workflow planning from being too brittle while preserving hard closeout enforcement.

### Unknown fields

Recommendation: deny unknown fields in core objects for v1 where practical, or at least collect warnings. Silent typos in workflow files are dangerous because they can disable enforcement by accident.

### Stable IDs and directory matching

Validate that workflow `id` matches the directory name by default. Allow override only with a warning or explicit setting if we find a real need.

### Built-in predicates

Closeout requirements may reference built-in predicates as well as checks:

```yaml
closeout:
  done:
    requires:
      - required_checks_passed
      - no_unapproved_goal_or_acceptance_changes
```

The parser should distinguish built-in predicates from missing check refs.

### Cycles

Validate acyclic graphs for:

- step `depends_on`
- check `requires`
- workflow calls, eventually

Workflow-call cycle validation can be deferred to a project/workflow-index layer if v1 only validates a single workflow plus direct parent/child link.

### Runnable step semantics

`next_runnable_steps` should initially be pure and conservative:

- step status is `todo` or `ready`;
- all `depends_on` steps are terminal-success (`done` or `done_with_concerns` depending on strictness);
- step checks that represent prerequisites are passed;
- worker exists if declared.

We may later need to separate prerequisite checks from completion checks because a build step can have checks that are only expected after it runs.

### Check timing

The current schema uses `steps.*.checks` for both prerequisites and completion proof. This is simple but ambiguous.

Potential future refinement:

```yaml
steps:
  build_runtime:
    requires: [context_complete]
    proves: [schema_module_added]
```

For v1, keep `checks` but parser/validator should not over-assume every step check must pass before the step is runnable.

### Review artifacts

Support review outputs as artifact checks:

```yaml
checks:
  reviewer_report_written:
    kind: artifact
    path: .imp/workflows/<id>/artifacts/reviews/reviewer.md
```

This aligns with reviewer workers writing disposable review documents while staying code-readonly.

### Path safety

Artifact/check paths should be normalized and constrained to the project/workflow root where appropriate. Avoid accepting paths that escape unexpectedly via `..` unless explicitly allowed.

### Schema versioning

Use `schema: imp.workflow/v1` now. Parser should reject unsupported schema versions with a clear diagnostic.

### Serialization round-trip

For v1, parsing/validation matters more than preserving comments/order. Avoid writing back workflows from Rust until we decide formatting behavior. The workflow tool can later own canonical formatting.
