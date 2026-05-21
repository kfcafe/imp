# imp-work vs mana feature parity matrix

This document tracks what `imp-work` must match or surpass before imp can remove its mana runtime dependency. It is intentionally broad: mana is not just a file format, it is a bundle of graph, scheduling, verification, run, UX, and durable-history conventions that imp has relied on.

Status legend:

- **surpasses** — imp-work already provides a stronger imp-native version.
- **matches** — imp-work has an equivalent capability.
- **partial** — some primitives exist, but behavior or tooling is incomplete.
- **missing** — no meaningful equivalent yet.
- **omit** — intentionally not part of imp-work; replacement/removal rationale required.

## Two-column parity matrix

| Mana capability / behavior | imp-work current equivalent | Status | Required work to match/surpass | Gas City inspiration / notes |
|---|---|---:|---|---|
| `.mana` durable project graph | `.imp/work` store primitives (`WorkStore`, tasks, memory, decisions, prototypes, runs, leases) | partial | Finish migration importer and write-mode conversion; define canonical `.imp/work` layout/versioning. | Gas City centers durable state in Beads; imp-work should make `.imp/work` the universal imp work substrate. |
| Units with id/title/description/design/acceptance | `Task`, `Epic`, `WorkItem`, `ManaShadowUnit` importer mapping | partial | Preserve all mana fields in importer; add explicit design/description fields or durable memory mapping policy. | Keep work primitive central; don't couple behavior to orchestration roles. |
| Parent/child tree / epics | `Task.parent`, `Epic`, new workflow tree helpers | partial | Expose tree through work tool; support parent auto-ready/auto-close policy; render full hierarchy. | Gas City convoys/container expansion show that containers should track batches without pretending to be workers. |
| Dependency links | `LinkKind::DependsOn`, `workflow::readiness_for`, `build_work_tree` | partial | Add cycle detection tests beyond stack-local detection; expose ready/blocked explanation in tool and run scheduler. | Gas City lifecycle uses dependency waves and barriers, not hints. |
| `mana next` ready work selection | scheduler primitives and workflow readiness helpers | partial | Implement critical-path/priority ready queue for imp-work and explain why tasks are blocked. | Mana `ReadyQueue` + Gas City ready bead query both separate observation from dispatch. |
| Dependency-ordered run waves | early `imp-work::Scheduler`; mana has `RunPlan`/`RunWave` | missing/partial | Add wave planning over imp-work tasks, respecting deps, priority, path conflicts, and ready status. | Gas City dependency-aware bounded parallel lifecycle: plan serially, execute concurrently, commit serially. |
| `mana run --jobs N` multi-agent dispatch | leases/runs exist as primitives, but no full multi-agent runner | missing | Implement `imp work run --jobs N` or tool action: ready waves, bounded concurrency, worker assignment, progress events, result aggregation. | Gas City agent pools: min/max, demand query, runtime provider abstraction, worker boundary, drain behavior. |
| Resource-aware dispatch / memory reserve | none in imp-work | missing | Add concurrency/resource policy: max jobs, memory reserve, idle timeout, keep-going, batch verify, worktree isolation. | Mana-pool has `PoolConfig`; Gas City separates pool demand from runtime provider calls. |
| Worker lease/claim | `Lease`, scheduler lease records, work-tool `claim`/`release` | partial | Make leases durable, expiring, and tied to run/task outcomes; prevent double-claim; expose heartbeat/status. | Gas City emphasizes runtime/session provider boundaries and double-claim tests. |
| Agent/subagent identity | `WorkerProfile` and leases are early primitives | partial | Define worker profiles, model/provider hints, assignment packets, and per-worker result summaries. | Gas City has zero hardcoded roles; all role behavior via config/prompt templates. |
| File-lock/path conflict dispatch | mana-pool has file locking/path metadata | missing | Use task paths to avoid concurrent conflicting jobs; report conflict blockers. | Plan serially and commit deterministically; worker completion order must not affect state. |
| Retry context | mana run derives attempts/failure notes; imp-work runs record outcomes | partial | Add retry policy/history to scheduler and importer; preserve prior mana attempts. | Gas City leaves retry to next reconciler tick with explicit terminal results. |
| Verify command per unit | `Check` + `CheckResult`; workflow close conventions | partial | Expose `verify` action; run checks with timeout; persist results/artifact refs; support fast/deferred verify. | Mana-pool has verify groups; Gas City tests require visible event/result behavior, not hidden side effects. |
| Verify-before-close | `workflow::close_task_with_conventions` | partial | Wire into work tool close action and store mutation; add forced close with reason and evidence. | Same principle as release gates: explicit evidence before completion. |
| Close/fail/reopen conventions | workflow close/fail helpers; task status model | partial | Add work tool actions; persist close/fail evidence; add reopen/drop conventions. | Gas City uses explicit terminal results and rollback rules. |
| Fail/blocker evidence | `fail_task_with_conventions` emits blocker memory | partial | Store reason, next action, checks, artifacts, and recovery hints; render in next/tree. | Gas City terminal result taxonomy: success, provider_error, deadline, canceled, panic_recovered. |
| Notes | `MemoryItem` | partial | Migration importer should convert notes; native UX should make notes easy and queryable. | Beads treats work/mail/molecules as one task-store substrate; memory should be first-class, not loose text. |
| Decisions | `Decision` plus `MemoryKind::Decision` import mapping | partial | Preserve decision status/resolution; expose decision add/resolve actions. | Config/prompt-driven policy decisions should be durable. |
| Facts / verified facts | `MemoryKind::Fact` exists | partial | Add verification/TTL semantics if needed, or intentionally omit in favor of evidence records. | Event/evidence log may be better than fact TTL for imp runtime. |
| Artifacts/evidence refs | `SourceRef`, run changed paths, prototype evidence | partial | Add explicit artifact refs and durable evidence summary records; migrate mana artifacts. | Gas City event bus and Beads store keep observable history separate from runtime process state. |
| Prototype experiments | native `Prototype`, prototype tool, evidence/observation/outcome | surpasses | Connect prototype outcomes to parent tasks and run evidence consistently. | imp-work is stronger here because prototype is an imp-native workflow, not generic graph notes. |
| Context assembly | `ContextPack`, context renderer, conversation memory capture | surpasses/partial | Tie context packs to run/task scheduling and importer output. | Gas City prompt templates are the behavioral spec; imp-work context packs should be equally explicit. |
| Memory/context search | imp has session/memory work; imp-work memory index exists | partial | Add work-memory search/filter and migration from mana notes/decisions/facts. | Gas City event/query docs separate stable machine output from human renderings. |
| Archive/history | mana archives closed units and stores attempts | missing/partial | Define imp-work archive/history policy, compaction, and migration of closed `.mana` units. | Gas City append-only event bus suggests history should be observable, not destructive moves only. |
| Status/list/show/tree CLI UX | native work tool can create/list/claim/run; tree not exposed yet | partial | Add full `work` actions and eventual CLI shell commands. | Gas City CLI is projection over API/domain types, not independent schemas. |
| JSON/machine output | work tool returns structured details; output mode contract exists | partial | Stabilize machine event/result schema for work actions and multi-agent runs. | Gas City events JSONL/SSE contract is a model: API is canonical, CLI is projection. |
| Review gates | mana has review/verify workflows | missing/partial | Add review status/action and evidence requirements if imp still needs review lanes. | Gas City review quorum formula shows review can be a formula/molecule, not hardcoded runtime logic. |
| Config / policy | imp has config; imp-work lacks dedicated policy model | partial | Add run policy: max jobs, keep-going, verify mode, conflict handling, trust, sandbox/worktree options. | Gas City progressive capability model: features activate by config presence. |
| Worktree isolation | mana-pool supports worktrees | missing | Add optional per-worker git worktree/sandbox strategy before true parallel editing. | Gas City runtime providers keep side effects in Layer 0; imp-work should keep execution substrate separate. |
| Claim/owner semantics | leases and claim action exist | partial | Add expiry/heartbeat/stale lease recovery and double-claim prevention tests. | Gas City tests explicitly cover double-claim conflicts and rollback behavior. |
| MCP/API embedding | mana has MCP/API surfaces | omit/partial | Decide if imp-work needs an external API or only imp-native tools. Do not rebuild mana platform accidentally. | Gas City has HTTP/SSE for supervisor; imp may only need local tool/CLI first. |
| Durable event stream | mana logs/attempts; gascity has event bus | missing | Add append-only work event log for task/run/check/lease/prototype events. | Strong recommendation from gascity: event bus as universal observation substrate. |
| Import/export/migration | `mana_shadow` dry-run/write importer started | partial | Finish robust parser/API path, parity reports, write mode, count checks, archive import, rollback/backups. | Migration should be one-way; no long-lived two-way sync unless forced. |
| Removal criteria | none automatic | missing | Define gates: migration parity, workflow parity, multi-agent run parity, UX parity, storage backup, docs. | Release gates/conformance matrix style from Gas City can become removal ledger. |

