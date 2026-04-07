# Mana Wiki Schema and Knowledge-Maintenance Workflow

> Proposal for `.10.2` — April 2026
>
> Defines the synthesized knowledge layer that sits on top of mana's
> existing units and facts. Concrete file schema, trust tiers, citation
> rules, and maintenance operations.
>
> Depends on: `.10.1` (imp memory architecture and mana ownership boundaries)

---

## Motivation

Mana stores the history of work. Units track what was done, facts record
what is verified, attempt logs capture what failed. But none of these
store **maintained understanding** — the kind of synthesized, cross-
referenced knowledge that answers questions like:

- "How does orchestration work in this project?"
- "Why did we choose worktree isolation?"
- "What are the known failure modes of close.rs?"
- "What changed about the auth system over the last month?"

Today every agent rediscovers this from code, unit descriptions, and
scattered notes. The wiki layer makes that understanding compound instead
of evaporate.

This is adapted from Karpathy's LLM Wiki pattern. The key difference for
mana: the wiki leverages mana's verification model. Strong claims should
point to `mana fact` entries or cite concrete sources. The wiki is not a
confidence-free dumping ground — it has explicit trust tiers.

---

## Architecture Decision

**Recommendation: phased hybrid.**

Phase 1: The wiki is an **imp-maintained companion layer** — a directory
of markdown files at `.mana/wiki/` that imp agents read and write using
standard file tools. No mana-core commands required.

Phase 2 (future): If the pattern proves valuable, mana-core could add
`mana wiki ingest`, `mana wiki query`, `mana wiki lint` as first-class
commands. But this should be driven by real usage, not speculative
architecture.

**Rationale:** The wiki is synthesized prose, not verified work. It does
not need verify gates, dependency graphs, or status tracking — those are
mana's core model. Forcing wiki pages into the unit model would weaken
both concepts.

---

## Directory Layout

```
.mana/wiki/
├── index.md              # Content catalog — page list with summaries
├── log.md                # Chronological maintenance log
├── systems/              # How things work
│   ├── orchestration.md
│   ├── session-memory.md
│   └── prompt-assembly.md
├── concepts/             # Key ideas and patterns
│   ├── fail-first.md
│   ├── produces-requires.md
│   └── frozen-snapshot.md
├── decisions/            # Why choices were made
│   ├── worktree-isolation.md
│   └── serde-yml-migration.md
├── playbooks/            # How to do things
│   ├── debugging-stuck-run.md
│   └── adding-a-new-tool.md
└── areas/                # Component-level overviews
    ├── commands-close.md
    └── compaction.md
```

### Page types (initial set)

| Type | Directory | Purpose | Example |
|------|-----------|---------|---------|
| System | `systems/` | How a subsystem works end-to-end | `orchestration.md` |
| Concept | `concepts/` | A pattern, invariant, or design principle | `fail-first.md` |
| Decision | `decisions/` | Why a choice was made, alternatives considered | `worktree-isolation.md` |
| Playbook | `playbooks/` | Step-by-step guide for a recurring task | `adding-a-new-tool.md` |
| Area | `areas/` | Component overview for a code area | `commands-close.md` |

New types can be added later. The directory name **is** the type — no
need for a type field in frontmatter.

---

## Page Schema

### Frontmatter

```yaml
---
title: Orchestration
summary: >
  How mana dispatches units to agents, manages waves,
  tracks completion, and handles failures.
sources:
  - file: src/commands/run/ready_queue.rs
  - file: src/spawner.rs
  - file: ARCHITECTURE.md
  - unit: "78"
  - fact: "112"
confidence: high
last_updated: 2026-04-07
related_pages:
  - concepts/fail-first.md
  - decisions/worktree-isolation.md
tags:
  - orchestration
  - agent-dispatch
  - run-command
---
```

**Required fields:**
- `title` — human-readable page title.
- `summary` — 1-3 sentence overview (used by index.md and query results).
- `last_updated` — ISO date of last substantive update.

**Optional fields:**
- `sources` — list of citations. Each entry has a type prefix:
  - `file:` — source file path relative to project root.
  - `unit:` — mana unit ID (open or archived).
  - `fact:` — mana fact ID.
  - `doc:` — design doc or external document path.
  - `url:` — external URL.
- `confidence` — `high`, `medium`, or `low`. See Knowledge tiers below.
- `related_pages` — paths to other wiki pages (relative to `.mana/wiki/`).
- `tags` — freeform labels for discovery.

### Body Sections

Recommended structure (not enforced, but agents should follow it):

```markdown
## Summary
Brief overview of the topic.

## Current behavior
How the system works right now, grounded in cited sources.

## Invariants
Rules that must hold. Each should cite a fact or test.

## Known edge cases
Documented failure modes, gotchas, surprising behavior.

## Recent changes
What changed recently (synthesized from closed units).

## Open questions
Unresolved items, explicitly marked as uncertain.

## Sources
Full citation list (duplicates frontmatter for human readers).
```

