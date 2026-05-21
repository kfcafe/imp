# imp-work Implementation Plan

`imp-work` is the native replacement for mana inside imp. The center is prepared work: durable memory, tasks, prototypes, context packs, runs, leases, and structured outcomes that let imp coordinate many low-memory subagents without losing conversational context.

## Foundation Sprint

This is not a minimal v0. It is the first coherent foundation. Scope discipline comes from prototypes, acceptance gates, and sequencing, not from shrinking the concept.

Implemented foundation modules:

- `model`: typed work items for tasks, epics, memory, decisions, checks, context packs, runs, leases, links, and source refs
- `store`: file-backed `.imp/work` layout, memory/task/epic/decision/prototype/context/run/lease persistence, dependency links, task reload, coordinator snapshot reload
- `memory`: conversational memory capture, classification, recent/topic/path/task/text retrieval
- `prototype`: first-class prototype model, observations, record policy, append-only prototype journal
- `context_pack`: deterministic task/prototype context compiler, renderer, stable-prefix hashes, stale detection
- `scheduler`: ready queue, dependency gating, leases, path locks, worker profiles, structured outcomes, coordinator summaries
- `prepared_worker`: single prepared-task launch loop with optional durable persistence
- `prepared_prototype`: prepared prototype launch loop with observation recording and parent-context staling
- `runtime`: executor traits and runtime seam for imp-core subagent execution without transcript coupling

Implemented product primitives:

- conversational memory
- tasks, epics, and subtasks
- first-class decisions
- first-class prototypes
- task dependencies
- task-owned context packs
- cache-stable prompt rendering
- structured outcomes
- runs and attempts
- persisted leases, path locks, and coordinator summaries

Defer broad product surfaces until the foundation is proven: full TUI replacement, embeddings/semantic search, massive live concurrency, container sandboxing, and committed mana import code.

## Vocabulary

- **Memory**: durable facts, preferences, decisions, notes, and conversation learnings.
- **Task**: concrete production work.
- **Epic**: larger work goal containing tasks/subtasks.
- **Prototype**: bounded disposable experiment whose durable output is evidence and learning.
- **Task Context Pack**: versioned, deterministic launch context for a task or prototype.
- **Run / Attempt**: append-only execution record.
- **Lease**: temporary worker claim on a schedulable work item.
- **Check**: verification command or manual criterion.

Avoid user-facing `mana`, `unit`, or generic `graph` terminology.

## Conversational Memory

imp-work must preserve the best mana behavior: while chatting, the agent can capture durable ideas that would otherwise be forgotten. The improvement is retrieval: the user should not need to remember where a note was stored.

Memory items should be classifiable as:

- fact
- preference
- decision
- follow-up
- note
- prototype learning

Memory should be linked by topic, task/epic/prototype, path, source conversation, time, and eventually semantic/entity matches. The first foundation can use append-only markdown plus text/path/topic retrieval; semantic retrieval can follow once the core loop works.

## Prototype Primitive

Prototype is both a work item and a tool.

A prototype asks:

> Answer this uncertainty with disposable code evidence.

A production task asks:

> Implement this cleanly.

Prototype fields:

- id
- title/question
- parent work item
- hypothesis
- evidence required
- sandbox policy
- timebox
- evidence
- learnings
- follow-ups
- decision: promote, discard, iterate, or inconclusive
- cleanup policy

Prototype code is guilty until promoted. The default is ignored scratch code under `.tmp/imp-prototypes` or another untracked sandbox. Durable value lives in learnings, evidence, memory, and promoted follow-up tasks.

### Prototype Tool

The agent-facing tool is named `prototype` to match imp's existing tool style. Its first action is `run`.

The tool contract is:

```text
question -> bounded disposable code experiment -> structured evidence -> learning -> promote/discard/iterate recommendation
```

It is not just bash. It creates an isolated scratch directory, writes complete prototype code, runs it with bounded time/output, captures artifacts/logs, and can reconcile learnings into `.imp/work`.

