# imp Memory Architecture and Mana Ownership Boundaries

> Proposal for `.10.1` — April 2026
>
> Defines the layered memory model for imp: what each layer owns, where
> it persists, who reads/writes it, and what should never cross boundaries.

---

## Overview

imp has four memory layers. Each serves a different temporal scope and
audience. This document names them, states their ownership rules, and
draws the boundaries that future runtime, prompt, and UI work should
respect.

| Layer | Temporal scope | Persistence | Owner (writes) | Consumers (reads) |
|-------|---------------|-------------|----------------|-------------------|
| 1. Session/runtime memory | Single conversation | JSONL session file | imp runtime | imp runtime, compaction |
| 2. Personal persistent memory | Cross-session, global | `~/.config/imp/memory.md`, `user.md` | Agent (via memory tool) | System prompt (frozen snapshot) |
| 3. Mana work memory | Project-durable | `.mana/` unit + fact files | Agent (via mana tool), orchestrator, workers | Prompt assembly, headless dispatch, `mana context` |
| 4. Synthesized project knowledge (wiki layer) | Project-durable, compounding | `.mana/wiki/` markdown pages (future) | Agent (maintenance operations) | Prompt assembly, on-demand query, human browsing |

---

## Layer 1: Session/runtime memory

**What it is.** The conversation history for a single imp session: user
messages, assistant responses, tool calls and results, compaction
boundaries, and custom entries.

**Where it lives.** Append-only JSONL files in `~/.config/imp/sessions/`.
Branch-local compaction entries replace older history with a structured
summary while preserving raw entries on disk.

**Key code surfaces:**
- `crates/imp-core/src/session.rs` — `SessionManager`, `SessionEntry`, `get_active_messages()`
- `crates/imp-core/src/imp_session.rs` — `ImpSession::prompt()`, context prefill injection
- `crates/imp-core/src/compaction.rs` — `prepare_messages_for_compaction()`, `execute_manual_compaction()`

**Write rules:**
- Only the imp runtime appends entries (messages, compactions, custom records).
- The agent never directly edits session files.

**Read rules:**
- `get_active_messages()` is the canonical model-visible history.
- Compaction replaces old prefix with a summary; raw entries remain for replay/fork/export.
- The frozen snapshot pattern: session history is assembled once per prompt call, not mutated mid-turn.

**What belongs here:**
- Conversation turns (user prompts, assistant responses, tool calls/results).
- Compaction summaries of older conversation history.
- Checkpoint and usage records.
- Session metadata (name, summary, branch structure).

**What does NOT belong here:**
- Project architecture knowledge.
- Verified project facts.
- Durable implementation plans.
- Anything that should survive beyond this conversation.

**Anti-goal:** Do not use session history as a project knowledge base. If
something is worth remembering across sessions, it belongs in Layer 2
(preferences), Layer 3 (facts/units), or Layer 4 (synthesized knowledge).

---

## Layer 2: Personal persistent memory

**What it is.** Two small, bounded markdown files that persist the
agent's personal notes and user profile across all sessions and all
projects.

**Where it lives.**
- `~/.config/imp/memory.md` (agent notes — 2,200 char limit, ~800 tokens)
- `~/.config/imp/user.md` (user profile — 1,400 char limit, ~500 tokens)

**Key code surfaces:**
- `crates/imp-core/src/memory.rs` — `MemoryStore` (load, save, add, replace, remove)
- `crates/imp-core/src/tools/memory.rs` — `MemoryTool` (agent-facing CRUD)
- `crates/imp-core/src/builder.rs` — loads memory at build time, passes to prompt assembly
- `crates/imp-core/src/system_prompt.rs` — Layer 6 injection

**Write rules:**
- The agent writes via the `memory` tool.
- Humans can edit the files directly.
- Writes persist to disk immediately but do NOT mutate the system prompt mid-session (frozen snapshot pattern, preserves LLM prefix caching).

**Read rules:**
- Loaded once at session start by `AgentBuilder::build()`.
- Injected as a static text block in Layer 6 of the system prompt.
- The tool result always shows live state so the agent knows what is currently stored.

**What belongs here:**
- User preferences and communication style.
- Environment facts (OS, shell, editor, common tools).
- Tool quirks and lessons learned that apply globally.
- Project-agnostic workflow preferences.

