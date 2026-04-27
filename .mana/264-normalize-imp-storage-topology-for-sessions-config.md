---
id: '264'
title: Normalize imp storage topology for sessions, config, skills, tools, auth, memory, and indexes
slug: normalize-imp-storage-topology-for-sessions-config
status: open
priority: 1
created_at: '2026-04-16T06:39:34.764999Z'
updated_at: '2026-04-16T06:39:34.764999Z'
labels:
- imp
- storage
- spec
kind: epic
feature: true
---

Goal: reconcile where imp stores durable user-global, user-machine, and project-local files so session recovery, extension loading, config lookup, and future migrations all follow one explicit topology instead of ad hoc path rules.

Current state:
- Current code inspection in `imp/` shows storage path logic is split across multiple modules rather than defined by one canonical storage API.
- `imp/crates/imp-core/src/config.rs` defines `Config::user_config_dir()` as XDG-style config storage and `Config::session_dir()` as XDG-style data storage (`~/.local/share/imp/sessions` fallback), even on macOS.
- `imp/crates/imp-core/src/tools/session_search.rs` independently special-cases macOS and stores the search index at `~/Library/Application Support/imp/session_index.db`, so session transcripts and the session-search index do not currently share one canonical data-root helper.
- `imp/crates/imp-core/src/resources.rs`, `tools/memory.rs`, `tools/extend.rs`, `tools/web/*`, and `imp-lua` each rely on their own path expectations for skills, prompts, soul, memory, auth, Lua extensions, and related user-global/project-local files.
- This creates product and operator confusion: sessions, indexes, skills, auth, memory, and extensions are all 'imp files', but the repo currently mixes XDG config, XDG data, project `.imp/`, and macOS-specific special cases without one durable contract.

Desired outcome:
- A source-backed inventory of every durable imp file surface.
- An explicit normalized topology that distinguishes config vs data vs cache, user-global vs project-local, and legacy compatibility lookup/migration rules.
- A central path-resolution API in imp-core that callers share instead of open-coding storage paths.
- Clear sequencing for implementation and docs so operators can reliably find sessions, indexes, auth, skills, memory, and extensions.

Decomposition:
1. Audit the current storage surface and document every durable file type, current path helper/callsite, precedence rule, and mismatch.
2. Specify the normalized storage contract and migration policy, including macOS behavior and compatibility lookup for legacy stores.
3. Implement a central imp-core storage-path API and migrate callers incrementally.
4. Repair the session/session-search mismatch as part of the broader topology rather than as an isolated one-off fix.
5. Update docs/operator guidance so humans know where imp stores each class of file.

In scope:
- imp file/storage topology
- durable user-global and project-local paths
- legacy compatibility and migration behavior
- session/index recovery reliability

Out of scope:
- changing mana graph storage
- redesigning all UX surfaces at once
- inventing new extension types beyond current shipped Lua reality
