# Integrate workflow slash commands results

## Summary

Repurposed the TUI slash-command surfaces around native imp workflow artifacts.

## User-facing behavior

- `/plan <goal>` now enqueues a workflow-backed planning prompt instructing the agent to use the native `workflow` tool and `.imp/workflows` schema as source of truth.
- `/status` now includes a concise workflow summary from `.imp/workflows`.
- `/run [id]` now shows the next native workflow action for the selected workflow or id.
- `/workflow [list|show|validate|run] [id]` provides lower-level workflow inspection.
- `/workflows` redirects to workflow list.

## Code changes made

- Added TUI workflow YAML helpers in `crates/imp-tui/src/app.rs`.
- Added `workflow_summary` to status snapshots.
- Added `serde_yaml.workspace = true` to `imp-tui`.
- Reserved `/plan`, `/status`, `/run`, `/workflow`, and `/workflows` away from old workflow-profile routing.
- Updated `/run` away from mana-run activation and toward native workflow run display.
- Removed unused `show_workflow_profiles` helper.

## Verification performed

```sh
cargo check -p imp-tui
cargo test -p imp-core workflow_
```

Results:

```text
cargo check -p imp-tui: finished successfully
cargo test -p imp-core workflow_: 49 passed; 0 failed
```

## Limitations

- `/plan` does not directly generate `workflow.yaml` yet; it prompts the agent to use the workflow tool/schema.
- TUI slash helpers parse workflow YAML directly rather than invoking the core workflow tool runtime.
- `/run` is advisory and does not dispatch workers or execute commands yet.
- Full old workflow-profile migration is deferred to the legacy consolidation workflow.

## Next workflow

Continue with:

```text
.imp/workflows/workflow-worker-orchestration/workflow.yaml
```

Goal: add workflow-backed worker orchestration for builder, verifier, and reviewer assignments.
