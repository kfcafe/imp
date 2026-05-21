# imp Session Storage and Search Recovery Audit

This audit resolves mana unit `50.16.5.1`: why `session_search` can report no indexed sessions even when raw session transcripts exist, and where an operator should look for recovery on this machine.

## Current code paths

Inspected files:

- `crates/imp-core/src/storage.rs`
- `crates/imp-core/src/config.rs`
- `crates/imp-core/src/imp_session.rs`
- `crates/imp-core/src/session.rs`
- `crates/imp-core/src/session_index.rs`
- `crates/imp-core/src/tools/session_search.rs`

Current canonical storage is no longer XDG-style. `storage::global_root()` resolves to:

```text
$HOME/.imp
```

Therefore current raw sessions are expected at:

```text
/Users/asher/.imp/sessions
```

Current session-search index path is expected at:

```text
/Users/asher/.imp/indexes/session_index.db
```

`session_search` (`recall`) looks for an existing global index first through `storage::existing_global_file(storage::global_session_index_path, "session_index.db")`, then legacy data roots, and finally falls back to `storage::global_session_index_path()`.

If the DB file does not exist, it returns:

```text
No sessions indexed yet. Session search becomes available after your first conversation.
```

That message means only that the FTS index is absent. It does **not** prove raw transcripts are absent.

## Local machine state on 2026-05-20

Observed raw session stores:

```text
/Users/asher/.imp/sessions
/Users/asher/.local/share/imp/sessions
```

Counts from direct inspection:

```text
/Users/asher/.imp/sessions: 539 jsonl files
/Users/asher/.local/share/imp/sessions: present; combined unique count command reported 1163 candidates across both stores
```

Latest raw sessions currently appear under `/Users/asher/.imp/sessions`, e.g.:

```text
/Users/asher/.imp/sessions/f83ea4d7-9d46-4078-afc8-0363fb3cadc7.jsonl
/Users/asher/.imp/sessions/1cd3078a-5401-4871-9422-f71ca7c19682.jsonl
/Users/asher/.imp/sessions/e94fda05-297e-4913-b016-4a99e8071179.jsonl
```

Observed index DB candidates:

```text
find /Users/asher/.imp /Users/asher/.local/share/imp '/Users/asher/Library/Application Support/imp' -maxdepth 3 -name 'session_index.db'
```

returned no `session_index.db` files.

## Diagnosis

The current recovery failure is an indexing gap plus historical storage drift, not proof that conversations are gone.

Important distinctions:

- raw session storage and session-search indexing are separate;
- `session_search` only queries the SQLite FTS index;
- absent index DB produces a misleading user-facing message;
- raw `.jsonl` transcripts can still be recovered manually from disk;
- older runs may exist under XDG-style `/Users/asher/.local/share/imp/sessions`, while current code writes to `/Users/asher/.imp/sessions`.

## Operator recovery path

When trying to recover a prior imp conversation on this Mac:

1. Check the current raw session directory first:

   ```sh
   ls -lt /Users/asher/.imp/sessions | head
   ```

2. If the target is older or missing, check the historical XDG-style store:

   ```sh
   ls -lt /Users/asher/.local/share/imp/sessions | head
   ```

3. Confirm whether search indexing exists:

   ```sh
   find /Users/asher/.imp /Users/asher/.local/share/imp '/Users/asher/Library/Application Support/imp' \
     -maxdepth 3 -name 'session_index.db' -print
   ```

4. If no index DB exists, do not rely on `session_search`. Search raw JSONL directly:

   ```sh
   rg -i 'phrase from the lost conversation' \
     /Users/asher/.imp/sessions \
     /Users/asher/.local/share/imp/sessions
   ```

5. Once candidate files are found, inspect them directly:

   ```sh
   sed -n '1,80p' /Users/asher/.imp/sessions/<session-id>.jsonl
   ```

6. A conversation is only likely absent after checking both raw stores and confirming no candidate content appears in either.

## Product/UX mismatch

The current `session_search` missing-index message is too strong. It implies search becomes available “after your first conversation,” but the inspected machine has many raw sessions and no index DB. Better behavior would say:

```text
No session search index exists yet. Raw transcripts may still exist at ~/.imp/sessions. Run or trigger session indexing to make them searchable.
```

## Follow-on fix direction

Small follow-on fixes should be tracked under the broader storage topology work (`264`):

1. Add an index bootstrap/rebuild path from raw session stores.
2. Improve `session_search` missing-index output to include raw session directories.
3. Decide whether to one-time import legacy `/Users/asher/.local/share/imp/sessions` into current `~/.imp/sessions` or teach indexing to scan both.
4. Add a test proving missing index + existing raw sessions yields a recovery-oriented message.

## Conclusion

Local conversation recovery should start from raw JSONL files, not the FTS index. On this machine, raw transcripts exist in substantial numbers, but no session index DB was found. The lost conversation should be searched for directly in `/Users/asher/.imp/sessions` and `/Users/asher/.local/share/imp/sessions` before declaring it absent.
