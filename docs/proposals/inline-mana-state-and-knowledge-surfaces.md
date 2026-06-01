# Inline Mana State and Knowledge Surfaces

> Proposal for `.10.5` — April 2026
>
> Defines how imp should surface mana work state and synthesized
> knowledge between turns in the TUI and runtime. Maps onto existing
> work in `27.2` and `27.3`.
>
> Depends on: `.10.4` (runtime context read path)

---

## Design Principle

Two fundamentally different things need surfacing:

1. **Work state** — what mana jobs exist, what is running, what just
   completed or failed. This changes within a session.
2. **Knowledge context** — what the project wiki and facts say about the
   area being worked on. This is stable within a session.

These should use different visual treatments. Work state is ephemeral
and action-oriented. Knowledge context is reference material.

---

## Existing Surfaces

### What already works

| Surface | Where | What it shows |
|---------|-------|---------------|
| Sidebar mana formatting | `views/sidebar.rs` | Expanded mana tool call details (create params, status results, run config) |
| Tool call summary | `views/tools.rs` | Compact one-line mana action summaries (`format_mana_args`) |
| Terminal bell | `app.rs` | Audible notification when agent run completes |
| Compaction display | `views/tree.rs` | Compaction summary nodes in session tree |

### What is planned but not built

| Unit | Surface | Status |
|------|---------|--------|
| `27.2` | Compact mana status/progress widget | Open, blocked on `27.1` + `30` |
| `27.3` | Non-blocking mana follow-ups and message delivery | In progress, 1 abandoned attempt |

---

## Proposed Surfaces

### Surface 1: Mana status row (extends `27.2`)

A single compact status row displayed between turns when mana work is
active. Not a sidebar — a thin inline banner that appears above the
editor when relevant.

**When visible:**
- After imp creates a mana unit during conversation.
- While background mana work is running.
- After a mana run completes (success or failure).

**Content:**
```
┌─ mana ────────────────────────────────────────┐
│ ● 3 units ready  ▶ 2 running  ✓ 1 just closed │
└───────────────────────────────────────────────┘
```

Or when no work is active:
```
(no mana status row shown)
```

**Behavior:**
- Updates after each mana tool call returns.
- Auto-dismisses when no open/running units remain.
- Clicking (mouse) or a keybind could expand to full `mana status` output.

**Relationship to `27.2`:** This IS `27.2` with a concrete spec. The
existing unit's acceptance criteria (compact area, transition updates,
concise transcript) map directly. The main refinement: it's a status row,
not a persistent sidebar widget.

### Surface 2: Background completion notification (extends `27.3`)

When a background mana run completes, surface a non-intrusive
notification between turns rather than injecting into the transcript.

**When visible:**
- After a mana unit dispatched in the background closes (success or failure).
- Between turns only — never interrupts active streaming.

**Content:**
```
┌─ mana ──────────────────────────────────────┐
│ ✓ .10.3 closed: Strengthen mana-first prompt │
│ ✗ .10.4 failed: verify non-zero (attempt 2)  │
└──────────────────────────────────────────────┘
```

**Behavior:**
- Queued during agent execution, displayed after the turn finishes.
- Dismissed after the user's next input.
- Failure notifications persist until acknowledged or the unit is retried.

**Relationship to `27.3`:** This is the "completion-notification path"
that `27.3` calls for. The existing unit's acceptance criteria (prompt
return, user messages still reach agent, completion arrives through
notification) map directly. The refinement: completion notifications
are visual between-turn banners, not injected messages.

### Surface 3: Knowledge context hint (new — future)

When the agent is working in an area that has relevant wiki pages, show
a subtle hint between turns.

**When visible:**
- At session start if wiki pages exist for the working directory.
- After the agent reads or writes files in an area with wiki coverage.

**Content:**
```
┌─ wiki ────────────────────────────────────────┐
│ 📄 systems/orchestration.md — updated 2d ago   │
│ 📄 concepts/fail-first.md — updated 5d ago     │
└────────────────────────────────────────────────┘
```

