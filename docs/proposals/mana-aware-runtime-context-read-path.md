# Mana-Aware Runtime Context Read Path

> Proposal for `.10.4` — April 2026
>
> Defines how imp's prompt assembly and headless dispatch should consume
> mana-derived durable context (facts, wiki pages) without collapsing
> the four memory layers into one.
>
> Depends on: `.10.1` (memory architecture), `.10.2` (wiki schema)

---

## Current Read Paths

### Prompt assembly (`builder.rs` → `system_prompt.rs`)

The `AgentBuilder::build()` method assembles the system prompt in layers:

| Layer | Source | Loaded from | Current state |
|-------|--------|-------------|---------------|
| 1 | Identity + tools + doctrine | Code constants + tool registry | Working |
| 1.25 | Execution policy | Code constants | Working |
| 1.5 | Environment | `cwd`, OS detection | Working |
| 2 | Project context | `AGENTS.md` files discovered on disk | Working |
| 3 | Skills index | `.imp/skills/` + `~/.config/imp/skills/` | Working |
| 4 | Mana facts | `AssembleParams.facts` | **Gap: `facts: &[]` — never populated** |
| 4.5 | Engineering guardrails | Config + profile resolution | Working |
| 5 | Task context | `SessionOptions.task` (headless only) | Working |
| 6 | Agent memory | `memory.md`, `user.md` frozen snapshots | Working |

**Key gap:** Layer 4 has working rendering code (`facts_layer()`) but
`builder.rs` always passes an empty slice. No mana facts are loaded at
session start.

### Headless dispatch (`imp run <id>` in `main.rs`)

1. `load_mana_unit()` reads the unit markdown file from `.mana/`.
2. File paths are extracted from `files:` frontmatter, `paths:` frontmatter,
   or auto-detected from the description text.
3. `context_prefill::assemble_context()` reads those files and packages them
   as a cached prefix message (`PrefillConfig` with 50K token budget).
4. The prefill messages are injected once before the first prompt in
   `ImpSession::prompt()`, with a synthetic assistant acknowledgment to
   maintain alternation.
5. Task context (title, description, verify, attempts, dependencies,
   decisions) is assembled into Layer 5 of the system prompt.

**Key gap:** No wiki pages or relevant facts are included in the prefill
or prompt assembly for headless runs.

### Interactive sessions

Same as prompt assembly above. No mana-derived context is loaded at
session start beyond what the agent discovers by using the `mana` tool
during the conversation.

---

## Prefix caching constraints

Anthropic's prefix caching caches the system prompt + early turns. For
cache stability:

1. **System prompt must be stable across turns.** Changes invalidate the
   cache. This is why personal memory uses the frozen snapshot pattern.
2. **Context prefill messages form the cacheable prefix.** Subsequent
   turns get `cache_read` on this prefix.
3. **Anything that changes per-turn should NOT be in the system prompt.**
   It should be in turn-level messages or tool results.

