# mana-next UX and Progressive Disclosure

Status: UX design for imp-next  
Parent: mana `394.3` / child `394.3.8`

## Summary

mana-next should feel invisible for routine TUI work and invaluable when work becomes durable, blocked, verified, resumed, delegated, or reviewed.

The user should not need to understand mana to ask imp to fix a small issue. But when the task matters, mana should provide the durable workflow ledger: status, blockers, decisions, verification, evidence, and closeout.

## Principles

1. **TUI first.** The imp TUI is the default user surface.
2. **Progressive disclosure.** Show workflow/mana details only when useful.
3. **Automatic bookkeeping.** The agent writes durable summaries/evidence refs; users should not manually update mana after every step.
4. **No transcript spam.** mana stores workflow summaries and artifact refs, not raw logs.
5. **Recovery-oriented.** A user should be able to resume, inspect blockers, and find evidence from mana.

## What users see by default

### Simple TUI request

```text
User: fix typo in README
```

Expected UX:

- no visible mana ceremony
- maybe no workflow record unless configured
- normal answer and diff summary
- optional evidence artifact depending on settings

### Meaningful code-change request

```text
User: fix failing auth tests
```

Expected UX:

- TUI may show a compact workflow status line:

```text
Workflow: Fix failing auth tests | local-auto | executing
```

- evidence packet path appears at closeout
- verification status appears if gates exist
- mana stores durable summary/evidence refs

### Mana-backed work

```text
imp run 394.2.1
```

Expected UX:

- mana unit is the workflow/task anchor
- TUI shows unit id/title/status
- verification and evidence refs attach to the mana ledger
- closeout updates status according to verification and final result

## User-facing concepts

Keep this small:

- **Workflow** — meaningful unit of work
- **Task** — piece of work inside workflow
- **Decision** — choice/blocker needing resolution
- **Verification** — proof gate
- **Evidence** — artifact proving what happened
- **Note** — useful progress/context

Avoid exposing internal distinctions like sidecars, indices, raw traces, or storage layout unless debugging.

## Suggested commands

Existing commands remain useful:

```bash
mana list
mana show 394.2
mana verify 394.2.1
mana close 394.2.1
```

Future workflow-friendly commands could be aliases/views:

```bash
imp workflow status
imp workflow evidence
imp workflow blockers
imp workflow resume 394.2
imp workflow decisions
```

These should read the mana ledger, not replace it.

## TUI surfaces

Minimum useful TUI surfaces:

- workflow title/id
- status/phase
- autonomy mode
- current blocker if any
- verification gate summary
- evidence packet path
- unresolved decisions
- closeout status

Example closeout:

```text
DONE
Workflow: 394.2.1 Define workflow contract Rust types
Verified: cargo test -p imp-core workflow_contract --lib
Evidence: .imp/runs/run_abc/evidence.md
Mana: updated 394.2.1 -> done
```

Example blocked state:

```text
BLOCKED
Reason: required verification failed
Verify: cargo test -p imp-core workflow_contract --lib
Evidence: .imp/runs/run_abc/evidence.md
Next: inspect verify.log or rerun after fixing compile error
```

## What mana records automatically

Automatic durable summaries:

- workflow started/completed
- blocker set/cleared
- verification started/completed
- evidence packet written
- decision needed/resolved
- child workflow started/completed later
- final closeout status

Not automatic/noisy:

- raw model text deltas
- every tool call
- full stdout/stderr
- entire trace JSONL
- private scratch reasoning

## Inspecting evidence

Users should be able to find evidence through:

- TUI closeout path
- mana workflow/task show
- future `imp workflow evidence`

Evidence remains in artifacts:

```text
.imp/runs/<run-id>/evidence.md
.imp/runs/<run-id>/trace.jsonl
.imp/runs/<run-id>/verify.log
.imp/runs/<run-id>/diff.patch
```

mana stores refs and summaries.

## Decisions and blockers

When the agent needs a durable user choice:

- create or update a Decision
- mark it as blocking if needed
- show it in TUI
- resolve it when the user answers

Examples:

- add dependency?
- run destructive command?
- accept missing verification?
- apply worktree diff?

## Compatibility with current mana

Current mana remains valid:

- epics/tasks continue to appear in trees
- verify command remains a verification source
- decisions remain decisions
- notes remain notes
- facts remain facts, but imp-next should avoid creating noisy facts

workflow-ledger fields are additive.

## FAQ

### Do I need to create a mana workflow before using imp?

No. Routine TUI use should work without manual mana setup.

### When does mana matter?

When work is durable: implementation tasks, verification, blockers, decisions, evidence, delegation, or resumption.

### Does mana store every agent message?

No. Raw traces live in run artifacts. mana stores durable summaries and references.

### Can I still use current mana commands?

Yes. mana-next is designed as a compatibility layer first.

### Is this a project-management system?

No. It is a workflow ledger for agent execution: what was attempted, what proved it, what is blocked, and where the evidence lives.
