# Archive stale history results

Status: done

## Archived paths or retain rationale

Archived stale public-confusing history to `~/imp-archive/stale-history-2026-05-29`.

Archived root planning/history files include:
- `ARCHITECTURE.md`
- `ENGINEERING_GUARDRAILS.md`
- `imp_core_plan.md`
- `IMP_DEEP_REVIEW.md`
- `imp_ontology.md`
- `imp_rebuild_plan.md`
- `imp_rebuild_strategy.md`
- `IMP_REVIEW.md`
- `LEARNING_LOOP_SPEC.md`
- `ontology.md`
- `SYSTEM_PROMPT_PROPOSAL.md`
- `vNext.md`

Archived stale docs include mana-era design/proposal/runtime docs listed in `.imp/workflows/archive-stale-history/artifacts/archive-manifest.md`.

Retained current public docs linked from README/docs index: architecture, workflows, tools, policy, RPC, sessions/evidence, Lua extensions, and ACP scaffold doc.

## Restore location

Restore from `~/imp-archive/stale-history-2026-05-29`.

## Reference verification

- Targeted `rg` for archived root/doc names under `README.md` and `docs/` returns no matches.
- `cargo fmt --check` passed.
- `cargo check --workspace` passed with pre-existing `imp-cli` ACP dead-code warnings.
