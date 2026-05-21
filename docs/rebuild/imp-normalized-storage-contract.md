# Normalized imp Storage Contract

This contract defines imp's canonical storage topology and migration policy.

## Canonical roots

imp uses two first-class roots:

- user-global root: `~/.imp/`
- project-local root: `<project>/.imp/`

XDG, macOS Application Support, and prior scattered locations are compatibility/migration sources only unless a future decision explicitly changes this policy.

## User-global `~/.imp/`

Allowed contents:

- `config.toml` — user-global imp configuration.
- `auth.json` — non-secret auth/provider metadata only.
- `soul.md` — user-global identity/operating context.
- `agents.md` — imp-native AGENTS-style operating instructions.
- `memory.md` — user-global memory document.
- `user.md` — user profile/context document.
- `sessions/` — durable raw session transcripts.
- `indexes/` — rebuildable indexes over durable data, including session index DBs.
- `skills/` — user-global skills.
- `prompts/` — reserved/experimental prompt templates.
- `tools/` — reserved/experimental shell-tool definitions; do not auto-enable without policy.
- `lua/` — shipped Lua extension substrate.
- `imports/` — imported external instruction/config artifacts.
- `work/` — native imp-work store.

Secrets are not stored as plain files. Secret values belong in the OS keychain or configured secret backend; disk files may store only metadata and references.

## Project-local `<project>/.imp/`

Allowed contents:

- `config.toml` — project-local configuration overrides.
- `soul.md` — project-local identity/context.
- `agents.md` — project-local operating instructions.
- `skills/` — project-local skills.
- `prompts/` — reserved/experimental prompt templates.
- `tools/` — reserved/experimental shell-tool definitions; policy-gated before activation.
- `lua/` — project-local Lua extensions.
- `work/` — project-local native imp-work store when the project owns work state.

Project `.imp/` is for project behavior, configuration, extension content, and project work. It is not the home for personal global sessions, user memory, auth metadata, or secrets.

## Precedence and merge rules

- Project-local `.imp/` overrides or extends global `~/.imp/` for project-aware behavior/content.
- User-global personal artifacts remain global only: sessions, indexes, auth metadata, personal memory, user profile, and machine-local state.
- Extension-like directories are merged in deterministic order: global first, project second, with project-local entries taking precedence on name conflicts unless the feature defines additive behavior.
- Config should use explicit merge rules per field; do not infer broad merges for sensitive/provider/auth settings.

## Legacy compatibility and migration

Legacy lookup sources may include:

- XDG config/data roots;
- macOS Application Support roots;
- historical `~/.local/share/imp` session stores;
- top-level `AGENTS.md` files;
- older imported instruction locations.

Policy:

1. Read legacy roots as migration/recovery sources.
2. Prefer writing new data to canonical `~/.imp/` or project `.imp/` only.
3. Do not delete legacy data automatically.
4. Migration tooling must report copied/skipped/lossy items.
5. After migration, canonical roots are the source of truth.

## Required imp-core helpers

Path decisions belong in imp-core, not scattered call sites. Required helper categories:

- `global_root()` -> `~/.imp/`
- `project_root(project)` -> `<project>/.imp/`
- `global_config_path()`
- `project_config_path(project)`
- `global_sessions_dir()`
- `global_indexes_dir()`
- `global_session_index_path()`
- `global_skills_dir()` / `project_skills_dir(project)`
- `global_prompts_dir()` / `project_prompts_dir(project)`
- `global_tools_dir()` / `project_tools_dir(project)`
- `global_lua_dir()` / `project_lua_dir(project)`
- `global_work_dir()` / `project_work_dir(project)`
- legacy-root discovery helpers for read-only migration/recovery.

## Non-goals

- No immediate deletion of legacy data.
- No automatic activation of shell-tool TOML roots.
- No migration of secret values into disk files.
- No storage of global personal history in project `.imp/`.
- No platform-native root split unless a future decision replaces the accepted `~/.imp/` policy.

## Implementation status

The current codebase has converged toward `~/.imp` and project `.imp` through `crates/imp-core/src/storage.rs`, with follow-up audits covering session index lifecycle, prompt/shell-tool wiring, and durable surface inventory.