## Gap list

Critical gaps before mana removal:

1. **Complete migration importer** — dry-run plus write mode from `.mana` to `.imp/work`, including archive/history and parity report.
2. **Work tool workflow actions** — expose tree, verify, close, fail, reopen/drop, and next-ready through native `work`.
3. **Dependency-aware multi-agent run** — ready queues, waves, `--jobs`, leases, heartbeats, result aggregation, keep-going, retries.
4. **Durable event/evidence log** — append-only work events for runs, checks, leases, task changes, and artifacts.
5. **Resource and conflict policy** — max concurrency, memory reserve, file path locking/conflict detection, worktree isolation.
6. **Archive/history model** — preserve closed/failed/attempt history from mana and define imp-work retention/compaction.
7. **Machine output contract** — stable structured run/work events equivalent or better than mana's JSON/run stream.
8. **Removal ledger** — explicit checklist proving mana tool, mana_worker, and mana-core dependency can be removed.

## Ordered implementation roadmap

1. Finish `359.1`: `.mana` -> `.imp/work` importer and parity/loss report.
2. Finish `360.1`/core workflow helpers and commit cleanly.
3. Implement `361.1`: native work tool `tree`, `verify`, `close`, `fail` actions.
4. Add `work next`/ready queue with blockers, priority, and dependency explanation.
5. Add dependency-wave planner for imp-work tasks.
6. Add multi-agent run coordinator with bounded concurrency and leases.
7. Add event/evidence log and machine output for work runs.
8. Add resource/path conflict policy and optional worktree isolation.
9. Run migration on a real `.mana` project in dry-run and write mode; compare counts and spot-check behavior.
10. Switch new local imp work to `.imp/work`; keep `.mana` import-only.
11. Remove mana runtime paths after removal criteria pass.

