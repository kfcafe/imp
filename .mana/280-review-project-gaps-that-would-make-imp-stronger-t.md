---
id: '280'
title: Review project gaps that would make imp stronger than current baseline
slug: review-project-gaps-that-would-make-imp-stronger-t
status: open
priority: 1
created_at: '2026-04-27T19:23:43.040906Z'
updated_at: '2026-04-27T21:58:00.843869Z'
acceptance: User receives a concise prioritized review; no code modifications are made.
notes: |-
  ---
  2026-04-27T19:23:49.503774+00:00
  Read-only review evidence gathered:
  - Repo is clean on branch split/imp at 0c82d65.
  - Core docs inspected: README.md, vNext.md, imp_rebuild_plan.md, imp_ontology.md, ARCHITECTURE.md, IMP_REVIEW.md, IMP_DEEP_REVIEW.md, ENGINEERING_GUARDRAILS.md, LEARNING_LOOP_SPEC.md.
  - Proposal docs inspected: guest-runtime-extension-substrate.md, imp-run-worker-contract-and-mana-run-handoff.md, mana-aware-runtime-context-read-path.md, tool-ab-harness-notes.md.
  - Crate map inspected: imp-cli, imp-core, imp-llm, imp-lua, imp-tui.
  - Largest files by current wc: imp-tui/app.rs 7078, imp-core/agent.rs 5782, imp-cli/lib.rs 5679, imp-core/tools/mana.rs 3514, imp-llm/providers/anthropic.rs 2463, imp-core/session.rs 2334, imp-tui/views/sidebar.rs 1781, imp-core/imp_session.rs 1777, imp-core/system_prompt.rs 1725, imp-core/config.rs 1550.
  - Existing mana roadmap already tracks many major gaps: 47 rebuild epic, 47.2 core decomposition, 47.3 runner/worktree/sandbox, 47.4 policy, 47.5 evidence, 27 mana tool UX, 29 discoverability, 37 usage reporting, 44 guest-runtime, 46 maturity pass, 267 opencode lessons.
  - Review synthesis: biggest gaps are not missing random features; they are making existing strengths reliable, visible, enforceable, and measurable: shared runtime bootstrap, hard policy boundaries, cancellation/tool execution semantics, evidence/eval loop, mana-aware context read path, productized checkpoints/planning, and install/onboarding polish.

  ---
  2026-04-27T19:36:25.552031+00:00
  Continuation/deeper inspection corrections:
  - Mana facts/status read path is no longer purely a proposal gap: `builder.rs` calls `mana_prompt_context::load_session_prompt_context(&cwd)` when explicit facts are empty, and `mana_prompt_context.rs` maps `mana_core::api::memory_context` into prompt facts + compact project memory status. Remaining gap is likely wiki/index integration and relevance/quality, not basic facts injection.
  - Cancellation has improved since `IMP_DEEP_REVIEW.md`: `agent.rs` now stores a shared `cancel_token`, passes it into `ToolContext`, and `tools/bash.rs` waits for cancellation and kills the process group. Remaining gap is proving coverage across all long-running tools and TUI Esc state, not basic bash cancellation only.
  - Lua policy has improved: `LuaRuntime::new()` defaults shell/http/secrets/env deny, but native tool calls still default allow and `LuaConfig::resolve_policy(_mode)` ignores mode. Remaining high-value gap is making mode-aware policy authoritative and deny/least-privilege for native tool access.
  - Checkpoints are more live than older review suggests: `CheckpointState` is wired into write/git snapshots, `ImpSession` persists checkpoint records, and TUI has restore command/tests. Remaining gap is productization/discoverability/timeline/diff, not raw mechanism.
  - Provider/auth duplication remains visible: `imp_session.rs` owns `resolve_runtime_connection`/`resolve_api_key`; `imp-cli/src/lib.rs` and `imp-tui/src/app.rs` still have local provider API-key/login helpers and ChatGPT routing checks.
  - Hotspot sizes are now larger than older review: app.rs 7078, agent.rs 5782, imp-cli lib.rs 5679, tools/mana.rs 3514.

  ---
  2026-04-27T20:01:37.315247+00:00
  User clarified comparison target: compare imp against https://github.com/dirac-run/dirac, not imp against its own baseline. Need inspect Dirac repo/public docs and update gap analysis accordingly.

  ---
  2026-04-27T20:02:52.220396+00:00
  Dirac comparison evidence:
  - Cloned https://github.com/dirac-run/dirac shallow into temp and inspected README, evals, package metadata, tool handlers, prompts, and file layout.
  - Dirac positioning: token-efficient coding agent; claims 64.8% average API cost reduction, 8/8 success on 8 public complex refactor evals with avg $0.18, and 65.2% Terminal-Bench-2 on gemini-3-flash-preview.
  - Dirac core differentiators in README/package/walkthroughs: hash-anchored edits, AST-native precision, multi-file batching, high-bandwidth context, native tool calling only, VS Code + CLI distribution, Plan Mode/Yolo Mode/history.
  - Inspected Dirac implementation evidence: `src/core/task/tools/handlers/EditFileToolHandler.ts` batches all edit_file blocks in a turn; `edit-file/BatchProcessor.ts` groups by path, applies edits in memory, requests combined approval, saves, then runs diagnostics in parallel; `GetFunctionToolHandler.ts` uses `ASTAnchorBridge` and function hashes; `GetFileSkeletonToolHandler.ts` extracts AST skeletons/call graph; `FindSymbolReferencesToolHandler.ts` uses `SymbolIndexService`; `ReplaceSymbolToolHandler.ts` replaces AST symbol ranges and diagnostics; `src/shared/tools.ts` exposes get_function/get_file_skeleton/find_symbol_references/edit_file/replace_symbol/rename_symbol.
  - Dirac prompt snapshots explicitly instruct surgical tools before full file reads, line-hash protocol, and batching all non-overlapping edits into one tool call.
  - Dirac has broader IDE/product packaging than imp today: VS Code extension marketplace, webview UI, walkthroughs, editor diagnostics/diff integration, command palette/context menu entries, CLI, ACP stubs.
  - Dirac also has weaknesses relative to imp: no mana-style durable work graph/verify gates/handoff substrate, no explicit Rust-native worker runtime boundary, no project-durable task orchestration comparable to mana, apparent large TS surface inherited from Cline with 136k TS LOC, and benchmark/readme heavily optimized around coding/refactor efficiency rather than broader agent runtime/policy graph.
  ## Attempt 1 — 2026-04-27T21:46:09Z
  Exit code: 2

  ```
  sh: line 0: test: analysis-only}: binary operator expected
  ```


  ---
  2026-04-27T21:58:00.843855+00:00
  Corrected malformed verify command from `test -n "analysis-only"} aisuiu` to the intended read-only analysis sentinel `test -n "analysis-only"`. Prior failed attempt was caused by the malformed verify string, not by analysis work failing.
