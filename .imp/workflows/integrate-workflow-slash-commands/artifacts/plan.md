# Integrate workflow slash commands plan

## Goal

Repurpose `/plan`, `/status`, and `/run` around native imp workflows instead of the older workflow-profile/mana-run semantics.

## V1 scope

Keep this integration thin and safe:

- `/status` should include native workflow status from `.imp/workflows`.
- `/run [id]` should call the native workflow engine conceptually by showing the next workflow action for the selected/id workflow.
- `/plan <goal>` should become workflow-backed at the UX level, but can initially enqueue a prompt that instructs the agent to use the native `workflow` tool to create/update a workflow. Full automatic `workflow.plan` artifact creation is deferred until the tool supports plan/update semantics deeply enough.
- Keep `/workflow` for lower-level inspection for now.
- Remove `/workflows` as a primary user surface if practical, or make it redirect to `/workflow list` semantics.

## Implementation observations

- `crates/imp-tui/src/app.rs` currently routes `/plan` through workflow profiles via `try_workflow_command`.
- `/status` currently calls `show_status_command` and renders mana/improve/runtime status.
- `/run` currently sets the active mana run under `mana-ui` builds.
- Native workflow tool lives in `imp-core`, but TUI slash commands can call parser/rendering helpers directly or enqueue prompts to use the tool.

## Implementation plan

1. Add a small TUI helper that discovers `.imp/workflows/*/workflow.yaml` and renders a concise workflow status section.
2. Include that section in `/status` output.
3. Repurpose `/run [id]` to show the next native workflow action using the same selection logic as `workflow run` where practical.
4. Repurpose `/plan <goal>` to enqueue a workflow-backed planning prompt that tells the agent to use the native workflow tool/schema; full artifact auto-generation comes later.
5. Update command palette/help text from mana-run/profile language to native workflow language.
6. Update tests that asserted old `/plan` workflow-profile wrapping or `/workflows` listing.

## Verification

```sh
cargo test -p imp-tui workflow
cargo test -p imp-core workflow_
```
