# Prototype imp workflow engine results

## Summary

This workflow captured the current schema-first direction for imp-native workflows and produced the first dogfood `workflow.yaml` artifact.

## Schema decisions made

- Workflow instances are canonical project artifacts at `.imp/workflows/<slug>/workflow.yaml`.
- Runtime/generated artifacts live beside the canonical file: `events.jsonl`, `trace.jsonl`, `state.json`, `results.md`, and `artifacts/`.
- Workflow instances use compact YAML with map-based IDs.
- Global/project configuration and reusable profiles are TOML.
- Project profiles live under `.imp/workflows/profiles/`.
- A model-facing native `workflow` tool should be the primary programmatic interface.
- `/plan`, `/status`, and `/run` should be thin user-facing wrappers over the workflow tool.
- The workflow tool can plausibly replace normal imp use of `mana`, `work`, and `prototype` once it reaches parity.
- Multiple workflows may exist in a project; one workflow is active per session unless selected otherwise.
- Workflows support dependencies between steps, checks, worker runs, prototype experiments, and called workflows.
- Workflows can call other workflows as normal `kind: workflow` steps; child workflows should live as sibling directories under `.imp/workflows/`.
- Acceptance criteria are map entries with their own verification status and optional linked checks.
- `checks` replaces `gates` as the primary requirement/verification term.
- `steps` replaces `phases` as the core executable unit.
- Parent agent coordinates and reviews, and may edit non-code artifacts. Builder workers write production code and tests.
- Worker definitions are reusable objects such as `builder`, `verifier`, and `reviewer`.
- Prototyping is a workflow feature, not a role: workflows can test approaches, discard prototypes, and record selected approach/rationale.
- Final answers should be constrained by workflow state: workflow validates factual claims and renders a factual skeleton; the agent adds concise prose.

## Verification performed

- Reviewed `docs/design/imp-native-workflow-engine.md` with `git diff --no-index` because it is an untracked new file.
- Reviewed `.imp/workflows/prototype-imp-workflow-engine/workflow.yaml` and this `results.md` with `git diff --no-index` because `.imp/` is currently ignored.
- Parsed `.imp/workflows/prototype-imp-workflow-engine/workflow.yaml` with Ruby YAML.
- Verified required top-level workflow keys: `schema`, `id`, `title`, `status`, `kind`, `settings`, `spec`, `context`, `steps`, `prototypes`, `checks`, `workers`, `results`, `closeout`.
- Verified `steps`, `checks`, `workers`, and `prototypes` are maps.
- Verified all step dependency, check, prototype, and worker references resolve.
- Verified all check `requires` references resolve.
- Verified each acceptance criterion has `text` and `status`.
- Verified the design doc includes expected terms covering the current decisions: `workflow tool`, `checks`, `steps`, `workflow call`, `map-based`, `.imp/workflows/profiles`, and `Git tracking policy`.

## Open questions remaining

- What is the first minimal Rust schema validation implementation?
- Which existing `workflow` structs can be reused versus renamed or retired?
- How much of `mana`, `work`, and `prototype` should be retired once workflow-tool parity exists?
- What exact setting controls branch/worktree creation for prototype and build steps?
- Should `/workflow` exist as a public advanced command or remain hidden/debug-only?
- Should `.gitignore` be changed to allow selected `.imp/workflows/**/workflow.yaml` and `results.md` files to be tracked while keeping traces/cache ignored?

## Recommended next workflow

Create a workflow specifically for schema refinement and Rust validation:

```text
.imp/workflows/define-workflow-schema/workflow.yaml
```

Acceptance for that workflow:

- formalize the v1 workflow schema enough for Rust parsing;
- define required/optional fields and status enums;
- validate duplicate ids, missing dependencies, missing check references, and required closeout checks;
- add Rust DTO/parser tests against the dogfood workflow artifact;
- preserve the parent-agent/worker-agent split in the schema;
- refine concise map-based YAML before committing to Rust shape.