Current foundation behavior:

- supports shell, Python, Rust, JavaScript, TypeScript, Go, Elixir, Ruby, Perl, Lua, Zig, Odin, and Swift when local runtimes exist
- TypeScript prefers `node --experimental-strip-types`, then bun, then deno
- writes sandbox artifacts under `.tmp/imp-prototypes/P-*`
- accepts `action = run`, `parent_work`, `evidence`, `learnings`, `followups`, `hypothesis_result`, and `recommended_action`
- accepts `record = none | memory | prototype`
- `memory` appends learnings to `.imp/work/memory.md`
- `prototype` appends a full observation to `.imp/work/prototypes.md`
- prepared prototypes can record learnings/followups against a parent task and stale related parent context packs

## Task Context Packs

Every schedulable task should own a prepared context pack. A subagent should be able to start from that pack without rediscovering the repo.

A task context pack includes:

- objective and user intent
- non-goals
- acceptance criteria
- relevant memory
- relevant files/symbols/tests
- source refs and provenance
- prior attempts
- checks
- allowed autonomy
- expected structured outcome

Prototype context packs are similar but emphasize question, hypothesis, allowed shortcuts, forbidden production coupling, evidence required, timebox, cleanup, and promotion criteria.

Context packs should be immutable by version, deterministic, source-referenced, stale-detectable, and cache-stable.

## Cache-Stable Rendering

Render context from most stable to least stable:

1. tools and base system prompt
2. project instructions
3. project memory summary
4. parent epic context
5. task/prototype context pack
6. dynamic launch message

Dynamic data such as timestamps, lease ids, attempt ids, and worktree paths must stay out of cacheable blocks.

Provider-specific cache controls belong outside imp-work, but imp-work should expose provider-neutral block stability metadata.

## Runtime Loop

The desired runtime loop is:

```text
conversation -> memory/task/prototype -> context pack -> worker/prototype run -> evidence -> decision -> task/memory updates
```

Prototype is the uncertainty valve. Use it when the agent is about to guess and code can answer faster than planning:

- API shape uncertainty
- parser/data model uncertainty
- provider cache behavior
- scheduler/path-lock races
- performance/concurrency questions
- unfamiliar runtime/library behavior
- risky abstractions

After a prototype run, imp-work reconciles the result:

- append prototype evidence
- write durable learnings to memory
- create follow-up task seeds
- mark prototype promoted/discarded/iterate in observation state
- delete/archive sandbox according to cleanup policy
- stale parent task context packs when prototype learnings/followups change context inputs

Prepared task workers follow the same pattern: launch from a rendered context pack, return `WorkOutcome`, persist run/outcome/summary/memory updates/follow-up task seeds, and stale related context packs when outcome updates change context inputs.

## Scheduler Foundation

For hundreds of subagents, the coordinator must not ingest hundreds of transcripts. Workers return structured outcomes; transcripts remain in logs and are pulled only when needed.

Scheduler foundation:

- ready queue
- task/prototype leases and heartbeats
- path locks
- worker profiles
- run/attempt records
- structured outcomes
- compact coordinator summaries
- context-pack readiness and staleness checks

## Storage

Tracked, human-readable state:

```text
.imp/work/tasks.md       # epics, tasks, dependencies, current task context_pack links
.imp/work/memory.md      # facts, preferences, decisions-as-memory, notes, prototype learnings
.imp/work/decisions.md   # first-class proposed/accepted/rejected/superseded decisions
.imp/work/prototypes.md  # planned/running/observed/promoted/discarded prototypes and observations
.imp/work/contexts/*.md  # human-readable rendered context packs
```

Ignored/generated state:

```text
.tmp/imp-prototypes/
.imp/work/.cache/items.jsonl  # cached runtime/derived WorkItems such as leases
.imp/work/runs/
.imp/work/logs/
```