## Gas City orchestration inspiration

Inspected files:

- `/Users/asher/gascity/AGENTS.md`
- `/Users/asher/gascity/README.md`
- `/Users/asher/gascity/engdocs/design/dependency-aware-bounded-parallel-lifecycle.md`
- `/Users/asher/gascity/engdocs/design/agent-pools.md`
- `/Users/asher/gascity/engdocs/architecture/dispatch.md`
- `/Users/asher/gascity/engdocs/architecture/event-bus.md`

Relevant patterns for imp-work:

1. **Work is the primitive, not orchestration.** Gas City says orchestration is a thin layer over the work substrate. imp-work should avoid rebuilding mana as a separate platform; task/check/memory/event data should be the center.
2. **Zero hardcoded roles.** Gas City treats role behavior as user-supplied config/prompt templates. imp-work multi-agent support should use worker profiles/config, not hardcoded reviewer/implementer roles.
3. **Plan serially, execute concurrently, commit serially.** This is the strongest pattern for imp-work multi-agent runs. Build dependency waves deterministically, run workers in bounded parallelism, then apply state transitions in planned order.
4. **Dependencies are barriers.** A dependency tree should produce waves; downstream tasks do not dispatch until upstream tasks close/verify.
5. **Event bus as observation substrate.** Gas City's append-only event log with sequence cursors is a strong model for imp-work run events and reattach/status UX.
6. **Agent pools with demand checks.** Gas City's pool model suggests imp-work should eventually support worker profiles with min/max/demand, but first implement fixed `--jobs` bounded concurrency.
7. **Runtime provider boundary.** Gas City separates sessions/runtimes from work state. imp-work should separate work graph/scheduler from imp agent runtime execution.
8. **Conformance tests.** Gas City uses conformance matrices for providers/events/workers. imp-work should add conformance-style tests for store, scheduler, worker runner, and event log before mana removal.
9. **No hidden framework intelligence.** Put policy in config/prompts where possible; keep imp-work deterministic infrastructure focused on routing, state, leases, and evidence.

## removal criteria

Do not remove mana from imp until all are true:

- `.mana` -> `.imp/work` migration imports active and archived units with acceptable parity report.
- Native imp-work dependency tree, next-ready, verify/close/fail, and reopen/drop workflows pass tests.
- Native work tool exposes the workflows the agent loop needs without calling mana.
- Multi-agent run supports bounded jobs, dependency waves, leases/heartbeats, and result aggregation.
- Existing imp workflows that currently call `mana` have imp-work equivalents or explicit deprecation/migration paths.
- A real project migration has been dry-run, written to `.imp/work`, spot-checked, and backed up.
- `mana` tool usage in imp is narrowed to import-only/legacy mode.
- Only then remove `crates/imp-core/src/tools/mana.rs`, `mana_worker`, mana-core dependency, and mana-specific runtime assumptions.
