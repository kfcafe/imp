# imp-work mana removal ledger

This ledger records the evidence required before removing mana runtime dependencies from imp.

## Current decision

Do **not** remove mana code in this patch. The parity foundation is now substantially implemented, but the final cutover is destructive and needs explicit approval after review of the ledger and committed migration work.

## Evidence by removal criterion

| Criterion | Evidence | Status |
|---|---|---:|
| `.mana` -> `.imp/work` migration imports active units | `crates/imp-work/src/mana_shadow.rs`; `cargo test -p imp-work mana_migration -- --nocapture` | satisfied for fixture coverage |
| Archived mana units import with history refs | `ManaHistoryRef`, `archived_units`, archive fixture test | satisfied for fixture coverage |
| Native dependency tree/readiness exists | `crates/imp-work/src/workflow.rs`; workflow tests | satisfied |
| Native verify/close/fail conventions exist | `close_task_with_conventions`, `fail_task_with_conventions`; work-tool actions | satisfied |
| Native work tool exposes tree/verify/close/fail | `crates/imp-core/src/tools/work.rs`; `cargo test -p imp-core work_tool -- --nocapture` | satisfied |
| Multi-agent run coordinator exists | `MultiAgentRunPlan`, `RunPolicy`, `plan_multi_agent_run`, `complete_multi_agent_run`; scheduler tests | satisfied as primitive coordinator |
| Resource/path conflict policy exists | `RunPolicy`, `PathConflictPolicy`, dispatch blocker tests | satisfied as baseline |
| Durable event/evidence log exists | `crates/imp-work/src/event.rs`; event log cursor/artifact tests | satisfied as baseline |
| Machine output has structured fields | work tool action details and parity matrix; existing work_tool tests | partial, but enough for current work-tool surface |
| Real-project migration has been run and spot-checked | Not yet performed | open before destructive removal |
| New work defaults to `.imp/work` | Work tool is native, but full runtime routing cutover is not complete | open before destructive removal |
| `.mana` support narrowed to import-only/legacy | Not yet implemented | open before destructive removal |
| Removal of mana tool/mana_worker/mana-core dependency approved | Not yet approved | blocked on explicit approval |

## Required final cutover steps

1. Commit the completed imp-work parity work in focused commits.
2. Run a real-project `.mana` dry-run import and write-mode import into `.imp/work` backup scope.
3. Compare counts, statuses, dependencies, verify commands, notes/decisions, and archived units.
4. Add routing change so new local work uses `.imp/work` by default.
5. Mark `.mana` support import-only/legacy.
6. Ask for explicit approval to remove/quarantine:
   - `crates/imp-core/src/tools/mana.rs`
   - `crates/imp-core/src/mana_worker.rs`
   - `mana-core` dependency from imp crates
   - mana-specific agent-loop workflow progress affordances
7. Run full workspace verification after removal.

## Current blocker for actual removal

The destructive removal is intentionally blocked until a real-project migration and routing cutover are reviewed. Fixture parity is strong enough to proceed to cutover preparation, but not enough to delete mana runtime paths safely without approval.