**What does NOT belong here:**
- Project-specific architecture knowledge (use mana facts or wiki).
- Implementation plans or work status (use mana units).
- Large or frequently changing content (the budget is ~2,200 chars total).
- Anything that requires verification (use `mana fact`).

**Anti-goal:** Do not turn `memory.md` into a project wiki. It is a
personal notepad for the agent, not a project knowledge base. If the
content is project-specific and would benefit another agent working on
the same project, it belongs in mana.

---

## Layer 3: Mana work memory

**What it is.** The durable project work graph: units of work with
verify gates, verified facts with TTL/staleness, attempt logs, notes,
dependency graphs, and archived completion records.

**Where it lives.** `.mana/` directory in the project root — YAML
frontmatter + markdown body files, plus `index.yaml` cache, `config.yaml`,
and `archive/` for closed work.

**Key code surfaces:**
- `mana` CLI and the imp-native `mana` tool (`crates/imp-core/src/tools/mana.rs`)
- `crates/imp-core/src/system_prompt.rs` — Layer 4 (mana facts), Layer 5 (task context)
- `crates/imp-core/src/builder.rs` — currently passes `facts: &[]` to assembly (gap)
- `crates/imp-cli/src/main.rs` — headless `imp run` loads unit metadata and file prefill

**Write rules:**
- Agents write via the `mana` tool (create, update, close, fact_create, etc.).
- Humans can edit `.mana/` files directly.
- Workers record progress via `mana update --note`.
- Orchestrators own unit structure, verification, and closure.

**Read rules:**
- `mana context <id>` assembles a complete briefing for a specific unit.
- `mana context` (no ID) outputs project-wide memory context (stale facts, claimed units, recent work).
- `mana recall` searches units by keyword.
- Headless `imp run` reads unit frontmatter and injects file prefill.
- System prompt Layer 4 has a `facts_layer()` slot — currently unpopulated in `builder.rs`.

**What belongs here:**
- Work units: tasks, bugs, features, epics.
- Verified project facts with shell-checkable proof (`mana fact`).
- Implementation plans decomposed into executable child jobs.
- Attempt history and failure notes.
- Dependency relationships and artifact coordination.
- Design decisions recorded on units.

**What does NOT belong here:**
- User preferences or personal agent notes (use Layer 2).
- Synthesized project understanding or summaries (use Layer 4 / wiki layer).
- Raw conversation history (stays in Layer 1).

**Current gaps:**
- `builder.rs` passes `facts: &[]` — mana facts are not yet loaded into the system prompt at session start.
- `mana context` (no ID) exists in mana CLI but is not yet wired into imp's session-start flow.
- No mechanism to surface relevant facts/units proactively based on working directory or active files.

**Anti-goal:** Do not use mana units as a freeform knowledge wiki. Units
are work items with verify gates. Facts are atomic verified claims. If
the knowledge is synthesized, cross-referenced, or narrative in nature,
it belongs in the wiki layer.

---

## Layer 4: Synthesized project knowledge (wiki layer)

**What it is.** A persistent, LLM-maintained, interlinked collection of
markdown pages that sits between raw mana artifacts and future questions.
This is the "compounding artifact" — the layer that turns closed work,
verified facts, and project evolution into maintained understanding.

Inspired by Karpathy's LLM Wiki pattern: instead of rediscovering
project knowledge from scratch each session, the agent incrementally
builds and maintains synthesized pages with cross-references, citations,
and explicit confidence levels.

**Where it will live.** `.mana/wiki/` — a directory of markdown files
with YAML frontmatter, managed by the agent, readable by humans and
tools.

**Status:** Not yet implemented. This proposal establishes the
architectural slot and ownership boundaries. The concrete schema and
workflow are defined in `.10.2`.

**Write rules (proposed):**
- The agent maintains wiki pages through structured operations (ingest, update, lint).
- Pages are never hand-edited during normal flow (the agent owns them).
- Humans can read and browse freely; manual edits are allowed but the agent may overwrite them on next maintenance pass.

**Read rules (proposed):**
- Prompt assembly may inject relevant wiki pages based on working context (similar to how facts would be injected in Layer 4).
- Headless dispatch may include relevant wiki pages alongside unit file prefill.
- On-demand query: the agent can read wiki pages like any other file when answering questions.
- The wiki index serves as the navigation entry point (like `mana recall` but for synthesized knowledge).

