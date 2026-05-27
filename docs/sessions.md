# Sessions and evidence

imp sessions are durable JSONL records for agent runs.

Primary implementation areas:

- `crates/imp-core/src/session.rs`
- `crates/imp-core/src/compaction.rs`
- `crates/imp-core/src/run_evidence.rs`
- `crates/imp-cli/src/lib.rs`

## Session records

Session entries include:

- header metadata
- user and assistant messages
- tool calls
- tool results
- usage metadata
- cost metadata
- branch metadata
- compaction entries
- labels/custom entries
- recovery checkpoints

## Branching

Sessions can contain branch metadata so alternate paths remain inspectable. Branching is useful when a user changes direction or when a run needs to preserve prior context without flattening it into a single transcript.

## Compaction

Compaction reduces older context while preserving a summary. The compaction entry is written to the session so later turns can see what was compacted and why.

## Evidence

Evidence surfaces include:

- verification command results
- runtime traces
- evidence packets
- workflow `results.md`
- workflow `events.jsonl`
- recovery checkpoints

CLI evidence commands:

```bash
imp evidence list
imp evidence latest
```

## Outcomes

Common terminal outcomes:

```text
DONE
DONE_WITH_CONCERNS
BLOCKED
NEEDS_CONTEXT
```

Use `DONE_WITH_CONCERNS` when the work is complete and verified but a material limitation or follow-up remains.