---

## Knowledge Tiers

Every claim in the wiki carries an implicit or explicit trust level.

### Tier 1: Verified facts

Atomic, shell-provable claims. These live in `mana fact`, **not** in
wiki pages. Wiki pages cite them.

Examples:
- "Project uses serde_yml 0.0.12" → `mana fact` with `grep` verify.
- "close.rs is the largest command module" → `mana fact` with `wc -l` verify.

**Rule:** If a claim can be expressed as a `mana fact` with a verify
command, it should be. The wiki page cites the fact ID.

### Tier 2: Synthesized pages (confidence: high or medium)

Maintained summaries built from multiple sources. Cannot be fully
machine-verified, but every strong claim should cite at least one source.

Examples:
- "How orchestration chooses ready work" (cites `ready_queue.rs`, unit 78).
- "The authentication flow" (cites files, design doc, relevant facts).

**Rule:** Pages at `confidence: high` should have sources for all major
claims. Pages at `confidence: medium` may have some unsourced synthesis
but should flag gaps.

### Tier 3: Open questions / hypotheses (confidence: low)

Useful observations that are not yet proven. Explicitly marked.

Examples:
- "MCP tools may diverge from CLI close behavior" → filed as open question.
- "Artifact passing may need a richer handoff model" → hypothesis section.

**Rule:** These must be visually distinct. Use the `## Open questions`
section or inline markers like `[hypothesis]` or `[unverified]`.

### Tier boundary rules

- A wiki page should never present a Tier 3 claim as if it were Tier 1.
- When a Tier 3 claim gets verified, convert it to a `mana fact` and
  update the wiki page to cite the fact.
- When a `mana fact` fails re-verification, wiki pages citing it should
  be flagged for review during the next lint pass.

---

## Special Files

### index.md

Content-oriented catalog of every wiki page. Updated on every ingest.

```markdown
# Wiki Index

## Systems
- [Orchestration](systems/orchestration.md) — How mana dispatches units to agents.
- [Session Memory](systems/session-memory.md) — How imp manages conversation history.

## Concepts
- [Fail-First](concepts/fail-first.md) — Verify must fail before unit creation.

## Decisions
- [Worktree Isolation](decisions/worktree-isolation.md) — Why agents use git worktrees.

## Playbooks
- [Debugging a Stuck Run](playbooks/debugging-stuck-run.md) — Steps when mana run hangs.

## Areas
- [commands/close.rs](areas/commands-close.md) — Largest command, verification + hooks.

---
Last updated: 2026-04-07 | 12 pages | 8 sources cited
```

The agent reads index.md first when answering wiki queries, then drills
into relevant pages. At moderate scale (~100 pages) this is sufficient
without embedding-based search.

### log.md

Chronological, append-only record of wiki maintenance events.

```markdown
## [2026-04-07] ingest | Unit 78 closed: worktree isolation
- Created: decisions/worktree-isolation.md
- Updated: systems/orchestration.md (added worktree section)
- Updated: index.md

## [2026-04-07] lint | Periodic health check
- Flagged: concepts/fail-first.md has no inbound links
- Flagged: fact 112 (cited in systems/orchestration.md) is stale
```

Parseable with `grep "^## \[" .mana/wiki/log.md | tail -5`.

---

## Operations

### Ingest

**Trigger:** Something important happened — a unit closed, a fact was
created, a design doc was added, a failure pattern repeated.

**Flow:**
1. Agent reads the source material (closed unit, new fact, design doc).
2. Agent reads `index.md` to find existing relevant pages.
3. Agent decides: create new page, update existing pages, or both.
4. For each affected page:
   - Update content with new information.
   - Add/update source citations in frontmatter.
   - Update `last_updated`.
   - Adjust `confidence` if new evidence changes certainty.
   - Update `related_pages` cross-references.
5. Update `index.md` with any new pages.
6. Append entry to `log.md`.

**When to ingest:**
- After closing a non-trivial unit (not every typo fix).
- After creating or reverifying an important fact.
- After adding or updating a design document.
- When a failure repeats across multiple attempts (pattern worth recording).

**When NOT to ingest:**
- Trivial one-line fixes.
- Work-in-progress that may change before completion.
- Raw conversation transcripts.

### Query

**Trigger:** Agent or user asks a question that the wiki might answer.