**Implication for mana context:** Mana facts loaded at session start are
stable within a session (facts don't change mid-conversation). Wiki
pages are similarly stable. Both can safely go into the system prompt or
prefill without breaking caching.

However, mana **work status** (unit progress, new child jobs, completion
notifications) changes within a session. This must flow through tool
results or follow-up messages, never the system prompt.

---

## Compaction interaction

When `/compact` runs or auto-compaction triggers:

1. Older conversation history is replaced by a structured summary.
2. `get_active_messages()` returns summary + preserved tail + new turns.
3. The system prompt is not affected by compaction.
4. Context prefill messages are injected once and then become part of the
   session history — they survive compaction as early entries.

**Implication:** Mana context injected into the system prompt (facts,
wiki summaries) survives compaction naturally because it is in the
system prompt, not in conversation history. Context prefill also survives
because it is injected as early messages that are typically in the
preserved tail.

**Risk:** If wiki pages or facts become stale during a very long session,
the system prompt will contain outdated information. This is acceptable
for Phase 1 (same tradeoff as frozen personal memory). Phase 2 could
add a "re-ground" operation that re-loads mana context after compaction.

---

## Proposed Read Path

### What to load and when

| Context type | When loaded | Where injected | Stability |
|-------------|-------------|----------------|-----------|
| Mana facts (all project facts) | Session start | System prompt Layer 4 | Frozen per session |
| Wiki index summary | Session start (interactive) | System prompt new Layer 4.1 | Frozen per session |
| Relevant wiki pages | Headless dispatch | Context prefill messages | Frozen per run |
| Mana memory context | Session start (interactive) | System prompt or first-turn injection | Frozen per session |
| Unit task context | Headless dispatch | System prompt Layer 5 | Frozen per run |
| Unit file prefill | Headless dispatch | Context prefill messages | Frozen per run |
| Work status updates | During session | Tool results from mana tool | Per-turn |

### Prompt assembly changes (interactive sessions)

**Phase 1 — populate Layer 4:**

In `builder.rs`, after discovering `agents_md` and `skills`, load mana
facts from `.mana/`:

```rust
// In builder.rs, before system_prompt::assemble()
let facts = if self.cwd.join(".mana").exists() {
    load_mana_facts(&self.cwd.join(".mana"))
} else {
    vec![]
};
```

This fills the currently-empty `facts: &[]` slot. Each fact renders as:
`- "DB is PostgreSQL" [verified 3d ago]`

**Phase 1 — add wiki index awareness:**

If `.mana/wiki/index.md` exists, read it and inject a compact summary
into the system prompt (new Layer 4.1, after facts). This tells the
agent what synthesized knowledge is available without injecting all
pages:

```
Available project wiki pages (read with `read` tool when relevant):
- systems/orchestration.md — How mana dispatches units to agents.
- concepts/fail-first.md — Verify must fail before unit creation.
[12 pages total — see .mana/wiki/index.md]
```

Token cost: ~200-400 tokens for a moderate wiki. Acceptable.

**Phase 1 — add mana memory context:**

If `mana context` (no-ID) output is available, inject a compact version
as an additional system prompt section. This surfaces stale facts,
claimed units, and recent work. Token cost depends on project state but
is bounded by mana's own truncation.

### Headless dispatch changes

**Phase 1 — add wiki page prefill:**

In `run_headless_mode()`, after loading file specs from the unit:

1. Read the unit's `paths` and `labels`.
2. If `.mana/wiki/index.md` exists, scan it for pages whose tags or
   paths overlap with the unit's scope.
3. Add matching wiki pages to the `file_specs` list for context prefill.

This means a unit working on `src/commands/run/` would automatically get
`systems/orchestration.md` if that wiki page lists
`src/commands/run/` in its sources.

**Phase 1 — add fact injection:**

Load project facts into the task context assembly so headless agents
see relevant facts alongside the unit description. This uses the same
`facts_layer()` rendering already in `system_prompt.rs`.

---

## What NOT to Do

1. **Do not load wiki pages into the system prompt for interactive
   sessions.** Wiki pages can be large. The index summary is enough;
   the agent reads full pages on demand with the `read` tool.

2. **Do not refresh mana context mid-session.** The frozen snapshot
   pattern is correct. Mana work status flows through tool results.
   Re-loading facts or wiki mid-session would break prefix caching.

3. **Do not mix work-status signals with knowledge context.** Facts and
   wiki pages are knowledge. Unit progress is work status. They use
   different delivery mechanisms (system prompt vs tool results).

4. **Do not make wiki pages mandatory.** If `.mana/wiki/` does not
   exist, all read paths gracefully degrade to current behavior.

5. **Do not duplicate mana context across layers.** If a fact is in
   Layer 4, do not also inject it via context prefill. Each piece of
   context should have exactly one injection point.

---

## Code Surfaces for Phase 1 Implementation

| Change | File | Effort |
|--------|------|--------|
| Load mana facts at session start | `crates/imp-core/src/builder.rs` | Small — read `.mana/` facts, convert to `Fact` structs |
| Add wiki index summary layer | `crates/imp-core/src/system_prompt.rs` | Small — new optional layer after facts |
| Add `AssembleParams.wiki_index` | `crates/imp-core/src/system_prompt.rs` | Small — new optional field |
| Load wiki index in builder | `crates/imp-core/src/builder.rs` | Small — read and parse `.mana/wiki/index.md` |
| Add wiki page prefill in headless | `crates/imp-cli/src/main.rs` | Medium — scan wiki index, match against unit scope |
| Add fact injection in headless | `crates/imp-cli/src/main.rs` | Small — load facts, pass to task context |

Total: ~6 focused changes, none requiring new crates or trait changes.

---

## Phase 2 (Future)

- **Re-ground after compaction:** After `/compact`, optionally reload
  mana facts and wiki index to catch changes from other agents.
- **Smart wiki page selection:** Use unit labels, paths, and description
  keywords to score wiki page relevance instead of simple path overlap.
- **Wiki query tool:** A dedicated tool that searches the wiki index and
  returns relevant pages, avoiding the need to `read` files manually.
- **Mana context streaming:** Surface mana memory context updates as
  tool-result-style injections during the session, not just at start.

---

## Summary

The narrowest Phase 1 that adds real value:

1. **Populate Layer 4** — load mana facts into `builder.rs` (filling the
   existing empty slot).
2. **Add wiki index awareness** — inject a compact summary of available
   wiki pages so the agent knows what to read.
3. **Add wiki page prefill for headless** — automatically include
   relevant wiki pages in `imp run` context alongside unit files.

All three preserve Prefix caching (frozen at session/run start), respect
Compaction interaction (system prompt survives compaction), and degrade
gracefully when no mana facts or wiki exist.
