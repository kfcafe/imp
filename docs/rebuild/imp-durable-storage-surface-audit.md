# imp Durable Storage Surface Audit

This audit inventories current durable imp-managed file surfaces and path-resolution behavior for mana unit `264.1`.

## Current path helpers

Current code now has a more centralized storage helper in `crates/imp-core/src/storage.rs`:

- canonical global root: `~/.imp`
- canonical project root: `<project>/.imp`
- canonical sessions: `~/.imp/sessions`
- canonical indexes: `~/.imp/indexes`
- canonical session index: `~/.imp/indexes/session_index.db`
- legacy roots remain compatibility inputs.

Older notes mention `Config::user_config_dir()` / `Config::session_dir()` as primary helpers. The current code has moved toward `storage.rs`, but many durable surfaces and callers still assemble paths at the edges.

## Storage surfaces

| Artifact | Scope / ownership | Current path(s) | Code owner/helper | Precedence / discovery | Known mismatch / risk |
| --- | --- | --- | --- | --- | --- |
| `config.toml` | user global config + project override | `~/.imp/config.toml`, `<project>/.imp/config.toml` | config/storage helpers and CLI/TUI callers | project can override/merge selected settings | path policy must stay explicit: config is not the same as transcript data |
| `auth.json` | user global auth metadata | `~/.imp/auth.json` | auth/setup CLI/TUI paths, imp-llm auth metadata | global only | secret values are not all in this file; keychain owns sensitive values |
| secret values | machine-local secure storage | OS keychain service `imp` | `imp-llm/src/auth.rs` / secret backend | global machine store | do not normalize into plain files |
| raw sessions | user global durable data | `~/.imp/sessions/*.jsonl`; historical `~/.local/share/imp/sessions/*.jsonl` | session/imp_session/storage helpers | current canonical root plus legacy recovery inputs | many raw sessions can exist without FTS index |
| session index | user global durable index | `~/.imp/indexes/session_index.db`; historical candidates may exist elsewhere | `session_index.rs`, `tools/session_search.rs`, storage helper | search reads index DB only | production indexing was unwired at audit time; path cleanup alone does not fix recall |
| `memory.md` | user-global memory doc | `~/.imp/memory.md` | memory tool/config paths | global only unless future project memory is introduced | currently a document surface, not structured DB |
| `user.md` | user-global personal context | `~/.imp/user.md` | resources/context assembly | global | should remain user-global unless project-specific persona is introduced explicitly |
| `soul.md` | user global + project local identity/instructions | `~/.imp/soul.md`, `<project>/.imp/soul.md` | resources discovery | user + project composition | merge/precedence must be deterministic |
| imported `AGENTS.md` | user global imported instructions | `~/.imp/AGENTS.md` | `import.rs` | import copies first selected instruction file | imported global instructions can surprise project-local behavior if not surfaced |
| skills | user global + project local extensions | `~/.imp/skills/**`, `<project>/.imp/skills/**` | `resources.rs`, import flows, extend tool | user + project discovery, project ancestry walk where applicable | agent-created skills under user tree need clear ownership |
| prompts | user global + project local templates | `~/.imp/prompts/**`, `<project>/.imp/prompts/**` | `resources.rs` prompt discovery | user + project discovery | historically defined but production wiring may be sparse; avoid preserving dead surfaces blindly |
| Lua extensions | user global + project local extension substrate | `~/.imp/lua/**`, `<project>/.imp/lua/**` | `imp-lua/src/loader.rs` | global plus project discovery | shipped extension substrate is Lua; TS extensions are future-only |
| shell tool TOML | possible tool extension config | caller-provided dirs to `load_shell_tools` | `tools/shell.rs` | production roots not clearly wired in current shipped path | may be an unwired/dead surface; should be confirmed before normalizing |
| web/cache artifacts | tool/runtime data | tool-specific, not fully audited here | `tools/web/*` | tool-specific | should not be mixed with config docs without explicit cache/data policy |
| checkpoints/artifacts | runtime recovery/evidence data | checkpoint/tool-specific stores | checkpoint/tool modules | runtime-generated | should remain data/artifact refs, not config |

## Current shipped reality vs desired cleanup

Current shipped reality:

- `~/.imp` is the accepted imp-native global root.
- project-local configuration/extensions live under `<project>/.imp`.
- sessions are raw JSONL files under `~/.imp/sessions`, with historical XDG-style raw sessions still relevant for recovery.
- session search requires an FTS index DB and does not search raw JSONL directly.
- auth metadata and secret values are intentionally split between file metadata and OS keychain.
- skills/prompts/soul/Lua have explicit global + project discovery behavior.
- some durable-looking surfaces (prompt templates and shell-tool TOML roots) may be defined more broadly than they are used.

Desired cleanup:

- keep one canonical storage-path module as the only place that defines roots;
- separate config, durable user data, cache/index, artifact/checkpoint, and secure-secret classes;
- make legacy roots migration inputs rather than continuing write destinations;
- add session-index lifecycle/rebuild support;
- either wire or retire currently ambiguous prompt/shell-tool storage surfaces;
- document precedence/merge rules for global and project `.imp` surfaces.

## Mismatch taxonomy

1. **Path assembly drift** — some call sites still assemble files directly rather than going through the storage helper.
2. **Config/data/index conflation** — not every durable artifact belongs in the same root category, even if all live under `~/.imp`.
3. **Session search lifecycle gap** — raw sessions can exist without an index, so `recall` can appear empty.
4. **Legacy recovery drift** — historical XDG/macOS paths may contain real user data.
5. **Potential dead surfaces** — prompts and shell-tool TOML need proof of runtime wiring or explicit deprecation.
6. **Secret safety boundary** — keychain values must not be migrated into plain file topology.

## Follow-on job recommendations

Completed/active follow-ons already align with this audit:

- `264.2`: normalized storage contract and migration policy.
- `264.4`: session-index lifecycle reliability.
- `264.6`: canonical filesystem roots.
- `264.7`: precedence and merge rules for global/project `.imp`.
- `264.8`: migration from legacy XDG/macOS paths.

Additional recommended child if not already tracked:

- Audit prompt-template and shell-tool TOML production wiring, then either document them as supported storage surfaces or retire/deprioritize them from normalized topology.

## Conclusion

The current topology has converged toward `~/.imp` plus project `.imp`, but durable surfaces still need class-specific policy. Session recovery failure is not just path mismatch; it also requires indexing lifecycle work. The next storage work should preserve the config/data/secret/index distinctions while eliminating ad hoc path assembly and ambiguity around legacy roots.
