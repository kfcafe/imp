# Archive Plan

Status: draft archive plan for `tighten-imp-product-surface`.

This is a planning artifact only. No files have been moved or deleted.

## Archive principle

Keep active repo/docs focused on the launch product:

```text
TUI + one-shot + workflows + tools + skills + sessions + auth/config + Lua extensions
```

RPC may remain as advanced/optional if low-cost. ACP is future/optional.

Anything that primarily documents old experiments, compatibility layers, rebuild eras, GUI prototypes, personality/soul, mana-next, imp-work, improve/eval/prototype product concepts, or planned-but-not-shipped surfaces should move out of the active docs path.

Archive destination options:

- `~/imp-archive/...` for material we do not want in the active repo at all.
- `docs/archive/...` for historical context that should remain versioned but clearly non-current.

Because user mentioned `~/imp-archive`, prefer `~/imp-archive` for large/obsolete experiments and root clutter.

## Root files

### Keep active root

- `AGENTS.md`
- `ARCHITECTURE.md` if updated to current product boundary
- `CHANGELOG.md`
- `README.md` after tightening
- `Cargo.toml`, lock/build files, crates/scripts/config

### Archive or move out of root

Root currently contains several design/prototype/review artifacts:

- `art.html`
- `art_original.html`
- `art_test.txt`
- `art.md`
- `draft.html`
- `imp-gui-wireframe.html`
- `imp_core_plan.md`
- `IMP_DEEP_REVIEW.md`
- `IMP_REVIEW.md`
- `imp_rebuild_plan.md`
- `imp_rebuild_strategy.md`
- `SYSTEM_PROMPT_PROPOSAL.md`
- `vNext.md`
- `ontology.md`
- `imp_ontology.md`
- `ENGINEERING_GUARDRAILS.md`
- `LEARNING_LOOP_SPEC.md`
- `just-bash.md`

Recommended disposition:

- Move UI/prototype HTML/text artifacts to `~/imp-archive/ui-prototypes/`.
- Move old rebuild/review/strategy docs to `~/imp-archive/reviews-and-rebuild-plans/` unless still referenced by active docs.
- Keep `just-bash.md`, `SYSTEM_PROMPT_PROPOSAL.md`, and `ENGINEERING_GUARDRAILS.md` only if they are intentionally retained as active design context; otherwise archive under `~/imp-archive/design-notes/`.
- Keep `imp_ontology.md` only if rewritten to current vocabulary; otherwise archive it because it still references `/ask → /plan → /work → /improve` and deprecated Lua/TypeScript assumptions.

## Active docs to keep/rewrite

These docs are likely launch-relevant but need tightening against the new boundary:

- `docs/index.md`
- `docs/architecture.md`
- `docs/workflows.md`
- `docs/tools.md`
- `docs/extensions-lua.md`
- `docs/sessions.md`
- `docs/policy.md` if still real product policy
- `docs/rpc.md` if RPC remains nice-to-have launch surface
- `docs/runtime-event-state-api.md` if RPC/runtime events stay
- `docs/verification-gates.md`
- `docs/run-evidence.md` if run evidence remains workflow/runtime evidence rather than old eval/prototype framing

Required rewrites:

- README should stop presenting planned/legacy features as product surface.
- Workflow docs should remove prototype/eval as first-class product concepts and emphasize workflow artifacts/checks/results.
- Workflow profile docs should stop describing profiles as top-level task slash commands.
- RPC docs should be advanced/optional, not core if launch scope is TUI/one-shot first.

## Docs to archive or move under historical design

### Mana-next / compatibility docs

Archive or move to `docs/archive/mana-next/` or `~/imp-archive/mana-next/`:

- `docs/mana-next-compatibility-adapter.md`
- `docs/mana-next-examples.md`
- `docs/mana-next-migration-test-plan.md`
- `docs/mana-next-runtime-event-mapping.md`
- `docs/mana-next-storage-strategy.md`
- `docs/mana-next-ux.md`
- `docs/mana-next-workflow-ledger.md`
- `docs/rebuild/mana-embedding-surface-audit.md`
- `docs/rebuild/mana-imp-contract-boundary-map.md`
- mana-heavy proposals under `docs/proposals/`

Reason:

- Mana is no longer active product surface.
- Workflows are the durable primitive.
- Existing markdown data can remain, but docs should not suggest mana-next is launch direction.

### imp-work rebuild/design docs

Archive or move to `docs/archive/imp-work/`:

- `docs/design/imp-work-global-store.md`
- `docs/design/imp-work-implementation-plan.md`
- `docs/design/imp-work-mana-feature-parity.md`
- `docs/design/imp-work-mana-migration-plan.md`
- `docs/design/imp-work-mana-removal-ledger.md`
- `docs/design/droid-mission-mode-vs-imp-work-plan.md`
- docs/rebuild that assume compatibility-first imp-work/mana migration

Reason:

- Target direction does not preserve imp-work as product concept.
- Workflows replace it.

### CLI chat docs

Archive:

- `docs/rebuild/imp-cli-interactive-shell.md`

