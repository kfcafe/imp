# Runtime, RPC, and ACP Surface Audit

Status: runtime/API classification artifact for `tighten-imp-product-surface`.

## Target boundary

Runtime infrastructure should serve:

- TUI
- one-shot/headless execution
- workflows and future workflow runner/subagents
- optional low-cost RPC

ACP/editor adapters are future/optional and should not shape launch cleanup.

## Evidence inspected

Files/docs:

- `crates/imp-core/src/runtime.rs`
- `crates/imp-core/src/agent/events.rs`
- `crates/imp-core/src/agent/run_loop.rs`
- `crates/imp-core/src/run_evidence.rs`
- `crates/imp-core/src/storage.rs`
- `crates/imp-core/src/imp_session.rs`
- `crates/imp-core/src/sdk.rs`
- `crates/imp-core/src/workflow/child_workflow.rs`
- `crates/imp-cli/src/lib.rs` RPC mode, JSON event conversion, headless paths
- `crates/imp-tui/src/app.rs` runtime accumulator/snapshot/event handling
- `docs/runtime-event-state-api.md`
- `docs/rpc.md`
- README RPC/ACP references

## Current runtime shape

Implemented runtime contract includes:

- `RuntimeEvent`
- `RuntimeStateSnapshot`
- `RuntimeStateAccumulator`
- `AgentEvent` compatibility adapter

Current snapshot includes:

- workflow summary
- autonomy mode
- workspace/worktree state
- phase
- active/completed tools
- approvals
- policy decisions
- verification gates
- evidence refs
- final status
- mana refs
- warnings/errors
- status items

Current event/state is used by:

- TUI runtime state accumulator in `imp-tui/src/app.rs`
- CLI/RPC JSON conversion in `imp-cli/src/lib.rs`
- trace/evidence/run artifact paths
- future GUI/adapter docs

## Classification matrix

| Surface | Classification | Rationale | Next action |
|---|---:|---|---|
| `AgentEvent` | keep/internal | Core stream emitted by agent loop and consumed by TUI/CLI/session. | Keep, but avoid making it launch-facing product API. |
| `RuntimeEvent` | keep/improve | Useful adapter for TUI/RPC/future workflow runner. | Keep as internal/advanced contract. |
| `RuntimeStateSnapshot` | keep/improve | Useful shared frontend/RPC state. | Keep; remove/rename mana refs as workflow-native later. |
| `RuntimeStateAccumulator` | keep | Deterministic reducer useful for TUI/RPC/tests. | Keep. |
| RPC JSONL mode | keep/internal | Useful for embedding/automation; user says nice to include but not necessary. | Keep if low-cost; docs as advanced/internal. |
| ACP/editor adapters | archive/future | Planned, not current launch product. | Move to roadmap/future docs. |
| SDK docs/examples | internal/decide | Could be useful product-adjacent API but not main launch surface. | Keep if low-cost; avoid expanding launch docs. |
| Run evidence/trace | keep/improve | Useful for workflow runner, debugging, verification. | Keep, but add retention/global storage later. |
| Workflow controller snapshots | fold/remove | Tied to ambient controller slated for defuncting. | Replace with workflow-runner events/state. |
| Mana refs in runtime snapshot | fold/remove | Mana is no longer durable primitive. | Migrate to workflow refs or compatibility-only field. |
| Worktree events | keep/internal/fold | Useful for improve/workflow runner sandboxing. | Keep as internal runtime facts; fold improve-specific UI into workflow runner. |
| Eval candidate closeout | fold/internal/remove | Uses `RunFinalStatus`; useful QA but product-bloated. | Fold into workflow evidence or internal dev. |

## Runtime pieces to keep

### Event stream

Keep `AgentEvent` and runtime event conversion because they support:

- TUI streaming
- one-shot output
- RPC output
- sessions
- run artifacts
- future workflow runner/subagent orchestration

Do not remove event infrastructure as part of product bloat cleanup.

### Runtime snapshots

Keep snapshot/accumulator idea because it prevents TUI/GUI/RPC from duplicating runtime state logic.

But adjust terminology over time:

- `mana_refs` should become `workflow_refs` or compatibility-only.
- `workflow_controller_snapshot` should disappear from normal TUI once ambient controller is defuncted.
- `autonomy_mode` may be kept as internal config/state, but `/autonomy` command can be removed.

### Run evidence

Keep evidence/trace machinery where it supports:

- workflow runner verification
- debugging
- reproducibility
- RPC/automation logs

But add storage hygiene later:

- retention policy
- global run storage
- prune/GC command if necessary

## Runtime pieces to de-emphasize or remove

### Ambient workflow controller events

Current event variants/serialization include `WorkflowControllerSnapshot` in:

- CLI legacy JSON conversion
- RPC conversion
- TUI event kind labeling
- run evidence event kinds
- storage path `workflow_controller.json`

Target:

- normal TUI/one-shot should not emit ambient controller snapshots.
- explicit workflow runner may emit workflow-run events instead.
- old artifacts can remain historical; no migration needed.

### Mana refs

Runtime state currently includes `mana_refs` and docs mention `mana_updated`.

Target:

- active runtime state should talk about workflows, not mana.
- if kept temporarily, mark as compatibility/internal.
- remove from launch docs.

### GUI framing

`docs/runtime-event-state-api.md` includes future GUI guidance.

Target:

- keep runtime state API useful for future UI surfaces, but remove GUI as launch-facing motivation.
- `imp-gui` should leave default workspace members.

### ACP/editor adapter framing

README/docs mention ACP/editor adapters as planned.

Target:

- future roadmap only.
- not in active README product list.

## RPC recommendation

Keep RPC if low-cost, but classify as advanced/internal.

Current CLI RPC supports:

- JSONL input commands:
  - prompt
  - cancel
  - steer
  - followup
- event output
- runtime event/state conversion

This is useful enough not to cut immediately, especially because it may support future product embedding.

But avoid expanding it during tightening:

- no ACP work now
- no new RPC command families
- no workflow-controller-specific public contract
- document as advanced, not primary launch path

## ACP recommendation

Do not implement ACP in this workflow.

Do:

- remove or relocate launch-facing ACP claims.
- keep any architecture notes under roadmap/future if still valuable.

## Workflow runner implications

The future `imp workflow run <id>` should reuse runtime infrastructure:

- emit AgentEvents/RuntimeEvents for step execution
- record run evidence/trace
- expose concise state through RuntimeStateSnapshot if useful
- support subagent/child workflow status

But it should not reuse ambient workflow controller semantics as public API.

Proposed future event/state names:

- `workflow_run_started`
- `workflow_step_started`
- `workflow_step_completed`
- `workflow_step_blocked`
- `workflow_check_started`
- `workflow_check_completed`
- `workflow_artifact_written`
- `workflow_run_completed`

Do not add these now unless implementing runner.

## Docs updates needed

### `docs/runtime-event-state-api.md`

Rewrite active version to say:

- runtime events/state serve TUI, RPC, one-shot, workflows.
- GUI/ACP are future consumers, not current product.
- mana refs are compatibility/internal or removed.

### `docs/rpc.md`

Keep if RPC remains advanced.

Rewrite to avoid:

- suggesting RPC is the main launch path.
- exposing workflow controller snapshots as durable public surface.

### README

Move ACP/editor adapters to roadmap/future.

If RPC stays, mention briefly under advanced usage.

## Risks

- Runtime types may already be consumed by TUI tests and RPC tests; changing names is nontrivial.
- Removing workflow controller snapshot events before runner replacement may reduce observability for current workflow automation.
- Run evidence docs may overlap with eval/prototype docs; cleanup should happen after workflow evidence shape is settled.

## Recommended sequence

1. Keep runtime/RPC code stable during first surface cuts.
2. Remove `imp-gui` from default members; do not delete runtime state API.
3. Defunct ambient workflow controller for normal TUI/one-shot only after workflow runner plan is accepted.
4. Replace workflow-controller-specific events/docs with workflow-runner events when runner exists.
5. Rewrite runtime/RPC docs as advanced/internal after README target surface is finalized.
