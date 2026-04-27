---
id: '69'
title: imp-cli no longer contains duplicate mana-unit loading helpers after worker-boundary cleanup
slug: imp-cli-no-longer-contains-duplicate-mana-unit-loa
status: open
priority: 3
created_at: '2026-04-09T13:24:45.970573Z'
updated_at: '2026-04-24T05:35:28.469512Z'
notes: |-
  ---
  2026-04-09T13:25:07.044401+00:00
  Observed during implementation of the worker-boundary cleanup slice: imp/crates/imp-cli/src/main.rs now reads as an adapter around imp_core::mana_worker for the real imp run path and no longer exposes the stale duplicate helper set under the old names. At the time of verification, the name-removal check passed but the broader `cargo check -p imp-cli` command was blocked by unrelated existing mana-core compile errors involving missing autonomy-related types/fields.

  ---
  2026-04-16T09:41:43.937387+00:00
  2026-04-16 cleanup note: treat this fact as the canonical current record for the worker-boundary cleanup outcome. Open fact `67` duplicated the same title/verify/paths; this unit is the later and slightly richer record because it also preserved the observed compile-context note at the time of verification.

  ---
  2026-04-24T05:35:28.469503+00:00
  Graph hygiene 2026-04-24: fact remains substantively true, but verify drifted because helper names now live in `imp-core/src/mana_worker.rs` / docs rather than `imp-cli/src/main.rs`. Updated verify to keep the negative check on imp-cli while checking canonical helper names in mana_worker/docs.
labels:
- fact
verify: cd /Users/asher/imp && ! rg -n "struct ManaUnit|fn load_mana_unit|fn find_mana_dir|fn parse_mana_unit" crates/imp-cli/src/main.rs && rg -n "load_assignment_with_mana_dir|assemble_prefill|build_task_context|build_task_prompt" crates/imp-core/src/mana_worker.rs docs/rebuild/imp-worker-boundary-plan.md
kind: epic
unit_type: fact
last_verified: '2026-04-09T23:16:10.195220Z'
stale_after: '2026-05-09T23:16:10.195220Z'
paths:
- imp/crates/imp-cli/src/main.rs
- imp/crates/imp-core/src/mana_worker.rs
- docs/rebuild/imp-worker-boundary-plan.md
---