Reason:

- CLI chat is targeted for removal.

### Personality/soul docs

Archive:

- `docs/proposals/soul-md-design-2026-04-05.md`
- root/system prompt/personality proposal material if not rewritten for prompt appendices

Reason:

- `/personality`, personality sliders, and `soul.md` product concept are targeted for removal.
- Replacement is generic prompt appendices (`~/.imp/prompt.md`, `.imp/prompt.md`).

### Eval/prototype docs

Archive or rewrite:

- `docs/eval-candidates.md`
- eval/prototype sections in README/workflows docs
- `docs/child-workflow-delegation.md` sections that treat eval-candidate capture as first-class product behavior

Reason:

- Eval/prototype should become workflow evidence/artifacts or internal dev tooling, not product surfaces.

### GUI/wireframe docs

Archive or mark future:

- `docs/tui-workflow-wireframes.md`
- `imp-gui-wireframe.html`
- root `art*.html`, `draft.html`, `art.md`, `art_test.txt`

Reason:

- TUI is first-class; GUI is not launch target.
- `imp-gui` should leave default build surface.

### TypeScript/ACP/future extension docs

Move to advanced/future or archive if not current:

- `docs/typescript-extension-bridge.md`
- `docs/proposals/guest-runtime-extension-substrate.md`
- `docs/proposals/guest-runtime-implementation-plan.md`

Reason:

- Current shipped extension support is Lua.
- TypeScript extensions may be future direction but should not appear shipped.
- ACP/editor adapter docs should be roadmap/future only.

### Rebuild docs

Most `docs/rebuild/*` are historical or migration-era. Candidate archive set:

- `docs/rebuild/imp-attach-path-cutover.md`
- `docs/rebuild/imp-bounded-subagent-orchestration.md` unless rewritten as future workflow-runner plan
- `docs/rebuild/imp-durable-storage-surface-audit.md`
- `docs/rebuild/imp-machine-streamed-error-envelope.md`
- `docs/rebuild/imp-normalized-storage-contract.md`
- `docs/rebuild/imp-output-mode-contract.md`
- `docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md`
- `docs/rebuild/imp-rebuild-migration-sequence.md`
- `docs/rebuild/imp-session-index-lifecycle-audit.md`
- `docs/rebuild/imp-session-storage-search-recovery-audit.md`
- `docs/rebuild/imp-shared-ui-event-seam.md`
- `docs/rebuild/imp-workflow-feature-inventory.md` after this workflow supersedes it

Reason:

- Active docs should not look like the repo is permanently rebuilding.
- Keep only if rewritten as concise current architecture/reference docs.

## README cleanup targets

README currently includes or advertises:

- workflow prototype results
- workflow inspect → validate → run → update → events → prototype/verify → review → closeout lifecycle
- planned API-addressable workflows
- preview/planned MCP, `.imp/agents`, ACP/editor adapters, hosted sync, workflow API
- compatibility/legacy mana integration
- TypeScript/Pi extension compatibility

Recommended README launch framing:

1. What imp is.
2. TUI quickstart.
3. One-shot quickstart.
4. Workflows as durable project files.
5. Tools and skills.
6. Lua extensions.
7. Sessions/resume.
8. Auth/config/providers.
9. Optional/advanced RPC.
10. Links to archive/roadmap, not inline planned-feature list.

Remove from README primary path:

- prototype/eval as product concepts
- planned MCP/ACP/sync/agents unless under short roadmap link
- legacy mana compatibility section
- TypeScript extension claims beyond future/experimental note

## Archive process

Before moving files:

1. Decide whether the archive is outside-repo `~/imp-archive` or in-repo `docs/archive` for each group.
2. Preserve relative path metadata in archive filenames or directory structure.
3. Do not archive files that current code/docs link to unless links are updated.
4. Use `git mv` for in-repo moves; use normal `mv` for `~/imp-archive` after approval.
5. After moving docs, run a link/reference check with `rg` for stale paths.

Suggested archive layout:

```text
~/imp-archive/
  root-prototypes/
  gui/
  rebuild-plans/
  mana-next/
  imp-work/
  personality-soul/
  eval-prototype/
  future-extension-design/
```

## Risks

- Some historical docs contain useful implementation decisions; archive does not mean delete.
- Docs may be linked from README or active architecture docs.
- Moving root docs may affect agent context if AGENTS/local instructions reference them.
- `CHANGELOG.md` should not be rewritten historically; it can retain old feature mentions.

## First archive implementation slice after approval

1. Move root HTML/prototype artifacts to `~/imp-archive/root-prototypes/`.
2. Move `imp-gui-wireframe.html` and GUI docs to `~/imp-archive/gui/` or `docs/archive/gui/`.
3. Move `docs/proposals/soul-md-design-2026-04-05.md` to `~/imp-archive/personality-soul/` after personality removal plan is approved.
4. Move `docs/rebuild/imp-cli-interactive-shell.md` to `~/imp-archive/cli-chat/` after CLI chat removal is approved.
5. Rewrite README before archiving broader docs so active links remain coherent.
