# soul.md design and migration plan

Date: 2026-04-05
Status: proposed

## Goal

Move imp personality from config-backed profile/sliders to a `soul.md` source-of-truth document while keeping a builder UI that can edit stable generated lines.

## Core decisions

- `soul.md` is the canonical personality document.
- The builder is a view/editor over `soul.md`, not a separate source of truth.
- The document stays markdown and human-first, inspired by OpenClaw's SOUL.md style.
- Builder-managed tunables live in one recognizable section.
- Each tunable corresponds to exactly one stable line.
- Matching is exact only: if a tunable line no longer matches a generated preset, the builder shows `edited`/`mod`.
- If the user changes an edited tunable in the builder, imp shows a confirmation with the exact diff that will be applied.

## Recommended document shape

```md
# Soul

This is who I am and how I should show up.

## Tunables

- Autonomy: Act independently by default and ask when blocked, uncertain, or facing a consequential decision.
- Brevity: Keep responses brief and focused on progress.
- Caution: Prefer small, reversible changes and verify assumptions before riskier actions.
- Warmth: Use a warm, supportive tone without becoming verbose.
- Planning: Think through structure and likely consequences before acting.

## Notes

I should feel capable, grounded, and helpful.
I should not sound corporate or performatively agreeable.
```

The builder owns only the `## Tunables` lines. The rest of the file remains freeform.

## Initial tunable labels

Start with a broader set than the current five sliders:

1. Autonomy
2. Brevity
3. Caution
4. Warmth
5. Planning
6. Candor
7. Thoroughness
8. Initiative
9. Precision
10. Playfulness
11. Deference
12. Skepticism
13. Teaching
14. Decisiveness
15. Formality

Each label should have a small set of generated variants (for example low / medium / high, or another compact preset set where appropriate), but each remains exactly one emitted line.

## Integration points in current imp

Current personality system:
- `crates/imp-core/src/personality.rs`
  - current identity words and 5 slider bands
- `crates/imp-core/src/config.rs`
  - stores `Config.personality`
- `crates/imp-core/src/builder.rs`
  - passes personality into prompt assembly
- `crates/imp-core/src/system_prompt.rs`
  - renders identity sentence and slider-derived working style lines
- `crates/imp-tui/src/views/personality.rs`
  - current personality overlay/editor
- `crates/imp-tui/src/app.rs`
  - saves/deletes personality profiles via config

## Prompt assembly plan

### Phase 1: additive support

Add soul loading without deleting the current system yet.

- Load `soul.md` from:
  - global path: `~/.imp/soul.md`
  - optional project override: `<repo>/.imp/soul.md`
- If present, inject it as a personality/soul layer in prompt assembly.
- Keep existing config personality support temporarily so migration is non-breaking.

Short-term prompt behavior:
- if `soul.md` exists, prefer it
- if not, fall back to current config personality

### Phase 2: canonical switch

- Make `soul.md` the only canonical personality source.
- Keep config only for unrelated settings.
- Remove config-backed personality prompt assembly.

## Builder behavior plan

The builder should stop editing config personality fields directly and instead operate on the `## Tunables` section in `soul.md`.

### Read path

1. Load `soul.md`.
2. Find the `## Tunables` section.
3. For each known label, read the matching bullet line.
4. Compare the line text against known generated variants.
5. Show:
   - preset value if exact match
   - `edited` if not exact match
   - unset/default if missing

### Write path

When the builder changes a tunable:
1. Generate the replacement line for that label.
2. If the current line is preset-matched, replace directly.
3. If the current line is `edited`, show a confirmation dialog with the exact diff.
4. Apply only the changed line.

The builder should preserve all freeform text and all other tunables untouched.

## Edited/mod state

No complex metadata is needed.

Rule:
- exact known line → preset
- anything else for that label → `edited`

This is intentionally simple and honest.

## Migration plan

### Step 1
Add `soul.md` loader + prompt integration behind fallback logic.

### Step 2
Add a generated default `soul.md` using the current default identity + slider lines as seed content.

### Step 3
Update `/personality` UI to:
- use `soul.md` as source of truth
- show Builder and Source tabs
- read/write the `## Tunables` section
- show `edited` for modified generated lines

### Step 4
Add overwrite confirmation with diff preview when changing an edited tunable.

### Step 5
Deprecate config-backed personality storage and profile management.

## Suggested simplifications

To avoid overengineering in the first implementation:

- do not add frontmatter
- do not add hidden IDs or metadata
- do not make one tunable control multiple lines
- do not add reset/recovery systems yet
- do not build fuzzy matching

## Open questions worth deciding during implementation

1. Exact file precedence: project soul vs global soul vs merge behavior.
   - simplest answer: project soul fully overrides global soul for the session.
2. Whether the builder should manage only `## Tunables`, or also a single top identity sentence line.
   - simplest answer: start with tunables only.
3. Whether all tunables share the same preset count.
   - simplest answer: no requirement; allow per-label preset sets.

## Recommendation

Implement the smallest viable version first:

- a real `soul.md`
- one builder-managed `## Tunables` section
- one line per tunable
- exact-match `edited` detection
- diff-confirm overwrite flow
- prompt reads the file directly

This keeps the system elegant, inspectable, and aligned with the principle that the text document is the truth.