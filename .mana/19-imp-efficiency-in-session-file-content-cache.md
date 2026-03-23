---
id: '19'
title: 'imp efficiency: in-session file content cache'
slug: imp-efficiency-in-session-file-content-cache
status: open
priority: 2
created_at: '2026-03-22T23:59:57.223111Z'
updated_at: '2026-03-22T23:59:57.223111Z'
verify: grep -q 'FileCache\|file_cache\|content_cache' /Users/asher/tower/imp/crates/imp-core/src/tools/mod.rs /Users/asher/tower/imp/crates/imp-core/src/tools/read.rs 2>/dev/null
---

## Problem
Every `read` tool call hits disk, even if the same file was read 2 turns ago and hasn't been modified. Agents frequently re-read the same files during a session.

## Design
Add a simple file cache to ToolContext or Agent:
- HashMap&lt;PathBuf, (SystemTime, String)&gt; — path → (mtime, content)
- On read: check mtime, return cached if unchanged
- On write/edit: invalidate cache entry for that path
- No max size needed — session-scoped, cleared on agent end

## Files
- `imp/crates/imp-core/src/tools/mod.rs` — add cache to ToolContext or separate FileCache
- `imp/crates/imp-core/src/tools/read.rs` — check cache before reading
- `imp/crates/imp-core/src/tools/write.rs` — invalidate on write
- `imp/crates/imp-core/src/tools/edit.rs` — invalidate on edit

## Acceptance
- File read twice in sequence returns cached content (no second disk read)
- File edit invalidates cache
- Tests pass