The current store is intentionally file-backed and append-friendly. It provides typed APIs for aggregate work item persistence/loading, memory/task/epic/decision/prototype/context/run/lease persistence, scheduler reload, coordinator reload, task status updates, decision/prototype lifecycle updates, context staling/refresh, dependency round-tripping, and lease release on outcomes. The next store evolution is to replace ad-hoc markdown parsing with a stricter file format or indexed cache once the UX shape settles.

## Native `work` Tool Surface

`imp-core` now exposes a native `work` tool backed by `imp_work::WorkStore`. This is the first user/agent-facing replacement surface for mana task and memory workflows.

Implemented actions:

- `create`: create source-controlled `task`, `epic`, `memory`, `decision`, or `prototype` records.
- `list`: list aggregate work items, with filters for `kind`, `status`, `parent_work`, `path`, and `limit`. Supports `context_pack` and cached runtime `lease` items too.
- `show`: show an item or context pack by id without knowing which file backs it.
- `next`: list scheduler-ready tasks. With `require_context: true`, only returns ready tasks with a loadable, non-stale task-owned context pack.
- `claim`: claim a scheduler-ready task by id or first ready task, mark it active, and persist a `Lease` cache item. With `require_context: true`, claim requires a fresh task-owned context pack.
- `update`: update task status, decision status, or prototype status.
- `context`: compile and persist a task/prototype context pack with retrieved memory. For tasks, updates `task.context_pack` to the generated context id.
- `refresh_context`: create a next-version task context pack from the current one, relink the task, and mark the previous context stale.
- `remember`: capture conversational memory with automatic classification and source/path/topic/parent links.
- `search`: retrieve conversational memory by text/query, topic, parent work, path, and limit.
- `outcome`: record a structured task outcome, update task status, persist run/outcome/summary/memory updates/follow-up task seeds, stale related contexts, and release persisted leases.
- `prototype_outcome`: record structured prototype evidence/learnings/follow-ups, update prototype lifecycle status, and feed learnings/follow-ups into parent work memory/context state.
- `runs`: inspect persisted run/outcome history and coordinator summary.

Important current behavior:

- task dependencies are persisted as `depends_on` links and gate scheduler readiness;
- task-owned context packs are persisted via `context_pack: CTX-...` in `tasks.md`;
- prepared-work scheduling can be opted into with `require_context: true`;
- claims persist lightweight lease records to `.imp/work/.cache/items.jsonl` and outcomes release them;
- conversational memory can be found later by search without the user remembering where it was stored.

## Current Verification

The foundation currently verifies with:

```sh
cargo test -p imp-work
cargo test -p imp-core work_tool
cargo test -p imp-core prototype -- --nocapture
```

At the time of this plan update, `imp-work` has 54 passing tests covering model, store, conversational memory, prototypes, context packs, scheduler, prepared workers, prepared prototypes, and the runtime seam. The native `work` tool has 25 passing focused tests covering create/list/show/next/claim/context/refresh_context/search/remember/update/outcome/prototype_outcome/runs behavior.

## Next Integration Work

The foundation crate is in place. The next work should connect it to user-visible imp surfaces:

1. Route selected agent memory/task behaviors from mana to the native `work` tool / `imp_work::WorkStore`.
2. Add a real imp-core subagent executor implementing `TaskExecutor` / `PrototypeExecutor`.
3. Surface `.imp/work` memory/tasks/prototypes/context packs in CLI/TUI views.
4. Add stricter source-controlled file format validation for `tasks.md`, `memory.md`, `decisions.md`, `prototypes.md`, and `contexts/*.md`.
5. Add provider-aware cache controls in `imp-llm` using imp-work's provider-neutral context block stability metadata.
6. Keep mana migration as an off-repo/local script only.

## Mana Migration

Mana is a reference and local migration source, not a runtime dependency. Do not commit `import/mana.rs` or a mana import module into `crates/imp-work`.

If migration is needed, use a local/off-repo script that emits imp-work files, validate the result, and discard the script.
