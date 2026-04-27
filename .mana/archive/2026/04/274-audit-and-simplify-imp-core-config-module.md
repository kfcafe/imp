---
id: '274'
title: Audit and simplify imp-core config module
slug: audit-and-simplify-imp-core-config-module
status: closed
priority: 2
created_at: '2026-04-27T05:16:10.912268Z'
updated_at: '2026-04-27T05:44:59.863418Z'
acceptance: Small focused refactor improves config.rs readability/maintainability; existing config behavior is preserved except for clear bug fixes covered by tests; no unrelated files changed.
notes: |-
  ---
  2026-04-27T05:16:14.425744+00:00
  Started audit. Existing dirty worktree includes config.rs but git reports no diff for it under root, so will avoid unrelated changes and verify before/after with focused tests.

  ---
  2026-04-27T05:16:27.388853+00:00
  Baseline verify `cargo test -p imp-core config::tests` is currently blocked by an unrelated compile error in crates/imp-core/src/import.rs:308 (unexpected closing delimiter). Will still make config.rs changes and use narrower checks where possible, then report this blocker.

  ---
  2026-04-27T05:17:23.708641+00:00
  Implemented focused config.rs cleanup: extracted AgentMode tool/action allowlists into named constants, delegated env mode parsing to AgentMode::from_name, reused serde default helper functions in Default impls, and fixed Config::merge to include theme/learning/lua. Added regression test for those merged fields and aligned planner update permission test with the actual allowlist/instructions.

  ---
  2026-04-27T05:17:41.808869+00:00
  Post-change verify `cargo test -p imp-core config::tests` still blocked before config tests execute, now by unrelated crates/imp-core/src/import.rs:765 unexpected closing delimiter in test module. `cargo fmt -p imp-core --check` passed for formatting.

  ---
  2026-04-27T05:18:12.029846+00:00
  Narrow file checks passed: `rustfmt --edition 2021 --check crates/imp-core/src/config.rs` and `git diff --check -- crates/imp-core/src/config.rs`. Full crate test remains blocked by unrelated import.rs delimiter error. Note: `cargo fmt -p imp-core --check` also surfaces unrelated formatting diffs in import.rs.

  ---
  2026-04-27T05:41:50.262128+00:00
  User asked to continue and make a targeted commit when done. Continuing within config.rs only; unrelated dirty files/import.rs compile error remain out of scope unless needed for verification.

  ---
  2026-04-27T05:44:13.926505+00:00
  Targeted staged verify passed using only staged config.rs changes: temporarily wrote index version to working tree, ran `cargo test -p imp-core --lib config::tests` (31 passed), then restored user's working-tree config changes. Staged diff intentionally excludes unrelated in-progress UI default change in config.rs and all other dirty files.
labels:
- config
- refactor
- rust
closed_at: '2026-04-27T05:44:59.863418Z'
close_reason: Config audit/refactor is present in current HEAD and targeted config tests pass. Remaining working-tree config diff is unrelated sidebar/chat default behavior and was left uncommitted.
verify: cargo test -p imp-core config::tests
is_archived: true
history:
- attempt: 1
  started_at: '2026-04-27T05:44:59.469879Z'
  finished_at: '2026-04-27T05:44:59.839814Z'
  duration_secs: 0.369
  result: pass
  exit_code: 0
  output_snippet: 'verify passed: cargo test -p imp-core config::tests'
outputs:
  text: |-
    running 31 tests
    test config::tests::agent_mode_auditor_mana_readonly ... ok
    test config::tests::agent_mode_full_allows_all_tools ... ok
    test config::tests::agent_mode_orchestrator_allows_read ... ok
    test config::tests::agent_mode_instructions ... ok
    test config::tests::agent_mode_orchestrator_blocks_write ... ok
    test config::tests::agent_mode_planner_allows_mana_create ... ok
    test config::tests::agent_mode_planner_blocks_mana_close_and_run ... ok
    test config::tests::agent_mode_reviewer_no_mana ... ok
    test config::tests::agent_mode_worker_allows_mana_update ... ok
    test config::tests::agent_mode_default_is_full ... ok
    test config::tests::agent_mode_worker_blocks_mana_create ... ok
    test config::tests::config_default_values ... ok
    test config::tests::config_merge_context_overrides_default ... ok
    test config::tests::config_load_missing_file_returns_default ... ok
    test config::tests::config_merge_hooks_extend ... ok
    test config::tests::config_merge_includes_theme_learning_and_lua ... ok
    test config::tests::config_merge_guardrails_preserves_unspecified_fields ... ok
    test config::tests::config_merge_project_overrides_user ... ok
    test config::tests::config_merge_personality_project_overrides_user_and_keeps_saved_profiles ... ok
    test config::tests::config_resolve_env_overrides ... ok
    test config::tests::config_merge_roles_extend ... ok
    test config::tests::agent_mode_config_deserialize ... ok
    test config::tests::config_parse_thinking_levels ... ok
    test config::tests::config_load_with_roles_and_hooks ... ok
    test config::tests::config_partial_toml_fills_defaults ... ok
    test config::tests::config_load_from_toml ... ok
    test config::tests::config_loads_personality_section ... ok
    test config::tests::config_resolve_missing_files_uses_defaults ... ok
    test config::tests::lua_config_resolves_capability_policy ... ok
    test config::tests::non_orchestrator_modes_block_spawn ... ok
    test config::tests::config_resolve_user_then_project ... ok

    test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 634 filtered out; finished in 0.00s
kind: task
paths:
- /Users/asher/tower/imp/crates/imp-core/src/config.rs
autonomy_disposition:
  kind: eligible
  review: unknown
  approval: unknown
  verify: satisfied
  visibility: unknown
  attempt_pressure: within_budget
  risk: unknown
  provenance: mixed
  continuation_budget: 3
---

Audit `/Users/asher/tower/imp/crates/imp-core/src/config.rs`, reduce obvious duplication and improve maintainability without broad behavior changes. Preserve existing public config shape and avoid touching unrelated dirty files. Focus areas observed before starting: duplicated AgentMode parsing (`AgentMode::from_name` plus `parse_agent_mode`), repetitive default literals in `UiConfig`/`LearningConfig`, merge logic that forgets `learning`, `theme`, and `lua`, and tests that include a likely stale assertion around planner update permissions. Inspect call sites before changing semantics.