**Behavior:**
- Read-only hint. No interaction required.
- Shows at most 3 most-relevant pages.
- Dismissed after the first turn (the agent has the index in its prompt).

**Relationship to `27.2`/`27.3`:** This is new work, not an extension of
either. It should be a separate follow-on unit after the wiki layer
exists and the status row infrastructure is proven.

### Surface 4: Sidebar enrichment (enhance existing)

The existing sidebar already formats mana tool output. Enrich it with:

- **After `mana create`:** Show the created unit's title, ID, and verify
  command in the sidebar detail.
- **After `mana status`/`mana list`:** Show a structured summary instead
  of raw text.
- **After `mana run`:** Show which units were dispatched and their
  current state.

This requires no new infrastructure — just better formatting in
`format_mana_output()` in `views/sidebar.rs`.

---

## What Should NOT Be Built

1. **No persistent mana dashboard panel.** imp's TUI is a conversation
   interface, not a project management tool. Mana state surfaces between
   turns and in the sidebar, not as a permanent panel.

2. **No wiki browser.** The agent reads wiki pages with the `read` tool.
   A built-in wiki browser would be scope creep.

3. **No real-time progress bars for agent dispatch.** Background agents
   are managed by mana, not imp's TUI. Completion notifications are
   enough.

4. **No automatic mana actions.** The TUI shows state; it does not
   autonomously create, close, or retry units. The agent makes those
   decisions.

---

## Implementation Phases

### Phase 1: Status row + sidebar enrichment

**Extends:** `27.2`
**Requires:** Basic `set_widget` / `set_status` infrastructure (from unit `30`)
**Scope:**
- Mana status row between turns (Surface 1).
- Richer sidebar formatting for mana tool results (Surface 4).
- No background work handling yet.

**Files:**
- `crates/imp-core/src/tools/mana.rs` — emit status metadata in tool output.
- `crates/imp-tui/src/app.rs` — render status row between turns.
- `crates/imp-tui/src/views/sidebar.rs` — enrich `format_mana_output()`.

### Phase 2: Background completion notifications

**Extends:** `27.3`
**Requires:** Non-blocking mana follow-up infrastructure.
**Scope:**
- Completion notification banners (Surface 2).
- Queue notifications during agent execution, display after turn.

**Files:**
- `crates/imp-core/src/agent.rs` — notification queue for background completions.
- `crates/imp-tui/src/app.rs` — render notification banners between turns.

### Phase 3: Knowledge context hints

**Requires:** Wiki layer exists, wiki index loaded at session start (`.10.4` Phase 1).
**Scope:**
- Wiki page hints (Surface 3).
- Relevance scoring based on working files.

**Files:**
- `crates/imp-tui/src/app.rs` — render wiki hints.
- `crates/imp-core/src/builder.rs` or new module — wiki relevance scoring.

---

## Distinguishing Work State from Knowledge

| Signal | Type | Visual treatment | Lifetime |
|--------|------|-----------------|----------|
| "3 units ready, 2 running" | Work state | Status row, bold/active colors | Until no active work |
| ".10.3 closed" | Work state | Notification banner | Until next user input |
| ".10.4 failed" | Work state | Notification banner, warning color | Until retried or dismissed |
| "systems/orchestration.md available" | Knowledge | Subtle hint, muted color | First turn only |
| "fact 112 is stale" | Knowledge (warning) | Warning in status row or notification | Until re-verified |

Work state uses action verbs and status colors. Knowledge context uses
reference-style formatting and muted colors. This distinction should be
maintained across all surfaces.

---

## Summary

Three concrete surfaces, phased:

1. **Status row** between turns showing active mana work (extends `27.2`).
2. **Notification banners** for background completion (extends `27.3`).
3. **Wiki hints** showing relevant knowledge pages (new, future).

Plus sidebar enrichment for better mana tool output formatting (no new
infrastructure needed).

All surfaces follow the principle: show state between turns, never
interrupt streaming, dismiss naturally, and visually separate work state
from knowledge context.
