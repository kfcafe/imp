---
id: '253'
title: Map current Tower imp+mana foundation against the 4-part SuperProduct core and sequence bridging work
slug: map-current-tower-impmana-foundation-against-the-4
status: open
priority: 1
created_at: '2026-04-14T08:09:12.239012Z'
updated_at: '2026-04-14T08:22:59.310812Z'
acceptance: A root architecture container exists for the SuperProduct alignment work, with child jobs that let another worker map current Tower reality to each of the 4 core layers and identify the narrowest bridging work needed for direct alignment.
notes: |-
  ---
  2026-04-14T08:22:59.310801+00:00
  Concrete bridge sequence from the 2026-04-14 follow-up question should be preserved for future execution: moving Tower from proto-runtime to direct 4-core alignment likely requires (1) an explicit scope decision that Tower itself should become the generalized product substrate rather than only the software-work runtime beneath it; then four bridge tracks — SuperState: evolve mana from unit/fact-centric durable work records into a typed Object/Assertion/Event/Rule object graph with universal scope/provenance/time/version dimensions while preserving file-native/workflow strengths; SuperAction: introduce stable inspectable action contracts over that shared state, with imp tools/runtime verbs becoming adapters/executors beneath product actions rather than the actions themselves; SuperComp: define a declarative versioned module/package contract that can add object vocab, workflows, projections/views, action affordances, permissions presets, and integration bindings without free-form product forks; SuperShell: treat imp CLI/TUI/viewer surfaces as an operator-shell proving ground but add a presentation-neutral shared-state surface model so shells render objects/queues/workflows/approvals/modules coherently rather than only agent runtime state. Cross-cutting bridge rules: keep mana as canonical durable truth, keep imp as live execution/runtime, keep final prompt/session behavior in imp, and migrate by adding canonical object/action/module/surface contracts first, then projecting current coding-agent-specific structures onto them rather than replacing everything at once.
labels:
- architecture
- tower
- imp
- mana
- superproduct
- planning
verify: cd /Users/asher/tower && test -f docs/architecture/tower-superproduct-alignment.md && rg -q 'SuperState' docs/architecture/tower-superproduct-alignment.md && rg -q 'SuperAction' docs/architecture/tower-superproduct-alignment.md && rg -q 'SuperComp' docs/architecture/tower-superproduct-alignment.md && rg -q 'SuperShell' docs/architecture/tower-superproduct-alignment.md && rg -q 'runtime foundation' docs/architecture/tower-superproduct-alignment.md
produces:
- docs/architecture/tower-superproduct-alignment.md
kind: epic
---

Goal: externalize a repo-grounded alignment and gap map between the current Tower stack (`mana` + `imp`) and the 4-part SuperProduct core defined in `/Users/asher/agents/PROJECT.md` and `/Users/asher/agents/spec.md`.

Current state:
- current Tower docs (`/Users/asher/tower/VISION.md`, `mana/README.md`, `mana/ARCHITECTURE.md`, `imp/README.md`) position `mana` as durable substrate/control plane and `imp` as the live runtime/shell.
- this aligns strongly with the runtime/substrate stance in `PROJECT.md §1.4`, but only partially with the product-owned layers `SuperState`, `SuperAction`, `SuperComp`, and `SuperShell`.
- repo-grounded assessment from the 2026-04-14 discussion: `mana` is closest to a proto-SuperState for software-work autonomy; `imp` is closest to proto-SuperAction plus proto-SuperShell; current extension seams are only a partial proto-SuperComp.
- the main gap is that Tower still models autonomous software work and agent orchestration more than a general Object/Assertion/Event/Rule operational substrate with a stable product-level action and module system.

Steps:
1. Write a synthesis note that maps current Tower surfaces to the 4-part core and preserves the current scorecard/alignment claims with file citations.
2. Decompose the work into one child job per core layer: SuperState, SuperAction, SuperComp, SuperShell.
3. For each layer, capture: what Tower already has, what is merely runtime-shaped, what is missing, and the narrowest bridging work that would move Tower toward direct alignment.
4. Sequence the resulting bridge work so future product/runtime planning can reference mana instead of re-deriving the gap from chat.

Files:
- /Users/asher/agents/PROJECT.md (read — 4-part core and runtime/product split)
- /Users/asher/agents/spec.md (read — substrate/capability/module/shell framing)
- /Users/asher/tower/VISION.md (read — current Tower stance)
- /Users/asher/tower/mana/README.md (read — mana product stance)
- /Users/asher/tower/mana/ARCHITECTURE.md (read — current implementation shape)
- /Users/asher/imp/README.md (read — current runtime/shell shape)
- /Users/asher/tower/docs/rebuild (read — current runtime contract/migration direction)

In scope:
- repo-grounded alignment map
- gap decomposition by SuperState/SuperAction/SuperComp/SuperShell
- sequencing of bridging architecture work

Out of scope:
- implementing the general business superproduct itself
- forcing Tower to become the business product directly without an explicit decision
- speculative claims not grounded in inspected docs/code

Do not:
- collapse the runtime/substrate layer into the product layer without stating the tradeoff
- treat current coding-agent-specific structures as already equivalent to the general business ontology
- invent a finished SuperComp module system or SuperShell product surface without mapping from existing Tower reality