**Flow:**
1. Agent reads `index.md` to find relevant pages.
2. Agent reads the relevant pages.
3. Agent synthesizes an answer with citations.
4. If the answer is valuable and reusable, it can be filed back as a new
   wiki page or section (this is one of Karpathy's key insights).

**Integration with imp runtime:**
- Interactive sessions: the agent can `read` wiki pages like any file.
- Headless dispatch: relevant wiki pages could be included in context
  prefill (designed in `.10.4`).
- Future: a dedicated query tool or slash command could search the wiki
  index and return relevant pages.

### Lint

**Trigger:** Periodic health check, or when the wiki has grown
significantly since the last lint.

**Checks:**
1. **Stale pages** — `last_updated` older than a threshold (e.g., 30 days)
   with no recent source activity.
2. **Orphan pages** — pages with no inbound links from other pages or
   index.md.
3. **Uncited claims** — pages at `confidence: high` with no sources in
   frontmatter.
4. **Stale citations** — facts cited in wiki pages that have failed
   `mana verify-facts` or gone past TTL.
5. **Missing pages** — concepts mentioned frequently in other pages but
   lacking their own page.
6. **Contradictions** — pages that make claims inconsistent with each
   other (hard to detect automatically; flag for human review).
7. **Disconnected from recent work** — pages about areas with heavy
   recent unit activity but no recent wiki updates.

**Output:** A lint report appended to `log.md` and optionally surfaced
in `mana context` (no-ID) output.

---

## What Stays in Units vs What Becomes Wiki

| Content | Where it lives | Why |
|---------|---------------|-----|
| "Fix the CSV export bug" | Mana unit | Active work item with verify gate |
| "CSV export uses streaming writes" | Mana fact | Atomic verifiable claim |
| "How the export system works" | Wiki: `systems/export.md` | Synthesized understanding |
| "We chose streaming over buffered for memory reasons" | Wiki: `decisions/streaming-export.md` | Design rationale |
| "Export fails silently on disk full" | Wiki: `areas/export.md` → Known edge cases | Learned from unit attempts |
| "Export might need compression support" | Wiki: open question in `systems/export.md` | Unverified hypothesis |

**The rule:** If it has a verify command, it is a `mana fact`. If it is
active work, it is a `mana unit`. If it is maintained understanding,
cross-referenced narrative, or design rationale, it is a wiki page.

---

## Citation Format

Sources in frontmatter use typed prefixes:

```yaml
sources:
  - file: src/commands/run/ready_queue.rs
  - unit: "78"
  - fact: "112"
  - doc: ARCHITECTURE.md
  - url: https://example.com/relevant-article
```

Inline citations in body text use markdown links:

```markdown
The ready queue selects unblocked units by priority
([ready_queue.rs](../../src/commands/run/ready_queue.rs), [unit 78](./../78-worktree-isolation.md)).
```

When citing a `mana fact`, include the fact title for human readability:

```markdown
The project uses serde_yml 0.0.12 ([fact 112: "serde_yml version"]).
```

---

## Naming Conventions

- File names: lowercase, hyphenated, descriptive. E.g., `worktree-isolation.md`.
- No numeric prefixes (unlike mana units). Wiki pages are navigated by
  topic, not by ID.
- Keep names stable. Renaming breaks cross-references.
- If a page grows too large, split it and leave a redirect note in the
  original.

---

## Gitignore and Versioning

`.mana/wiki/` should be **tracked in git** (unlike `index.yaml` which
is a local cache). The wiki is a project artifact — it should be visible
in PRs, diffable, and restorable.

Add to `.gitignore` only transient files if any are created (e.g., a
future search index cache).

---

## Bootstrap: Starting a Wiki from Scratch

For an existing project with mana history:

1. Create `.mana/wiki/index.md` and `.mana/wiki/log.md`.
2. Pick 3-5 most important subsystems or decisions.
3. Create one page each, citing existing files/units/facts.
4. Update index.md.
5. Append bootstrap entry to log.md.

This is a 15-minute task for an agent. The wiki grows from there through
normal ingest operations.

---

## Interaction with imp Runtime

### Phase 1 (no runtime changes)
- Wiki pages are plain markdown files. The agent reads them with the
  `read` tool like any other file.
- The agent discovers pages by reading `.mana/wiki/index.md`.
- No special prompt assembly or tool integration needed.

### Phase 2 (designed in .10.4)
- `builder.rs` could load relevant wiki pages based on working context
  (cwd, active files, mana task paths) and inject them into the system
  prompt or context prefill.
- Stale wiki pages (flagged by lint) could appear in `mana context`
  warnings alongside stale facts.
- A future `/wiki` slash command or `mana wiki query` could provide
  structured access.

### Phase 3 (designed in .10.5)
- TUI surfaces could show wiki-derived summaries between turns.
- The sidebar could distinguish wiki knowledge from raw mana status.

---

## Summary

The mana wiki is a `.mana/wiki/` directory of LLM-maintained markdown
pages with YAML frontmatter. It sits on top of mana's existing units and
facts as a **synthesized knowledge layer**. Every strong claim cites
sources. Trust tiers (verified → synthesized → hypothesis) prevent
confident sludge. Three operations maintain it: ingest (after important
events), query (when questions arise), and lint (periodic health check).

The wiki starts as plain files that agents read and write. Runtime
integration comes later, driven by real usage.
