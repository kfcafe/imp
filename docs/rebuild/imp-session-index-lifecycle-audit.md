# Session Index Lifecycle Audit

This audit resolves workflow unit `264.4`: determine whether imp indexes saved sessions during normal runtime lifecycle and define the smallest correct ownership seam for reliable `session_search`.

## Inspected files

- `crates/imp-core/src/session_index.rs`
- `crates/imp-core/src/tools/session_search.rs`
- `crates/imp-core/src/session.rs`
- `crates/imp-core/src/imp_session.rs`
- `crates/imp-cli/src/lib.rs`
- `crates/imp-tui/src/app.rs`
- `crates/imp-core/src/storage.rs`
- `docs/rebuild/imp-session-storage-search-recovery-audit.md`

## Current index implementation

`crates/imp-core/src/session_index.rs` defines `SessionIndex`:

- `SessionIndex::open(path)` creates parent dirs, opens SQLite, creates `sessions`, and creates `session_content` FTS5 table.
- `SessionIndex::index_session(&SessionWorkflowger)` extracts searchable text from user/assistant text blocks and compaction summaries.
- Tool results are intentionally skipped as too noisy.
- Re-indexing is idempotent: metadata is upserted and old FTS content is deleted/reinserted.
- `SessionIndex::search(query, limit)` queries FTS and returns snippets plus first-message/session metadata.
- Tests cover create/search, no results, multiple sessions, idempotence, `is_indexed`, FTS boolean query behavior, and snippets.

The index implementation itself is usable.

## Current search implementation

`crates/imp-core/src/tools/session_search.rs` exposes the `recall` tool:

- resolves an index DB path through `index_db_path()`;
- if the file does not exist, returns a text message saying no sessions are indexed;
- opens `SessionIndex` and performs FTS search;
- renders matching session snippets.

Current index lookup order:

1. `storage::existing_global_file(storage::global_session_index_path, "session_index.db")`
2. legacy data roots with `root.join("session_index.db")`
3. fallback to `storage::global_session_index_path()`

Current canonical path from `storage.rs`:

```text
~/.imp/indexes/session_index.db
```

## Production lifecycle wiring finding

Repo search found production creation/list/open of sessions in CLI and TUI, but no production callsite that indexes sessions during normal lifecycle.

Observed non-test callsites:

- `SessionWorkflowger::new`, `open`, `continue_recent`, `list`, and `in_memory` appear in `imp-cli`, `imp-tui`, and `imp-core` session runtime paths.
- `SessionIndex::index_session(...)` appears only in `session_index.rs` tests and `session_search.rs` tests.
- `SessionIndex::open(...)` appears only in the `recall` search tool and tests.

Conclusion: indexing is currently unwired in production. Raw sessions can be saved successfully while `session_search` remains permanently unavailable unless an index is created through tests or manual code.

## Relationship to path topology work

This is adjacent to but distinct from broad storage topology:

- Current canonical raw sessions root is `~/.imp/sessions`.
- Current canonical index root is `~/.imp/indexes/session_index.db`.
- Historical stores such as `~/.local/share/imp/sessions` may still contain transcripts.
- No index DB was found on this machine during `50.16.5.1` inspection.

Path normalization is mostly addressed elsewhere. The remaining reliability issue here is lifecycle/index bootstrap.

## Smallest correct ownership seam

The smallest correct seam is a session-indexing service in `imp-core` that can be invoked explicitly from CLI/TUI/runtime boundaries without embedding index logic in each surface.

Proposed imp-core API:

```rust
pub fn index_session_file(path: &Path) -> Result<IndexSessionOutcome>
pub fn index_session_workflowger(session: &SessionWorkflowger) -> Result<IndexSessionOutcome>
pub fn rebuild_global_session_index() -> Result<SessionIndexRebuildReport>
```

Expected behavior:

- `index_session_workflowger` indexes the active session into `storage::global_session_index_path()` if the session has a persisted path.
- in-memory sessions are skipped with a clear outcome.
- `index_session_file` opens a raw `.jsonl` session and indexes it.
- `rebuild_global_session_index` scans `storage::global_sessions_dir()` plus legacy session dirs and indexes all valid `.jsonl` sessions.
- errors are returned to callers; no silent background indexing.

This keeps SQLite/FTS lifecycle in `imp-core`, while CLI/TUI decide when to invoke it.

## Recommended first implementation slice

Do not add automatic background indexing everywhere at once.

First slice:

1. Add an `imp-core` helper module around `SessionIndex` for explicit indexing/rebuild.
2. Add focused unit tests with temp raw sessions:
   - index one persisted session workflowger;
   - skip in-memory session;
   - rebuild indexes multiple session files;
   - invalid/unreadable files are reported without aborting all good files.
3. Improve `session_search` missing-index output so it mentions raw transcript directories and rebuild/indexing rather than implying no conversations exist.
4. Add a CLI-local or tool-local manual reindex command only after the helper exists.

Candidate verify gate:

```sh
cargo test -p imp-core session_index -- --nocapture
cargo test -p imp-core session_search -- --nocapture
cargo check -p imp-cli -p imp-tui
```

## Later lifecycle hooks

After the explicit helper is proven, add production hooks cautiously:

- index after successful assistant turn persistence;
- index on session close/shutdown where available;
- index after compaction writes a summary;
- offer manual `:sessions reindex` / CLI command for recovery;
- make `recall` suggest reindex when raw sessions exist but the DB is missing.

TUI and CLI should call the same imp-core helper. They should not each own their own indexing logic.

## Failure and UX rules

- Missing index must not imply missing raw sessions.
- Indexing failure must not block saving a session transcript.
- Search should fail helpfully with raw session locations and reindex guidance.
- Rebuild should produce a report: indexed count, skipped count, failed count, and representative errors.
- Legacy raw session stores are migration inputs, not canonical new write destinations.

## Conclusion

`SessionIndex` is implemented and tested, but production lifecycle wiring is absent. The smallest reliable fix is an explicit imp-core indexing/rebuild helper plus improved missing-index messaging, followed by a manual reindex command and only then automatic lifecycle hooks.
