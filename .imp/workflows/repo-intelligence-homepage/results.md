# Repo intelligence homepage

Status: implemented, pending final workflow closeout

## Shipped

- Documented a lightweight repo intelligence contract for homepage, scan, and agent context use.
- Integrated a vendored whatlang-style repository inventory scanner for homepage stats without adding a tokei dependency.
- Added tree-sitter-backed repo indexing summaries for symbol and test counts.
- Surfaced compact repo intelligence on the TUI homepage.
- Added compact repo intelligence to agent context assembly so coding turns know the current repo root, primary language, indexed files, symbols, and tests.
- Improved `scan search`, `scan related`, and `scan impact` with repo-intelligence symbol/test counts in text output and structured details.

## Scan-tool quality note

The scan integration intentionally reuses the symbol index those scan actions already build. It does not add a second parse/index pass, so the existing scan tool should not regress from duplicate tree-sitter work.

## Verification

Passed:

- `cargo check -p imp-core`
- `cargo check -p imp-tui`
- `cargo test -p imp-core scan_repo_intelligence_counts_reuse_symbol_index_shape --lib`
- `cargo test -p imp-core repo_intelligence --lib`

## Known concern

Homepage visual review remains pending. The implementation compiles and the homepage data path was reviewed, but a hands-on TUI visual smoke pass has not been completed in this closeout.
