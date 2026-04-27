---
id: '272'
title: Add Kimi model compatibility and fix Ctrl+L model picker in imp
slug: add-kimi-model-compatibility-and-fix-ctrll-model-p
status: closed
priority: 2
created_at: '2026-04-27T03:50:46.456878Z'
updated_at: '2026-04-27T04:04:13.591580Z'
notes: |-
  ---
  2026-04-27T03:50:52.739632+00:00
  Starting implementation. Current repo has unrelated dirty YouTube web-tool files; will avoid those paths. Inspecting imp-llm model/provider routing and imp-tui Ctrl+L model selector before changing code.

  ---
  2026-04-27T03:58:26.135690+00:00
  Findings: repo already has Moonshot/Kimi and Kimi Code provider entries, aliases, auth env lookup, and OpenAI-compatible routing. Remaining Kimi runtime gap is thinking/tool-call compatibility: OpenAI-compatible stream emits reasoning deltas to UI but drops them from persisted assistant messages, and request conversion never sends Moonshot `reasoning_content`/`thinking` back on follow-up tool turns. Ctrl+L picker likely UX bug: current model aliases are not normalized in the selector and the picker can open with no actionable/default selection when auth filtering hides models. Implementing targeted provider request fixes plus picker normalization/current-model inclusion.

  ---
  2026-04-27T04:04:07.273497+00:00
  Implemented targeted changes: updated Moonshot/Kimi built-in model catalog/aliases, added Kimi thinking/temperature handling and reasoning_content preservation in OpenAI-compatible provider, and fixed Ctrl+L picker to include/canonicalize the current model and initially highlight it. Verification passed: cargo test -p imp-llm model::tests; cargo test -p imp-llm openai_compat; cargo test -p imp-tui model; cargo check -p imp-cli -p imp-llm -p imp-tui. Note: worktree has unrelated pre-existing dirty files outside this unit, including YouTube/web-tool work and other imp-core/TUI changes.
labels:
- imp
- provider
- kimi
- model-picker
- imp-llm
- imp-tui
closed_at: '2026-04-27T04:04:13.591580Z'
verify: cd /Users/asher/tower/imp && cargo test -p imp-llm model::tests -- --nocapture && cargo test -p imp-tui model -- --nocapture && cargo check -p imp-cli -p imp-llm -p imp-tui
checkpoint: '6d2325a21e86a8e5699303e0b3aa572ef048938f'
verify_hash: '81730cfed710258ac971569408396ba771a0739d1a984449ac5b1e388406e783'
claimed_by: imp
claimed_at: '2026-04-27T03:50:49.648279Z'
is_archived: true
history:
- attempt: 1
  started_at: '2026-04-27T04:04:12.699243Z'
  finished_at: '2026-04-27T04:04:13.534949Z'
  duration_secs: 0.835
  result: pass
  exit_code: 0
  output_snippet: 'verify passed: cd /Users/asher/tower/imp && cargo test -p imp-llm model::tests -- --nocapture && cargo test -p imp-tui model -- --nocapture && cargo check -p imp-cli -p imp-llm -p imp-tui; no declared path overlap'
outputs:
  text: |-
    running 19 tests
    test model::tests::builtin_openai_codex_models_retag_openai_models ... ok
    test model::tests::find_by_alias_falls_back_to_exact_id ... ok
    test model::tests::find_by_alias_resolves_gpt5 ... ok
    test model::tests::find_by_alias_resolves_chatgpt ... ok
    test model::tests::find_by_alias_resolves_codex ... ok
    test model::tests::find_by_alias_resolves_haiku ... ok
    test model::tests::find_by_alias_resolves_opus ... ok
    test model::tests::find_by_alias_resolves_gemini_pro ... ok
    test model::tests::find_by_alias_resolves_kimi ... ok
    test model::tests::find_by_alias_resolves_kimi_turbo ... ok
    test model::tests::find_by_alias_resolves_sonnet ... ok
    test model::tests::find_by_alias_returns_none_for_unknown ... ok
    test model::tests::register_skips_duplicates ... ok
    test model::tests::list_by_provider_filters_correctly ... ok
    test model::tests::resolve_meta_guesses_moonshot_for_kimi_models ... ok
    test model::tests::resolve_meta_synthesizes_spark_preview ... ok
    test model::tests::provider_registry_includes_moonshot ... ok
    test model::tests::resolve_meta_synthesizes_legacy_openai_model ... ok
    test model::tests::resolve_meta_synthesizes_gpt_5_5_alias ... ok

    test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 210 filtered out; finished in 0.01s


    running 13 tests
    test keybindings::tests::ctrl_p_cycles_model_forward ... ok
    test keybindings::tests::ctrl_shift_p_cycles_model_backward ... ok
    test views::model_selector::tests::custom_model_is_available_after_builtin_matches ... ok
    test views::model_selector::tests::custom_model_is_selected_when_no_builtin_matches ... ok
    test views::model_selector::tests::model_selector_initially_selects_current_model ... ok
    test app::session_lifecycle::model_picker_includes_current_alias_even_without_auth ... ok
    test views::settings::tests::chosen_models_round_trip_into_config ... ok
    test views::settings::tests::cycle_model_is_safe_with_empty_model_options ... ok
    test app::session_lifecycle::filtered_model_options_includes_chatgpt_oauth_only_models ... ok
    test views::settings::tests::empty_chosen_models_means_all_models ... ok
    test app::session_lifecycle::filtered_model_options_hides_chatgpt_oauth_only_models_when_openai_api_key_exists ... ok
    test app::session_lifecycle::tui_integration_model_switch_updates_context_window ... ok
    test app::session_lifecycle::tui_integration_model_switch_via_cycle ... ok

    test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 196 filtered out; finished in 0.03s
kind: task
paths:
- imp/crates/imp-llm/src/model.rs
- imp/crates/imp-llm/src/providers
- imp/crates/imp-tui/src/app.rs
- imp/crates/imp-tui/src/views
- imp/crates/imp-cli/src
attempt_log:
- num: 1
  outcome: success
  agent: imp
  started_at: '2026-04-27T03:50:49.648279Z'
  finished_at: '2026-04-27T04:04:13.591580Z'
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

Implement Kimi compatibility in imp without disturbing existing uncommitted YouTube web-tool work. Investigate current provider/model routing, add Kimi/Moonshot model support through the narrowest compatible provider path, and fix the TUI Ctrl+L model picker behavior so users can reliably choose/switch models from the picker. Update tests for model resolution and picker behavior where practical. Avoid broad refactors and avoid touching crates/imp-core/src/tools/web/* unless required, because those files already contain unrelated local changes.
