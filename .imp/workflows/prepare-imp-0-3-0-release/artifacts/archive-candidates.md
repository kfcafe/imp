# Archive candidates

Cleanup policy: archive only clearly unused/dead code and stale docs.

Evidence-backed candidates requiring focused verification before moving to `~/imp-archive/imp-0.3.0-release-candidate`:

1. Historical `.mana/*.md` work records: many stale planning artifacts are not active release docs. Archive only if repo still treats `.mana` as old history and no current workflow depends on them.
2. Stale docs that advertise preview/planned surfaces as active. Prefer editing docs over archival when the file contains current documentation.
3. Orphaned personality UI/backend leftovers identified by prior `tighten-imp-product-surface` workflow. Archive/delete only after reference scan proves no active compile path.
4. Legacy improve/mana internals: do not archive in this workflow unless compile/reference evidence proves dead; current instructions say compatibility may remain.

Current conclusion: no code should be moved yet without a narrower reference/compile pass. The first safe cleanup target is documentation wording around ACP/editor adapter and planned surfaces.