labels:
- analysis
- imp
- roadmap
- competitive-review
verify: test -n "analysis-only"} aisuiu
attempts: 1
history:
- attempt: 1
  started_at: '2026-04-27T21:46:09.926751Z'
  finished_at: '2026-04-27T21:46:09.982026Z'
  duration_secs: 0.055
  result: fail
  exit_code: 2
  output_snippet: 'sh: line 0: test: analysis-only}: binary operator expected'
kind: task
---

## Goal
Produce a concise architecture/product gap review for the current `/Users/asher/imp` repo focused on what would make imp stronger than its current baseline and stronger against peer coding-agent products.

## Scope
- In scope: read-only review of repo docs, mana roadmap, crate structure, known review findings, and high-signal code/module hotspots.
- Out of scope: code changes, broad implementation, dependency changes, or committing.

## Deliverable
- A prioritized gap analysis with evidence from inspected files and mana state.
- Call out what imp already has that should be amplified, and where the actual gaps are.

## Focus
- `README.md`
- `vNext.md`
- `imp_rebuild_plan.md`
- `imp_ontology.md`
- `IMP_REVIEW.md`
- `IMP_DEEP_REVIEW.md`
- `docs/proposals/*`
- crate structure under `crates/`

## Done / Verify
- Done when the user receives a ranked memo and durable review state is captured in mana.
- Verify: `test -n "analysis-only"`
