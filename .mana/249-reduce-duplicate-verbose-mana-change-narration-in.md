---
id: '249'
title: Reduce duplicate verbose mana-change narration in imp chat
slug: reduce-duplicate-verbose-mana-change-narration-in
status: open
priority: 2
created_at: '2026-04-13T23:30:07.716739Z'
updated_at: '2026-04-24T05:37:22.464635Z'
acceptance: '1. System prompt no longer redundantly instructs the model twice to summarize the same mana delta in normal replies. 2. Interactive mana tool flows still surface visible mana state, but background orchestration does not force a second redundant chat summary when the UI/widget already shows it. 3. Any skill wording changed should reinforce concise mana reporting rather than repeated restatement. 4. Focused tests pass.'
notes: |-
  ---
  2026-04-13T23:40:07.214394+00:00
  Implemented the first over-reporting fix after auditing prompt + runtime layers. Changes landed in imp/crates/imp-core/src/system_prompt.rs, imp/crates/imp-core/src/tools/mana.rs, imp/.imp/skills/mana/SKILL.md, and imp/.imp/skills/mana-delegation/SKILL.md. Specific fixes: (1) removed the duplicated execution-policy instruction that separately required a concise mana delta summary and replaced it with a non-duplicative rule to update mana before the substantive reply while avoiding verbose restatement when tool output/UI already shows the delta; (2) gated background-run AgentCommand::FollowUp summaries in the native mana tool behind !ui.has_ui(), so open interactive imp chats rely on the visible mana widget/status instead of also getting a second chat summary; headless/non-UI flows still receive the follow-up summary; (3) tightened local mana skills to say at most one brief delta summary and only when the tool/UI did not already make the change obvious. Verification: cargo test -p imp-core system_prompt_includes_conversation_time_mana_planning_doctrine -- --nocapture; cargo test -p imp-core background_run_enqueues_follow_up_on_completion_without_ui -- --nocapture; cargo test -p imp-core background_run_with_ui_does_not_enqueue_follow_up_on_completion -- --nocapture. While running focused tests, the build surfaced an unrelated but current compile blocker in many imp-core tool tests: ToolContext test helpers were missing the newer turn_mana_review field. I patched those helper initializers across imp-core tool test modules so focused imp-core tests compile again; these are test-only support changes, not runtime behavior changes.

  ---
  2026-04-13T23:40:55.842668+00:00
  Durable next-step decomposition after the first fix:
  1. Remaining mana-specific audit if continuing: inspect interactive reply generation after native mana mutations to find any remaining duplicate narration beyond the background-run follow-up path. Focus on imp/crates/imp-core/src/agent.rs follow-up nudges, confidence-continue text, turn mana review surfaces, and any places where assistant chat may restate widget-visible `mana delta` output.
  2. Broader verbosity audit if continuing: inspect global prompt/personality/skill layers that may over-encourage chat narration even when tool/UI output already carries the state. Focus on imp/crates/imp-core/src/system_prompt.rs working-style and execution-policy wording, any soul/personality rendering that affects verbosity, and mana-related skills under imp/.imp/skills/.
  3. Preference carried forward: in interactive/UI mode, prefer one visible mana state surface over redundant duplicate chat summaries; reserve extra chat narration for headless/no-UI flows or when the tool/UI did not make the change obvious.

  ---
  2026-04-14T00:04:13.736475+00:00
  Second-pass audit completed. New findings + fixes: (1) there was still a prompt-level contradiction in imp/crates/imp-core/src/system_prompt.rs: the main mana doctrine still said to summarize the mana delta for the user, while the execution-policy layer had already been tightened to avoid verbose restatement when the tool/UI already shows the delta. Rewrote that remaining doctrine line to: update mana before the substantive reply; summarize only when it adds value beyond what the mana tool/UI already made visible. (2) inspected agent follow-up nudges in imp/crates/imp-core/src/agent.rs and found wording that still nudged visibility but did not explicitly discourage duplicate chat restatement. Tightened mana_externalization_follow_up_text() and confidence_continue_follow_up_text() so they preserve momentum while explicitly avoiding repeated summary when the delta is already visible. (3) discovered that Tower-root chats do not load imp/.imp/skills/... when cwd is /Users/asher/tower; skill discovery walks cwd ancestry and user config, and there was no /Users/asher/tower/.imp/skills override. That meant root chats were still inheriting the older global ~/.imp/skills/mana/SKILL.md. Added Tower-root overrides at /Users/asher/tower/.imp/skills/mana/SKILL.md and /Users/asher/tower/.imp/skills/mana-delegation/SKILL.md so root chats now inherit the tighter non-duplicative mana guidance. Verification for second pass: cargo test -p imp-core system_prompt_includes_conversation_time_mana_planning_doctrine -- --nocapture; cargo test -p imp-core agent_queues_confidence_continue_follow_up_after_visible_mana_turn -- --nocapture; cargo test -p imp-core agent_queues_mana_externalization_follow_up_after_planning_turn -- --nocapture. All passed. Residual note: the user-global ~/.imp/skills/mana/SKILL.md remains older and more verbose, but Tower-root chats are now overridden by the root-local skill files.

  ---
  2026-04-14T00:05:00.223005+00:00
  User reported still seeing the old nudge string: 'Before you continue: externalize the durable plan or decomposition you just described into mana now ... and keep the delta visible.' Repo-grounded check confirms that exact old string no longer exists in current Tower source; current agent.rs wording now says to avoid extra chat restatement when the mana tool/UI already makes the delta obvious. Likely explanation: already-open imp session / previously queued follow-up / older built binary still running. Practical follow-up if needed: verify which imp binary/path interactive chats are launching from and whether fresh sessions pick up the changed runtime text.

  ---
  2026-04-14T00:10:48.260104+00:00
  Verified install/runtime path and updated the main binary. Evidence: `which imp` resolved to /Users/asher/bin/imp, and before reinstall that binary still contained the old string ending with 'keep the delta visible'. Built fresh with `cargo build -p imp-cli`, verified /Users/asher/tower/target/debug/imp contained the new wording ('avoid extra chat restatement...' and the tightened confidence nudge), backed up the installed binary to /Users/asher/bin/.backup/imp.pre-mana-verbosity-fix.20260413-171033, then replaced /Users/asher/bin/imp with the fresh build. Post-install verification: /Users/asher/bin/imp sha256 now matches /Users/asher/tower/target/debug/imp, and `strings /Users/asher/bin/imp` shows the new wording, not the old 'keep the delta visible' string. Fresh imp sessions should now use the updated runtime text.

  ---
  2026-04-14T01:44:43.593495+00:00
  Patched the user-global mana skill at /Users/asher/.imp/skills/mana/SKILL.md to match the tightened non-duplicative guidance used in Tower-local overrides. This matters for non-Tower chats or any cwd that does not have a nearer .imp/skills/mana override. There is no user-global mana-delegation skill present currently, so only the global mana skill needed patching. Fresh sessions outside Tower should now stop inheriting the older verbose mana guidance.

  ---
  2026-04-14T01:49:59.850495+00:00
  Created the missing user-global delegation skill at /Users/asher/.imp/skills/mana-delegation/SKILL.md using the tightened non-duplicative version aligned with Tower-local guidance. Global mana-related skill coverage is now consistent for fresh non-Tower sessions: both ~/.imp/skills/mana/SKILL.md and ~/.imp/skills/mana-delegation/SKILL.md now discourage redundant mana delta restatement when tool/UI output already shows the change.
