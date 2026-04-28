# Dirac-inspired code tools v1

Status: v1 contract after first implementation slice.  
Scope: imp native tool surface for code orientation, reads, and edits.

## Goal

Adopt the highest-value Dirac primitives without cloning Dirac's IDE-first architecture:

- structural code orientation before broad reading;
- precise symbol/block extraction;
- stale-safe anchored edits;
- batched edit transactions with checkpoint evidence;
- repeatable benchmark evidence.

The model-facing surface should stay small. For v1, improve existing tools instead of adding more tool names.

## Canonical tool surface

### `scan`

Use `scan` for code structure, not literal search.

Actions:

- `scan`: outline a directory as compact code skeletons.
- `build`: outline specific files as compact code skeletons.
- `extract`: return exact code blocks from target forms:
  - `file#symbol`
  - `file:start-end`
  - `file:line`

Expected output:

- compact skeletons with symbol kind/name/signature and source line ranges;
- extraction blocks with structured details including path, symbol, kind, language, start_line, end_line, and truncation state where available;
- enough context to decide what to read/edit next without dumping full files.

Use `scan` before broad text search when the question is about symbols, definitions, file shape, or coherent code blocks. Use shell/`rg` for exact string search.

### `read`

Use `read` for stable line-oriented file reads.

V1 additions:

- optional `anchors: true` emits session-local per-line anchors;
- anchors are integrity markers for later edits, not security tokens;
- reads still support offset/limit and stable line output.

Anchors should be requested when the next action is likely to replace a range and stale-file safety matters.

### `edit`

`edit` is the canonical model-facing edit tool.

Modes:

1. Exact/fuzzy single replacement:
   - `path`
   - `oldText`
   - `newText`

2. Anchored range replacement:
   - `path`
   - `anchorStart`
   - optional `anchorEnd`
   - `replacement`

3. Transaction edits:
   - `edits: [{ path?, oldText, newText }]`
   - optional top-level `path` as default path
   - `dryRun`

Required behavior:

- validate before writing;
- reject stale anchors;
- reject overlapping exact edits;
- write no partial transaction on validation failure;
- checkpoint touched files before writes;
- return combined diff and structured transaction evidence.

### `multi_edit`

`multi_edit` is legacy compatibility only.

Direction:

- do not present it as a preferred peer to `edit`;
- keep old calls working through alias/shim while compatibility matters;
- work toward removing separate model-facing exposure;
- keep transaction implementation behind `edit` as the canonical surface.

## Naming strategy for `scan`

Decision: keep `scan` for v1.

Rationale:

- It already exists and works.
- It now clearly describes tree-sitter-backed structural code inspection.
- Renaming to `search` would blur the distinction from literal text search.
- Renaming to `codesearch` may help discoverability but is unnecessary churn before eval evidence.

Compatibility rule:

- Do not remove or rename `scan` without an alias period.
- If eval transcripts show repeated misuse, consider adding a compatibility alias such as `codesearch` that routes to `scan`.
- Any alias should preserve one conceptual surface: structural code orientation, not general grep.

Decision rule:

- Keep `scan` if models reliably choose it for skeletons/symbols and `rg`/shell for literals.
- Add alias/help text if models underuse it.
- Rename only if benchmark evidence shows the name materially hurts task success and compatibility can be preserved.

## Non-goals for v1

Out of scope:

- new model-facing tools named `symbol_extract`, `code_skeleton`, `symbol_skeleton`, or `anchor_edit`;
- full LSP integration;
- semantic rename/replace-symbol writes;
- references/definition/hover/signature help;
- diagnostic repair loops;
- IDE-specific assumptions;
- broad tool registry refactors.

These may be future phases, but v1 keeps `read`, `scan`, and `edit` as the durable vocabulary.

## Relationship to Dirac

Dirac's strongest inspected advantages were:

- symbol-native reading;
- file skeletons;
- anchor-backed editing;
- batch edit discipline;
- benchmark reporting.

imp's v1 answer:

- `scan` covers skeleton and symbol/block extraction;
- `read` emits optional anchors;
- `edit` consumes anchors and performs transactions;
- checkpoint machinery provides recovery evidence;
- `evals/dirac-comparison` captures repeatable benchmark runs.

## Benchmark plan

Use `evals/dirac-comparison` to run the same public task set Dirac reported.

Benchmark discipline:

1. Pin each upstream repo commit in a task spec.
2. Before every run: `git reset --hard && git clean -fd`.
3. Run one agent at a time in an isolated checkout.
4. Capture prompt, provider/model, command, transcript, diff, verifier output, pass/fail, and token/cost usage if available.
5. Do not claim pass without verifier evidence.
6. Compare failures qualitatively against tool-use transcripts: did the model use `scan`, `read` anchors, and `edit` transactions appropriately?

Initial target: run the same eight Dirac tasks from `github.com/dirac-run/dirac/evals/README.md`, starting with one smoke task (`datadict`) before a full matrix.

## Verification evidence from implementation slice

Implemented before this spec was written:

- `scan` skeleton/symbol output improvements with Rust/TypeScript tests.
- `read` opt-in anchors and `edit` anchored replacement.
- `edit` transaction behavior via `edits[]`, including multi-file support and dry-run.
- `multi_edit` collapsed toward `edit` as canonical model-facing tool with compatibility alias/shim.
- Dirac-derived eval harness scaffold under `evals/dirac-comparison`.

Future changes should update this doc when the contract changes, not just the code.
