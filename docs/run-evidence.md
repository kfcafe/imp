# JSONL-first run evidence

Status: nightly slice for workflow epic 432

imp records run evidence as structured JSONL first, with optional human views derived from that data.

## Storage

```text
~/.imp/runs/
  index.jsonl
  run_<id>/
    events.jsonl
    evidence.html
```

- `events.jsonl` is the per-run source of truth.
- `index.jsonl` is an append-only summary for discovery and can be rebuilt from run directories later.
- `evidence.html` is a generated local viewer for humans.
- Markdown evidence packets are not part of this nightly slice; if needed later they should be generated/exported from JSONL rather than appended to chat.
- SQLite is intentionally not used for v1. Local JSONL scanning is simpler, inspectable, and good enough for nightly scale.

## UX

Evidence must not spam normal chat. The user should discover evidence explicitly through commands such as:

```sh
imp evidence list
imp evidence latest
```

The TUI may surface evidence in status/sidebar affordances, but should not append `Evidence: <path>` to the transcript for routine runs.

## Event model

Each JSONL line is a versioned event with:

- `run_id`
- `timestamp`
- `kind`
- optional turn/tool/status/summary fields

The schema is intentionally compact for nightly. Workflow-branch work can extend it with richer policy, verification, diff, and workflow references.