labels:
- imp
- prompt
- mana
- ux
verify: cargo test -p imp-core system_prompt_ -- --nocapture && cargo test -p imp-core mana_tool_ -- --nocapture
checkpoint: '97eb51b8933344533fbb622a95b1d171ce7c426e'
verify_hash: f0edb28beea1cb4888ea81abf28c160f7a619cb10d1dafb7091c5bc7ea353175
kind: epic
attempt_log:
- num: 1
  outcome: abandoned
  agent: imp
  started_at: '2026-04-13T23:32:01.228752Z'
  finished_at: '2026-04-24T05:37:22.464635Z'
---

Audit imp's prompt/runtime/skill layers for over-reporting mana changes in interactive chat, identify where the same mana delta is being surfaced multiple times, and implement the smallest coherent fix. Current inspected evidence: imp/crates/imp-core/src/system_prompt.rs repeats between-turn mana update / mana delta summary guidance in both Mana doctrine and Execution discipline; imp/crates/imp-core/src/tools/mana.rs emits `mana delta: ...` tool outputs and widgets for create/update/close/etc.; background orchestration also queues an AgentCommand::FollowUp summary via make_follow_up_summary(); agent confidence follow-up text explicitly says the mana delta is already visible. Goal: keep mana visible without making the assistant restate the same change twice. Scope: prompt wording, mana tool follow-up behavior, and the mana skill text if needed. Verify by running focused imp-core tests covering system prompt wording and mana tool behavior; add/adjust tests if necessary.