**Knowledge tiers within the wiki layer:**
1. **Verified facts** — atomic claims with shell proof. These live in `mana fact`, not wiki pages. Wiki pages may cite them.
2. **Synthesized pages** — maintained summaries built from facts, units, files, and design docs. Carry explicit source citations and confidence levels.
3. **Open questions / hypotheses** — useful but unproven claims, explicitly marked as tentative.

**What belongs here:**
- How systems work (architecture summaries, module overviews).
- Why decisions were made (design rationale, tradeoff records).
- What keeps breaking (failure patterns, known edge cases).
- How to do things (playbooks, debugging guides).
- What changed recently (synthesized from closed units).

**What does NOT belong here:**
- Raw work status (that is mana units).
- Conversation history (that is Layer 1).
- User preferences (that is Layer 2).
- Unverified claims presented as facts (use the hypothesis tier or `mana fact`).

**Anti-goal:** Do not let the wiki become confident sludge. Every strong
claim should either be a verified `mana fact` or cite specific sources
(files, units, facts). Unsourced claims should be explicitly marked as
hypotheses or open questions.

---

## Boundary Rules

These rules govern what crosses layer boundaries:

### Session → Mana (Layer 1 → Layer 3)
- When a conversation produces durable plans, architecture decisions, or
  implementation structure, externalize into mana units/facts during the
  conversation — do not wait until the end.
- When a unit closes, the completion record lives in mana (archived unit).
  The session transcript is not the record of completion.

### Session → Personal memory (Layer 1 → Layer 2)
- When the agent learns a durable user preference or environment fact,
  save it to `memory.md` or `user.md`.
- Do not save project-specific knowledge here.

### Mana → Wiki (Layer 3 → Layer 4)
- When important units close, update relevant wiki pages (ingest operation).
- When facts are created or reverified, wiki pages that cite them may need updating.
- The wiki cites mana artifacts; it does not replace them.

### Wiki → Prompt (Layer 4 → runtime)
- Relevant wiki pages should be discoverable at session start or task dispatch.
- Wiki content is injected as context, not as authoritative instruction.
- Stale or contradicted wiki pages should be flagged, not silently served.

### Personal memory → Prompt (Layer 2 → runtime)
- Frozen at session start. Changes during the session persist to disk
  but do not alter the current system prompt (prefix caching).

### Mana facts → Prompt (Layer 3 → runtime)
- Should be loaded at session start (currently a gap: `facts: &[]` in builder.rs).
- In headless mode, unit-specific facts should be included in task context.

---

## What Each Role Should Use

| Role | Session (L1) | Personal (L2) | Mana (L3) | Wiki (L4) |
|------|-------------|---------------|-----------|-----------|
| Full | Read/write (automatic) | Read/write | Read/write | Read; write via maintenance ops |
| Worker | Read/write (automatic) | Read only (frozen) | Read; update notes only | Read only |
| Orchestrator | Read/write (automatic) | Read only (frozen) | Read/write (full) | Read; trigger maintenance |
| Planner | Read/write (automatic) | Read only (frozen) | Read; create units/facts | Read only |
| Reviewer | Read only | Read only (frozen) | Read only | Read only |

---

## Phased Implementation

### Phase 0 (current state)
- Session/compaction: working.
- Personal memory: working (frozen snapshot).
- Mana units/facts: working but facts not loaded into prompt assembly.
- Wiki: not implemented.

### Phase 1 (near-term, no wiki)
- Wire mana facts into `builder.rs` → `facts_layer()` at session start.
- Wire `mana context` (no-ID) output into session-start context for interactive sessions.
- Strengthen prompt doctrine to distinguish the 4 layers (`.10.3`).

### Phase 2 (wiki foundation)
- Define wiki schema and maintenance operations (`.10.2`).
- Design runtime read path for wiki pages (`.10.4`).
- Design inline UI surfaces (`.10.5`).

### Phase 3 (wiki implementation)
- Create `.mana/wiki/` structure.
- Implement ingest/query/lint operations.
- Wire wiki pages into prompt assembly and headless dispatch.
- Add TUI surfaces for wiki state.

---

## Summary

> **Session memory is what is happening now.**
> **Personal memory is who the user is and how the agent works.**
> **Mana is what work exists and what has been verified.**
> **The wiki is what this project means and what we have learned.**

Each layer has a clear owner, a clear persistence model, and clear
boundaries. Project knowledge flows outward from sessions into mana and
wiki — never the reverse. The agent's job is to externalize durable
understanding into the appropriate layer during the conversation, not
after.
