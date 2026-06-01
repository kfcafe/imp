# Define workflow schema results

## Summary

The schema-definition workflow refined the v1 workflow artifact around concise map-based YAML and produced the reference materials needed for the first Rust parser/validator prototype.

## Chosen concise YAML schema shape

Use compact YAML as the canonical workflow format:

- map-based IDs for `steps`, `checks`, `workers`, `prototypes`, and `spec.acceptance`;
- `steps` as the core executable unit;
- `checks` as the requirement/verification primitive;
- top-level `prototypes` for durable uncertainty and experiment tracking;
- `kind: workflow` steps for workflow calls;
- sibling directories for called workflows;
- acceptance criteria as objects with `text`, `status`, and optional linked `checks`.

The reference is written in:

```text
.imp/workflows/define-workflow-schema/artifacts/schema-reference.md
```

## Status vocabulary

Defined status sets for:

- workflows;
- steps;
- checks;
- prototypes;
- acceptance criteria.

Important distinction:

- steps use `done` for completed work;
- checks use `passed` for satisfied conditions;
- acceptance criteria use `done` as the summary state, backed by linked checks.

The reference is written in:

```text
.imp/workflows/define-workflow-schema/artifacts/statuses.md
```

## Validation rules

Defined first validator rules for:

- required top-level fields;
- map-based collection shape;
- step dependency references;
- check references;
- prototype references;
- worker references;
- acceptance criteria shape;
- closeout requirements;
- parent/child workflow calls;
- allowed status values;
- mutation/approval constraints.

The reference is written in:

```text
.imp/workflows/define-workflow-schema/artifacts/validation-rules.md
```

## Rust parser/validator implementation plan

The first Rust slice should parse and validate the two dogfood workflow fixtures:

```text
.imp/workflows/prototype-imp-workflow-engine/workflow.yaml
.imp/workflows/define-workflow-schema/workflow.yaml
```

It should add typed DTOs, validation diagnostics, and a `next_runnable_steps` helper without integrating into the agent loop yet.

The plan is written in:

```text
.imp/workflows/define-workflow-schema/artifacts/rust-parser-plan.md
```

## Verification performed

- Reviewed changed workflow artifacts with `git diff --no-index` because `.imp/` is ignored.
- Parsed both workflow YAML files with Ruby YAML.
- Verified required top-level fields.
- Verified map-based collections.
- Verified step dependencies, check references, prototype references, and worker references.
- Verified parent/child workflow linkage.
- Verified acceptance entries have `text` and `status`.
- Verified passed artifact checks point at existing files.

## Open questions

- Should selected `.imp/workflows/**/workflow.yaml` and `results.md` be unignored now or after the Rust parser prototype?
- Which existing Rust workflow structs should be adapted versus replaced?
- Should the first Rust parser live inside `crates/imp-core/src/workflow/` or in a new schema-focused submodule?
- How strict should draft-mode validation be for workflow calls whose child workflow does not exist yet?
