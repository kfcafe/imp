# Dead code audit

## Prior concerns
# Tighten imp product surface — results

Status: completed with staged-backend concerns

## What changed

- Removed `crates/imp-gui` from workspace `default-members` while keeping it explicitly buildable.
- Tightened the TUI visible slash command surface to launch-focused commands.
- Removed/hid legacy or experimental TUI command dispatch/help for `/fork`, `/copy`, `/status`, `/eval`, `/improve`, `/mana`, `/scope`, `/run`, `/plan`, `/clean`, `/queue`, `/personality`, `/memory`, checkpoints, and export/session aliases.
- Removed CLI chat shell surface and dead chat-shell runtime/parser code.
- Removed CLI personality/soul editor surface and helpers.
- Removed CLI eval candidate surface and TUI eval command path.
- Stopped default prompt assembly from injecting personality, soul, memory, or user-profile blocks.
- Updated README to describe the tighter launch product: TUI, one-shot, JSONL RPC, native tools, workflows, durable sessions, auth/secrets, Lua extensions, and SDK preview.
- Fixed `imp-lua` Unix process-group compilation by adding the missing `CommandExt` import and `libc` dependency.

## Verification performed

Final verification passed:

```sh
cargo check -p imp-core -p imp-tui -p imp-cli -p imp-lua
cargo test -p imp-tui command_palette --lib
cargo test -p imp-tui slash_unknown --lib
cargo test -p imp-cli parse_tool_output_display --lib
```

Focused diff reviewed:

```sh
git diff --stat -- README.md Cargo.toml crates/imp-cli/src/lib.rs crates/imp-core/src/builder.rs crates/imp-lua/Cargo.toml crates/imp-lua/src/bridge.rs crates/imp-tui/src/app.rs crates/imp-tui/src/views/command_palette.rs
```

Diff size captured at `/tmp/tighten-imp-product-surface-final.diff` for local review: 2751 lines.

## Remaining concerns

- Some compatibility internals remain staged rather than fully deleted, especially mana/improve-related TUI/backend code and optional CLI mana feature gates. They are no longer part of the visible launch command/help/palette surface.
- `crates/imp-tui/src/views/personality.rs` still exists and is compiled because fully deleting personality UI internals was unsafe in this slice after a failed broad removal attempt. Product access is removed from the visible command surface; deeper file deletion should happen as a separate focused cleanup.
- Memory backend/config code remains for compatibility, but default prompt assembly no longer injects memory or user profile content and the memory command/tool/product surface is removed from active docs/help/palette.

## Suggested next tasks

1. Focused deletion of orphaned personality TUI view and remaining personality backend/config types once no references remain.
2. Focused deletion or compatibility isolation of improve/mana TUI internals after workflow runner replacement is ready.
3. Archive old root/docs experimental artifacts to `~/imp-archive` in a dedicated docs/archive cleanup slice.
4. Implement executable workflow runner via `.imp/workflows/implement-executable-workflow-runner/workflow.yaml` so future workflows can execute build steps directly.

## Candidate experimental/legacy files
crates/imp-cli/src/acp/events.rs
crates/imp-cli/src/acp/mod.rs
crates/imp-cli/src/acp/protocol.rs
crates/imp-core/benches/core_hot_paths.rs
crates/imp-core/examples/reuse-bench.rs
crates/imp-core/examples/sdk_session.rs
crates/imp-core/examples/tool_ab_harness.rs
crates/imp-core/examples/tool_surface_live.rs
crates/imp-core/src/agent/mana_loop.rs
crates/imp-core/src/agent/workflow_integration/mana_compat.rs
crates/imp-core/src/eval_candidate_closeout.rs
crates/imp-core/src/eval_candidate.rs
crates/imp-core/src/mana_next/ledger.rs
crates/imp-core/src/mana_next/mod.rs
crates/imp-core/src/mana_prompt_context.rs
crates/imp-core/src/mana_review.rs
crates/imp-core/src/mana_run_state.rs
crates/imp-core/src/mana_worker.rs
crates/imp-core/src/memory.rs
crates/imp-core/src/personality.rs
crates/imp-core/src/tools/mana.rs
crates/imp-core/src/tools/memory.rs
crates/imp-gui/Cargo.toml
crates/imp-gui/README.md
crates/imp-gui/src/lib.rs
crates/imp-gui/src/main.rs
crates/imp-tui/src/views/mana_navigator.rs

## TODO/FIXME markers
README.md:3:Local terminal coding agent in Rust. imp runs through the TUI, one-shot prompts, or a JSONL RPC protocol. It uses structured tools, durable sessions, and file-backed workflows for planned, inspectable development work.
README.md:53:- preview Rust SDK
README.md:210:Current storage is local and file-backed. API-addressable workflows are planned.
README.md:266:Output includes agent, tool, stream, runtime event, and runtime state payloads. `--runtime-json` emits the shared runtime event/state shape alongside legacy JSON fields.
README.md:403:- Rust SDK preview
README.md:405:Preview/planned:
README.md:408:- MCP planned
README.md:409:- `.imp/agents` planned
README.md:411:- hosted sync/team collaboration planned
README.md:412:- workflow API planned
README.md:414:Compatibility/legacy:
README.md:416:- legacy `mana` integration is optional and compatibility-oriented
README.md:417:- TypeScript/Pi extension compatibility is experimental and not a shipped extension surface
docs/workflows.md:3:imp workflows are local project artifacts for planned, multi-step work. They keep the plan, execution state, checks, prototype results, events, and closeout notes in files under the project.
docs/workflows.md:126:- API-addressable workflows are planned, not shipped.
crates/imp-llm/src/model.rs:494:            id: "kimi-k2-0905-preview".into(),
crates/imp-llm/src/model.rs:507:            id: "kimi-k2-turbo-preview".into(),
crates/imp-llm/src/model.rs:614:            id: "google/gemini-3.1-flash-lite-preview".into(),
crates/imp-llm/src/model.rs:627:            id: "google/gemini-3-flash-preview".into(),
crates/imp-llm/src/model.rs:920:            name: "GPT-4o (legacy custom)".into(),
crates/imp-llm/src/model.rs:938:            name: "o3 (legacy custom)".into(),
crates/imp-llm/src/model.rs:956:            name: "o4-mini (legacy custom)".into(),
crates/imp-llm/src/model.rs:974:            name: "GPT-5.3 Codex Spark (preview)".into(),
crates/imp-llm/src/model.rs:1097:        ("kimi-k2".into(), "kimi-k2-0905-preview".into()),
crates/imp-llm/src/model.rs:1098:        ("kimi-k2-0905".into(), "kimi-k2-0905-preview".into()),
crates/imp-llm/src/model.rs:1099:        ("kimi-k2-turbo".into(), "kimi-k2-turbo-preview".into()),
crates/imp-llm/src/model.rs:1196:    fn resolve_meta_synthesizes_spark_preview() {
crates/imp-llm/src/model.rs:1206:    fn resolve_meta_synthesizes_legacy_openai_model() {
crates/imp-llm/src/model.rs:1210:            .expect("legacy openai model should synthesize");
crates/imp-llm/src/model.rs:1240:        assert_eq!(model.id, "kimi-k2-turbo-preview");
docs/architecture.md:28:- preview Rust SDK
docs/proposals/tool-review-2026-04.md:170:| 6 | Delete dead code (old tools) | Low — cleanup |
docs/imp-next-workflow-runtime.md:378:- planned approach or reason no plan was needed
docs/imp-next-workflow-runtime.md:614:Delegation should create child workflow runs with durable parent links, status, evidence refs, cancellation, and stale/blocked detection. TUI should visualize child runs through the same event/state API.
docs/imp-next-workflow-runtime.md:627:tool.planned
docs/proposals/guest-runtime-implementation-plan.md:48:- deny-by-default behavior for new capabilities, preserving existing Lua compatibility behind an explicit legacy/default profile if needed.
docs/proposals/guest-runtime-implementation-plan.md:69:- manifest-based Lua packages are accepted alongside legacy files/directories;
docs/proposals/guest-runtime-implementation-plan.md:73:- compatibility fixtures for legacy `.lua` files and `init.lua` directories;
docs/proposals/guest-runtime-implementation-plan.md:111:- security review before enabling beyond local experimental profiles.
docs/design/imp-work-mana-feature-parity.md:32:| Fail/blocker evidence | `fail_task_with_conventions` emits blocker memory | partial | Store reason, next action, checks, artifacts, and recovery hints; render in next/tree. | Gas City terminal result taxonomy: success, provider_error, deadline, canceled, panic_recovered. |
docs/design/imp-work-mana-feature-parity.md:46:| Claim/owner semantics | leases and claim action exist | partial | Add expiry/heartbeat/stale lease recovery and double-claim prevention tests. | Gas City tests explicitly cover double-claim conflicts and rollback behavior. |
docs/design/imp-work-mana-feature-parity.md:94:3. **Plan serially, execute concurrently, commit serially.** This is the strongest pattern for imp-work multi-agent runs. Build dependency waves deterministically, run workers in bounded parallelism, then apply state transitions in planned order.
docs/design/imp-work-mana-feature-parity.md:112:- `mana` tool usage in imp is narrowed to import-only/legacy mode.
docs/design/dirac-inspired-code-tools.md:12:- stale-safe anchored edits;
docs/design/dirac-inspired-code-tools.md:51:Anchors should be requested when the next action is likely to replace a range and stale-file safety matters.
docs/design/dirac-inspired-code-tools.md:78:- reject stale anchors;
docs/design/dirac-inspired-code-tools.md:86:`multi_edit` is legacy compatibility only.
crates/imp-llm/src/auth.rs:1402:    fn provider_lookup_candidates_include_legacy_render_casing() {
crates/imp-gui/src/lib.rs:112:    diff_preview: String,
crates/imp-gui/src/lib.rs:168:            diff_preview: "RuntimeStateSnapshot -> GuiRuntimeViewModel\n  phase\n  tools\n  worktree\n  evidence\n".to_owned(),
crates/imp-gui/src/lib.rs:285:                        code_block(ui, &mut self.diff_preview);
crates/imp-gui/src/lib.rs:355:                args_preview: Some("cargo test -p imp-gui".into()),
docs/proposals/mana-wiki-schema-and-workflow.md:264:- Flagged: fact 112 (cited in systems/orchestration.md) is stale
docs/proposals/mana-wiki-schema-and-workflow.md:441:  warnings alongside stale facts.
crates/imp-tui/src/views/settings.rs:1728:                WriteOverwritePolicy::BlockStale => "block stale",
crates/imp-tui/src/views/settings.rs:2206:    fn current_field_clamps_stale_selection() {
crates/imp-gui/README.md:5:This crate starts as a standalone shell for the imp workbench UI: project status, mana work navigation, execution timeline, inspector, diff preview, and terminal output panes. Runtime integration will be added incrementally after the layout and app boundary are stable.
docs/design/imp-work-implementation-plan.md:15:- `context_pack`: deterministic task/prototype context compiler, renderer, stable-prefix hashes, stale detection
docs/design/imp-work-implementation-plan.md:108:- TypeScript prefers `node --experimental-strip-types`, then bun, then deno
docs/design/imp-work-implementation-plan.md:114:- prepared prototypes can record learnings/followups against a parent task and stale related parent context packs
docs/design/imp-work-implementation-plan.md:135:Context packs should be immutable by version, deterministic, source-referenced, stale-detectable, and cache-stable.
docs/design/imp-work-implementation-plan.md:177:- stale parent task context packs when prototype learnings/followups change context inputs
docs/design/imp-work-implementation-plan.md:179:Prepared task workers follow the same pattern: launch from a rendered context pack, return `WorkOutcome`, persist run/outcome/summary/memory updates/follow-up task seeds, and stale related context packs when outcome updates change context inputs.
docs/design/imp-work-implementation-plan.md:194:- context-pack readiness and staleness checks
docs/design/imp-work-implementation-plan.md:204:.imp/work/prototypes.md  # planned/running/observed/promoted/discarded prototypes and observations
docs/design/imp-work-implementation-plan.md:228:- `next`: list scheduler-ready tasks. With `require_context: true`, only returns ready tasks with a loadable, non-stale task-owned context pack.
docs/design/imp-work-implementation-plan.md:232:- `refresh_context`: create a next-version task context pack from the current one, relink the task, and mark the previous context stale.
docs/design/imp-work-implementation-plan.md:235:- `outcome`: record a structured task outcome, update task status, persist run/outcome/summary/memory updates/follow-up task seeds, stale related contexts, and release persisted leases.
crates/imp-tui/src/views/startup.rs:360:pub fn truncate_preview(text: &str, max_lines: usize, max_chars: usize) -> String {
crates/imp-tui/src/views/startup.rs:379:    let mut preview = lines.join("\n");
crates/imp-tui/src/views/startup.rs:381:        if !preview.is_empty() {
crates/imp-tui/src/views/startup.rs:382:            preview.push('\n');
crates/imp-tui/src/views/startup.rs:384:        preview.push_str("[… truncated preview]");
crates/imp-tui/src/views/startup.rs:386:    preview
crates/imp-tui/src/views/startup.rs:392:        render_section_line, summarize_inline, summarize_lines, truncate_preview,
crates/imp-tui/src/views/startup.rs:437:    fn truncate_preview_marks_truncation() {
crates/imp-tui/src/views/startup.rs:439:        let preview = truncate_preview(text, 2, 32);
crates/imp-tui/src/views/startup.rs:440:        assert_eq!(preview, "a\nb\n[… truncated preview]");
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:38:### What is planned but not built
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:220:| "fact 112 is stale" | Knowledge (warning) | Warning in status row or notification | Until re-verified |
crates/imp-llm/src/providers/openai.rs:141:#[allow(dead_code)]
crates/imp-llm/src/providers/openai.rs:621:#[allow(dead_code)]
crates/imp-tui/src/app.rs:42:#[allow(dead_code)]
crates/imp-tui/src/app.rs:2450:        let _ = imp_core::storage::reconcile_legacy_into_global_root();
crates/imp-tui/src/app.rs:3779:                .queued(self.queued_message_preview(area.width))
crates/imp-tui/src/app.rs:5664:    fn queued_message_preview(&self, terminal_width: u16) -> Option<String> {
crates/imp-tui/src/app.rs:5668:            &single_line_preview(text),
crates/imp-tui/src/app.rs:5960:    #[allow(dead_code)]
crates/imp-tui/src/app.rs:5982:    #[allow(dead_code)]
crates/imp-tui/src/app.rs:8566:        self.handle_agent_event_legacy(event)
crates/imp-tui/src/app.rs:8569:    fn handle_agent_event_legacy(&mut self, event: AgentEvent) {
crates/imp-tui/src/app.rs:9009:fn single_line_preview(text: &str) -> String {
crates/imp-tui/src/app.rs:10141:            "active replacement handle should survive stale completion"
crates/imp-tui/src/app.rs:10192:            "active replacement handle should survive stale failure"
crates/imp-tui/src/views/editor.rs:410:    queued_preview: Option<String>,
crates/imp-tui/src/views/editor.rs:440:            queued_preview: None,
crates/imp-tui/src/views/editor.rs:493:    pub fn queued(mut self, preview: Option<String>) -> Self {
crates/imp-tui/src/views/editor.rs:494:        self.queued_preview = preview;
crates/imp-tui/src/views/editor.rs:562:        let prompt_activity_state = if self.queued_preview.is_some() {
crates/imp-tui/src/views/editor.rs:668:        if let Some(preview) = self.queued_preview.as_deref() {
crates/imp-tui/src/views/editor.rs:671:                let label = format!("{} queued {}", queued_glyph(), preview);
crates/imp-tui/src/views/editor.rs:766:    let preview = truncate_display_width(first, 48);
crates/imp-tui/src/views/editor.rs:768:    Some(format!("[{preview} + {extra_lines} lines]"))
docs/proposals/mana-aware-runtime-context-read-path.md:98:**Risk:** If wiki pages or facts become stale during a very long session,
docs/proposals/mana-aware-runtime-context-read-path.md:157:as an additional system prompt section. This surfaces stale facts,
crates/imp-cli/src/acp/mod.rs:476:#[allow(dead_code)]
crates/imp-tui/src/views/ask_bar.rs:695:    fn tab_to_edit_clamps_stale_option_cursor() {
crates/imp-tui/src/views/ask_bar.rs:722:    fn confirm_clamps_stale_selected_cursor() {
crates/imp-tui/src/views/ask_bar.rs:736:    fn cursor_screen_position_handles_tiny_area_and_stale_cursor() {
docs/proposals/imp-memory-architecture-and-mana-boundary.md:116:verify gates, verified facts with TTL/staleness, attempt logs, notes,
docs/proposals/imp-memory-architecture-and-mana-boundary.md:137:- `mana context` (no ID) outputs project-wide memory context (stale facts, claimed units, recent work).
docs/design/oss-launch-checklist.md:56:- [ ] Add a "What is experimental" section.
docs/design/oss-launch-checklist.md:74:- [ ] Make sure old mana language is removed or clearly marked legacy.
docs/design/oss-launch-checklist.md:154:- [ ] `docs/extensibility/agents.md`: `.imp/agents` if available or planned.
docs/design/oss-launch-checklist.md:168:- [ ] Docs avoid stale future-tense promises unless clearly marked planned.
docs/design/oss-launch-checklist.md:169:- [ ] Docs use native imp-work vocabulary, not mana, except in migration/legacy notes.
docs/design/oss-launch-checklist.md:240:- [ ] Verify context pack creation/refresh/stale detection.
docs/design/oss-launch-checklist.md:312:- [ ] Audit root directory for stale plans, experiments, and duplicate docs.
docs/design/oss-launch-checklist.md:314:- [ ] Make launch docs distinguish stable, experimental, and legacy.
crates/imp-llm/src/providers/anthropic.rs:189:        #[allow(dead_code)]
crates/imp-llm/src/providers/anthropic.rs:196:        #[allow(dead_code)]
crates/imp-llm/src/providers/anthropic.rs:201:        #[allow(dead_code)]
crates/imp-llm/src/providers/anthropic.rs:219:        #[allow(dead_code)]
crates/imp-llm/src/providers/anthropic.rs:253:#[allow(dead_code)]
crates/imp-llm/src/providers/anthropic.rs:262:#[allow(dead_code)]
crates/imp-llm/src/providers/anthropic.rs:280:#[allow(dead_code)]
crates/imp-tui/src/views/welcome.rs:1046:    fn selected_provider_and_step_clamp_stale_indices() {
crates/imp-tui/src/views/sidebar.rs:909:    summary = truncated_scalar_preview(&summary, MAX_TEXT_CHARS);
crates/imp-tui/src/views/sidebar.rs:916:fn truncated_scalar_preview(value: &str, max_chars: usize) -> String {
crates/imp-llm/src/providers/openai_compat.rs:1058:    fn kimi_legacy_preview_omits_thinking_control() {
crates/imp-llm/src/providers/openai_compat.rs:1059:        let model = test_model_for_provider("kimi-k2-turbo-preview", "moonshot");
crates/imp-tui/src/views/session_picker.rs:228:        let has_preview = inner.width >= 88;
crates/imp-tui/src/views/session_picker.rs:229:        let columns = if has_preview {
crates/imp-tui/src/views/session_picker.rs:241:        let preview_area = columns[1];
crates/imp-tui/src/views/session_picker.rs:249:            if has_preview {
crates/imp-tui/src/views/session_picker.rs:250:                render_preview_empty(preview_area, buf, self.theme);
crates/imp-tui/src/views/session_picker.rs:263:            if has_preview {
crates/imp-tui/src/views/session_picker.rs:264:                render_preview_empty(preview_area, buf, self.theme);
crates/imp-tui/src/views/session_picker.rs:270:        if has_preview {
crates/imp-tui/src/views/session_picker.rs:271:            render_session_preview(preview_area, self.state.selected_session(), buf, self.theme);
crates/imp-tui/src/views/session_picker.rs:298:        let preview = session
crates/imp-tui/src/views/session_picker.rs:320:        let preview_width = area.width.saturating_sub(6) as usize;
crates/imp-tui/src/views/session_picker.rs:324:        let preview = truncate(&preview, preview_width);
crates/imp-tui/src/views/session_picker.rs:346:            let preview_line = Line::from(vec![
crates/imp-tui/src/views/session_picker.rs:348:                Span::styled(preview, theme.muted_style()),
crates/imp-tui/src/views/session_picker.rs:350:            buf.set_line(area.x, base_y + 2, &preview_line, area.width);
crates/imp-tui/src/views/session_picker.rs:369:fn render_preview_empty(area: Rect, buf: &mut Buffer, theme: &Theme) {
crates/imp-tui/src/views/session_picker.rs:385:fn render_session_preview(
docs/reference-monitor-policy.md:369:Compatibility checkpoint labels currently map selected policy codes to legacy
docs/reference-monitor-policy.md:525:Compatibility checkpoint labels currently map selected policy codes to legacy
docs/design/imp-host-sync-mirror-daemon.md:564:A run request should be idempotent. A daemon may crash; leases expire and the request can be retried or marked stale.
crates/imp-tui/src/views/tree.rs:323:        let has_preview = inner.width >= 140 && inner.height >= 18;
crates/imp-tui/src/views/tree.rs:324:        let columns = if has_preview {
crates/imp-tui/src/views/tree.rs:342:            has_preview,
crates/imp-tui/src/views/tree.rs:344:        if has_preview {
crates/imp-tui/src/views/tree.rs:345:            render_tree_preview(columns[1], self.state.selected_node(), buf, self.theme);
crates/imp-tui/src/views/tree.rs:442:fn render_tree_preview(area: Rect, node: Option<&FlatTreeNode>, buf: &mut Buffer, theme: &Theme) {
docs/mana-next-migration-test-plan.md:237:- stale facts do not map to high-trust workflow evidence without verification
crates/imp-llm/src/providers/openai_codex.rs:117:            "responses=experimental".to_string(),
crates/imp-cli/src/usage_report.rs:343:        legacy_records: records
crates/imp-cli/src/usage_report.rs:600:        (None, None) => "(legacy usage with unknown model)".to_string(),
docs/worktree-auto.md:320:| branch/path collision | fail closed; suggest keep/recover/remove stale worktree |
docs/typescript-extension-bridge.md:145:those tools should be treated as legacy compatibility tools until they are
docs/typescript-extension-bridge.md:401:  `policy-disabled`, `legacy compatibility`)
docs/typescript-extension-bridge.md:403:- compatibility debt from legacy Pi APIs where applicable
docs/typescript-extension-bridge.md:475:2. Extend discovery to prefer `imp.extension.json` manifests and keep legacy Pi
crates/imp-cli/src/lib.rs:207:    /// Emit shared runtime_event/runtime_state payloads alongside legacy JSON events
crates/imp-cli/src/lib.rs:277:    /// Open the viewer/inspector surface (planned; not fully implemented yet)
crates/imp-cli/src/lib.rs:279:        /// Viewer area to open (planned: sessions, tree, logs, checkpoints)
crates/imp-cli/src/lib.rs:688:    legacy_records: usize,
crates/imp-cli/src/lib.rs:1365:    let _ = imp_core::storage::reconcile_legacy_into_global_root();
crates/imp-cli/src/lib.rs:1490:    let _ = imp_core::storage::reconcile_legacy_into_global_root();
crates/imp-cli/src/lib.rs:1639:    let _ = imp_core::storage::reconcile_legacy_into_global_root();
crates/imp-cli/src/lib.rs:1695:    let _ = imp_core::storage::reconcile_legacy_into_global_root();
crates/imp-cli/src/lib.rs:1705:    let _ = imp_core::storage::reconcile_legacy_into_global_root();
crates/imp-cli/src/lib.rs:1741:    let _ = imp_core::storage::reconcile_legacy_into_global_root();
crates/imp-cli/src/lib.rs:1926:    let _ = imp_core::storage::reconcile_legacy_into_global_root();
crates/imp-cli/src/lib.rs:1959:    let _ = imp_core::storage::reconcile_legacy_into_global_root();
crates/imp-cli/src/lib.rs:2472:    let value = legacy_json_event_value(event)?;
crates/imp-cli/src/lib.rs:2509:fn legacy_json_event_value(event: &AgentEvent) -> Result<Value, Box<dyn std::error::Error>> {
crates/imp-cli/src/lib.rs:3215:    let mut value = rpc_agent_event_legacy_json(event);
crates/imp-cli/src/lib.rs:3229:fn rpc_agent_event_legacy_json(event: &AgentEvent) -> Value {
crates/imp-cli/src/lib.rs:4492:            .expect("legacy native work run flags are no longer a subcommand");
crates/imp-tui/src/views/tool_output.rs:1755:                "status": "planned",
crates/imp-tui/src/views/tool_output.rs:1781:            assert!(plain.iter().any(|line| line.contains("status: planned")));
docs/trust-labels-and-provenance.md:110:Saved user/project memory and prior summaries. Useful context, but stale or
docs/trust-labels-and-provenance.md:112:Memory carries age/staleness metadata where available.
docs/trust-labels-and-provenance.md:139:- `stale`
docs/workflow-first-ux.md:208:should describe it as planned or advanced rather than implying every role flow is
docs/design/droid-gap-map-and-imp-roadmap.md:128:- planned tasks and dependencies
docs/design/droid-gap-map-and-imp-roadmap.md:355:- a task with stale context must refresh before execution
docs/child-workflow-delegation.md:31:7. Support cancellation, stale/blocked detection, and safe worktree boundaries.
docs/child-workflow-delegation.md:71:- cancellation/stale metadata
docs/child-workflow-delegation.md:88:    pub stale: Option<ChildStaleState>,
docs/child-workflow-delegation.md:316:A child is stale when it has not emitted progress after its idle timeout, or when
docs/child-workflow-delegation.md:336:Do not silently retry stale children forever. Parent integration decides whether
docs/child-workflow-delegation.md:410:- show blocked/stale reason
docs/child-workflow-delegation.md:423:- cancellation command path and stale/cancel policy decisions
docs/child-workflow-delegation.md:445:7. Cancellation and stale detection.
docs/release-promotions/commit-board.html:59:<script id="commit-data" type="application/json">[{&quot;sha&quot;: &quot;4e7f7464e1ef12b17dee43636fdfdebf8385ad59&quot;, &quot;short&quot;: &quot;4e7f746&quot;, &quot;subject&quot;: &quot;Reduce imp TUI startup latency&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-19T11:38:14-07:00&quot;, &quot;side&quot;: &quot;nightly-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/mana_prompt_context.rs&quot;, &quot;crates/imp-llm/src/auth.rs&quot;, &quot;crates/imp-llm/src/providers/openai.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 214, &quot;deletions&quot;: 43, &quot;risk_score&quot;: 10, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;moderate churn (257 lines)&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-llm/src/providers&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;d6521026f113f6fe80b5f55150cf66658190289f&quot;, &quot;short&quot;: &quot;d652102&quot;, &quot;subject&quot;: &quot;Prepare vanilla imp release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T14:02:13-07:00&quot;, &quot;side&quot;: &quot;nightly-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;.mana/.3-set-up-harbor-adapter-and-terminal-bench-20-runner.md&quot;, &quot;.mana/.5-add-safe-automatic-context-compaction-for-long-run.md&quot;, &quot;.mana/.5.1-add-disabled-by-default-auto-compaction-config-sca.md&quot;, &quot;.mana/.6-hardening-pass-reduce-bugs-and-contract-mismatches.md&quot;, &quot;.mana/.6.6-enforce-lua-extension-capability-boundaries.md&quot;, &quot;.mana/.6.7-propagate-cancellation-into-active-tool-execution.md&quot;, &quot;.mana/.6.8-align-diff-tool-registration-with-mode-contracts.md&quot;, &quot;.mana/.9-upgrade-imp-mana-authoring-prompt-contract-for-orc.md&quot;, &quot;.mana/.gitignore&quot;, &quot;.mana/21-imp-efficiency-smarter-tool-output-truncation.md&quot;, &quot;.mana/245.1-define-manaimp-contract-implications-of-file-nativ.md&quot;, &quot;.mana/245.1.1-define-vnext-manaimp-subagent-handoff-packet-for-o.md&quot;, &quot;.mana/248-comprehensive-imp-uiux-review-upgrade-and-polish-a.md&quot;, &quot;.mana/248.14-implement-restrained-ansi-emphasis-for-shell-typog.md&quot;, &quot;.mana/248.16.5-create-svg-wireframes-for-candidate-imp-tui-layout.md&quot;, &quot;.mana/248.16.7-revise-imp-tui-wireframes-and-core-memo-after-user.md&quot;, &quot;.mana/248.17-design-terminal-emulator-native-coding-agent-cockp.md&quot;, &quot;.mana/248.18-harden-and-humanize-imp-error-streaming-across-pro.md&quot;, &quot;.mana/248.18.1-extract-shared-imp-core-streamed-error-normalizati.md&quot;, &quot;.mana/248.18.2-harden-imp-core-partial-stream-and-silent-eof-diag.md&quot;, &quot;.mana/248.18.3-design-stable-machine-facing-streamed-error-envelo.md&quot;, &quot;.mana/248.7-plan-shared-uxruntime-seams-for-shell-tui-and-view.md&quot;, &quot;.mana/248.9-capture-and-sequence-real-user-feedback-on-the-new.md&quot;, &quot;.mana/249-reduce-duplicate-verbose-mana-change-narration-in.md&quot;, &quot;.mana/250-shape-getimpdev-landing-page-direction-and-impleme.md&quot;, &quot;.mana/254-fresh-smoke-trial-for-native-imp-run-on-an-isolate.md&quot;, &quot;.mana/256-run-one-shot-native-imp-print-smoke-before-imp-run.md&quot;, &quot;.mana/257-draft-imp-ontologymd-for-shared-featureruntime-lan.md&quot;, &quot;.mana/259-audit-panic-usage-and-detached-task-failure-policy.md&quot;, &quot;.mana/263-audit-and-isolate-library-level-stderr-writes-that.md&quot;, &quot;.mana/263.2-classify-mana-core-stderr-writes-for-embedded-risk.md&quot;, &quot;.mana/264-normalize-imp-storage-topology-for-sessions-config.md&quot;, &quot;.mana/264.1-audit-current-imp-durable-storage-surfaces-and-pat.md&quot;, &quot;.mana/264.2-specify-normalized-imp-storage-contract-and-migrat.md&quot;, &quot;.mana/264.3.1-add-shared-imp-core-storage-path-module-for-canoni.md&quot;, &quot;.mana/264.3.2-migrate-config-auth-session-and-session-search-cal.md&quot;, &quot;.mana/264.3.3-migrate-instruction-discovery-to-canonical-impagen.md&quot;, &quot;.mana/264.3.4-implement-non-destructive-migration-from-legacy-im.md&quot;, &quot;.mana/264.4-audit-and-fix-imp-session-index-lifecycle-wiring-f.md&quot;, &quot;.mana/264.6-decide-canonical-imp-filesystem-roots-for-global-a.md&quot;, &quot;.mana/264.7-specify-precedence-and-merge-rules-between-imp-and.md&quot;, &quot;.mana/264.8-specify-migration-from-xdgmacos-legacy-paths-into.md&quot;, &quot;.mana/266-cross-codebase-review-compare-imp-and-hermes-agent.md&quot;, &quot;.mana/266.1-design-adoption-path-provider-resilience-and-auth.md&quot;, &quot;.mana/266.2-design-adoption-path-session-recall-memory-and-con.md&quot;, &quot;.mana/266.3-design-adoption-path-extension-seams-and-product-s.md&quot;, &quot;.mana/266.4-imp-vnext-ranked-roadmap-and-phased-execution-plan.md&quot;, &quot;.mana/266.4.7-phase-5-epic-selective-later-product-surface-expan.md&quot;, &quot;.mana/267-adopt-highest-value-product-lessons-from-opencode.md&quot;, &quot;.mana/268.1-diagnose-native-imp-mana-tool-divergence-from-cli.md&quot;, &quot;.mana/27-improve-mana-pool-competitive-grade-dispatch-engin.md&quot;, &quot;.mana/27.14-define-attempt-scoped-autonomy-observation-record.md&quot;, &quot;.mana/27.2-imp-ui-compact-mana-statusprogress-surface.md&quot;, &quot;.mana/271-add-native-youtube-video-interpretation-support-to.md&quot;, &quot;.mana/271.1-implement-pure-http-youtube-transcript-extraction.md&quot;, &quot;.mana/271.2-harden-imp-spawn-and-mana-closetool-execution-agai.md&quot;, &quot;.mana/272-add-native-video-context-ingestion-architecture-fo.md&quot;, &quot;.mana/272.1-implement-pure-http-youtube-transcript-extraction.md&quot;, &quot;.mana/272.2-design-richer-video-interpretation-beyond-transcri.md&quot;, &quot;.mana/273-diagnose-and-harden-kimi-code-oauth-model-routing.md&quot;, &quot;.mana/273.5-sprint-import-and-execute-pi-typescript-extensions.md&quot;, &quot;.mana/273.5.10-prove-bun-ts-adapter-against-local-pi-color-palett.md&quot;, &quot;.mana/273.5.11-add-official-pi-dynamic-tools-compatibility-fixtur.md&quot;, &quot;.mana/273.5.12-define-sprint-1-typescriptpi-extension-support-bou.md&quot;, &quot;.mana/273.5.13-probe-dependency-bearing-pi-extension-compatibilit.md&quot;, &quot;.mana/273.5.4-normalize-typeboxjson-schemas-from-typescript-exte.md&quot;, &quot;.mana/275-assess-and-sequence-next-llm-oauth-provider-integr.md&quot;, &quot;.mana/275.10-inventory-pi-and-imp-provideroauth-surfaces.md&quot;, &quot;.mana/275.11-sequence-pi-provideroauth-parity-implementation.md&quot;, &quot;.mana/275.6-assess-pi-google-antigravity-provider-route-for-im.md&quot;, &quot;.mana/275.9-research-unofficial-cursor-provider-support-for-im.md&quot;, &quot;.mana/276-investigate-and-harden-tui-esc-cancellation-for-hu.md&quot;, &quot;.mana/277-fix-imp-tui-clean-ui-corruption-and-string-join-ov.md&quot;, &quot;.mana/278-fix-inspector-mode-interaction-model.md&quot;, &quot;.mana/28.1-make-imp-run-the-canonical-mana-worker-runtime-whi.md&quot;, &quot;.mana/28.5.1-patch-imp-system-prompt-with-mana-first-planning-d.md&quot;, &quot;.mana/28.5.6-implement-turn-scoped-mana-review-packet-aggregati.md&quot;, &quot;.mana/28.5.7-render-between-turn-mana-review-packets-across-imp.md&quot;, &quot;.mana/28.5.7.1-add-shared-imp-core-turnmanadelta-renderer-and-man.md&quot;, &quot;.mana/28.5.7.2-render-compact-between-turn-mana-block-and-textual.md&quot;, &quot;.mana/28.5.7.3-render-between-turn-mana-review-packets-in-imp-cli.md&quot;, &quot;.mana/28.5.7.4-add-shared-manareviewmode-config-and-presentation.md&quot;, &quot;.mana/28.5.7.5-wire-imp-tui-compact-widget-tray-block-and-sidebar.md&quot;, &quot;.mana/280-review-project-gaps-that-would-make-imp-stronger-t.md&quot;, &quot;.mana/280.1-run-dirac-evals-with-imp-using-gemini-secret.md&quot;, &quot;.mana/280.2-adopt-dirac-inspired-code-intelligence-and-precise.md&quot;, &quot;.mana/280.2.1.1-decide-migration-safe-naming-strategy-for-imp-scan.md&quot;, &quot;.mana/280.2.2-implement-read-oriented-symbol-extraction-and-skel.md&quot;, &quot;.mana/280.2.3-add-anchor-backed-read-and-stale-safe-edit-flow-to.md&quot;, &quot;.mana/280.2.4-implement-edit-transaction-batching-with-combined.md&quot;, &quot;.mana/282-design-native-scoped-secret-injection-for-command.md&quot;, &quot;.mana/285-verify-installed-imp-binary-includes-latest-secret.md&quot;, &quot;.mana/290-complete-imp-codebase-quality-audit.md&quot;, &quot;.mana/290.1-split-imp-tui-apprs-by-responsibility.md&quot;, &quot;.mana/290.2-split-imp-core-agentrs-into-focused-runtime-module.md&quot;, &quot;.mana/290.3-split-imp-cli-librs-into-command-modules.md&quot;, &quot;.mana/290.4-split-native-mana-tool-implementation-into-focused.md&quot;, &quot;.mana/291-rewrite-imp-readme-around-product-features-mana-an.md&quot;, &quot;.mana/31.2-add-guardrail-config-types-and-profile-selection-t.md&quot;, &quot;.mana/31.3-integrate-guardrails-into-the-imp-system-prompt-an.md&quot;, &quot;.mana/31.4-add-the-initial-zig-guardrail-profile-and-document.md&quot;, &quot;.mana/33-chat-view-replace-duplicated-animation-logic-with.md&quot;, &quot;.mana/34-sidebar-detail-header-use-spinnerframe-and-respect.md&quot;, &quot;.mana/35-editor-remove-dead-tick-and-animationlevel-params.md&quot;, &quot;.mana/36-animation-config-reconcile-minimal-namingdocs-afte.md&quot;, &quot;.mana/37-add-first-class-usage-accounting-and-reporting-to.md&quot;, &quot;.mana/37.5-test-and-document-imp-usage-accountingreporting.md&quot;, &quot;.mana/41-anthropic-api-parity-adopt-claude-code-patterns-fo.md&quot;, &quot;.mana/44-define-memory-and-code-intelligence-architecture-f.md&quot;, &quot;.mana/44.1-author-guest-runtime-extension-substrate-proposal.md&quot;, &quot;.mana/44.1.10-implement-documentworkspace-symbols-with-ast-first.md&quot;, &quot;.mana/44.1.11-implement-hover-and-signature-help-on-the-phase-1.md&quot;, &quot;.mana/44.1.12-unify-code-intelligence-diagnostic-summaries-with.md&quot;, &quot;.mana/44.1.14-evaluate-whether-repeated-evidence-promotion-flows.md&quot;, &quot;.mana/44.1.5-plan-guarded-write-oriented-semantic-actions-and-p.md&quot;, &quot;.mana/44.1.5.5-specify-semantic-write-execution-contract-for-prev.md&quot;, &quot;.mana/44.1.6-sequence-phase-1-read-oriented-imp-code-intelligen.md&quot;, &quot;.mana/44.1.6.1-define-shared-normalization-envelopes-for-read-ori.md&quot;, &quot;.mana/44.1.6.2-plan-diagnostics-plus-ast-alignment-for-the-first.md&quot;, &quot;.mana/44.1.6.3-plan-document-symbols-and-go-to-definition-over-th.md&quot;, &quot;.mana/44.1.6.4-plan-references-and-workspace-symbol-browsing-for.md&quot;, &quot;.mana/44.1.6.5-plan-hover-and-signature-enrichment-after-core-rea.md&quot;, &quot;.mana/44.1.7-roll-out-phase-1-read-oriented-imp-code-intelligen.md&quot;, &quot;.mana/44.1.8-normalize-read-oriented-code-intelligence-queryres.md&quot;, &quot;.mana/44.1.9-implement-phase-1-diagnostics-go-to-definition-and.md&quot;, &quot;.mana/44.3-translate-guest-runtime-design-into-phased-impleme.md&quot;, &quot;.mana/45-tower-rebuild-around-explicit-contracts-durable-le.md&quot;, &quot;.mana/45.10.5-update-docs-for-mana-platform-substrate-and-imp-pr.md&quot;, &quot;.mana/45.11-capture-near-term-imp-execution-lanes-under-the-im.md&quot;, &quot;.mana/45.11.1-resolve-consequential-defaults-for-near-term-imp-i.md&quot;, &quot;.mana/45.11.1.1-clarify-whether-native-rust-not-lua-applies-to-imp.md&quot;, &quot;.mana/45.11.1.2-sequence-near-term-imp-implementation-order-from-s.md&quot;, &quot;.mana/45.4-phase-3-introduce-runner-protocol-and-local-adapte.md&quot;, &quot;.mana/45.4.2-plan-the-first-imp-local-runner-adapter-that-consu.md&quot;, &quot;.mana/45.4.4-plan-the-cutover-from-current-imp-run-plus-mana-ru.md&quot;, &quot;.mana/45.5-phase-4-rebuild-imp-around-stable-workerruntime-se.md&quot;, &quot;.mana/45.5.1-map-imp-core-hotspots-into-target-runtime-context.md&quot;, &quot;.mana/45.5.3-write-a-compact-imp-decomposition-order-for-post-c.md&quot;, &quot;.mana/45.7-phase-6-harden-policy-isolation-and-migration-surf.md&quot;, &quot;.mana/45.7.4-evaluate-whether-imp-should-add-a-native-gitrepo-t.md&quot;, &quot;.mana/46-broaden-imp-attention-beyond-toolsprompting-under.md&quot;, &quot;.mana/46.1-reconcile-long-session-runtime-safety-gaps-and-tur.md&quot;, &quot;.mana/46.2-reconcile-user-visible-discoverability-and-ux-brea.md&quot;, &quot;.mana/46.2.1-surface-usage-reporting-in-the-tui-commandhelpstar.md&quot;, &quot;.mana/47-rebuild-imp-around-explicit-runtime-boundaries-pro.md&quot;, &quot;.mana/47.1-contracts-and-ownership-boundary-for-mana-imp-rebu.md&quot;, &quot;.mana/47.6-sequence-the-imp-rebuild-as-an-incremental-migrati.md&quot;, &quot;.mana/50-define-cli-first-operator-surface-for-imp-with-tui.md&quot;, &quot;.mana/50.10-implement-guided-cli-parity-flows-for-settings-per.md&quot;, &quot;.mana/50.10.1-implement-terminal-native-imp-settings-flow-for-cl.md&quot;, &quot;.mana/50.10.1.2-let-imp-chat-no-tools-reach-the-shell-without-prov.md&quot;, &quot;.mana/50.10.2-implement-terminal-native-imp-personality-flow-for.md&quot;, &quot;.mana/50.11-implement-first-shell-to-view-handoff-for-sessions.md&quot;, &quot;.mana/50.11.2-align-imp-chat-view-handoff-with-explicit-imp-view.md&quot;, &quot;.mana/50.12-flip-plain-imp-to-imp-chat-after-shell-readiness-g.md&quot;, &quot;.mana/50.13-plan-extraction-of-shared-fullscreen-consumed-runt.md&quot;, &quot;.mana/50.14-specify-the-shared-imp-ui-request-and-runtime-even.md&quot;, &quot;.mana/50.16-follow-on-cli-native-affordance-stack-after-505-se.md&quot;, &quot;.mana/50.16.1-define-stable-human-vs-machine-output-modes-across.md&quot;, &quot;.mana/50.16.2-plan-cli-first-checkpoint-productization-after-out.md&quot;, &quot;.mana/50.16.3-plan-visible-cli-first-planning-artifacts-and-exec.md&quot;, &quot;.mana/50.16.4-plan-first-class-approval-policy-layer-for-cli-fir.md&quot;, &quot;.mana/50.16.5-surface-session-browsing-and-session-search-as-fir.md&quot;, &quot;.mana/50.16.5.1-audit-and-reconcile-imp-session-storage-and-sessio.md&quot;, &quot;.mana/50.16.6-plan-detachedbackground-local-execution-after-cli.md&quot;, &quot;.mana/50.17-define-stable-human-vs-machine-output-contracts-fo.md&quot;, &quot;.mana/50.18-define-cli-first-session-browsing-and-sessionsearc.md&quot;, &quot;.mana/50.19-define-stable-imp-human-vs-machine-output-contract.md&quot;, &quot;.mana/50.20-plan-first-cli-first-checkpoint-productization-ove.md&quot;, &quot;.mana/50.21-specify-visible-planning-artifacts-and-checklist-b.md&quot;, &quot;.mana/50.22-specify-the-first-visible-planning-workflow-and-pl.md&quot;, &quot;.mana/50.23-specify-cli-first-approval-policy-and-blocked-stat.md&quot;, &quot;.mana/50.24-define-the-first-cli-first-approval-policy-surface.md&quot;, &quot;.mana/50.25-specify-detachedbackground-local-execution-contrac.md&quot;, &quot;.mana/50.26-define-the-first-local-detachedbackground-executio.md&quot;, &quot;.mana/50.6-design-the-cli-first-interactive-shell-path-for-im.md&quot;, &quot;.mana/50.9-implement-the-first-cli-first-proving-slice-with-e.md&quot;, &quot;.mana/51.6.1-audit-current-mana-core-embedding-surface-against.md&quot;, &quot;.mana/65-root-mana-currently-lists-child-513-but-direct-sho.md&quot;, &quot;.mana/69-imp-cli-no-longer-contains-duplicate-mana-unit-loa.md&quot;, &quot;.mana/73-code-intelligence-outputs-are-transient-by-default.md&quot;, &quot;.mana/81-design-imp-native-delegation-tool-around-imp-run-a.md&quot;, &quot;.mana/81.10-define-codemap-backed-context-seam-for-imp-run-and.md&quot;, &quot;.mana/82-assess-gpt-54-pro-support-through-openai-chatgpt-o.md&quot;, &quot;.mana/82.2-add-gpt-54-pro-to-imp-model-registry-only-after-oa.md&quot;, &quot;.mana/83-harden-imp-tui-text-box-cursor-and-bounds-handling.md&quot;, &quot;.mana/RULES.md&quot;, &quot;.mana/archive/2026/03/.2-design-canonical-usage-schema-and-aggregation-help.md&quot;, &quot;.mana/archive/2026/03/16-imp-core-hardening-production-ready-agent-engine.md&quot;, &quot;.mana/archive/2026/03/16.1-wire-config-agent-agentbuilder-thresholds-hooks-re.md&quot;, &quot;.mana/archive/2026/03/16.2-tool-argument-validation-json-schema-before-execut.md&quot;, &quot;.mana/archive/2026/03/16.3-llm-retry-with-exponential-backoff-and-jitter.md&quot;, &quot;.mana/archive/2026/03/16.4-loop-detection-prevent-infinite-tool-call-retry-lo.md&quot;, &quot;.mana/archive/2026/03/16.5-file-not-found-suggestions-with-levenshtein-rankin.md&quot;, &quot;.mana/archive/2026/03/16.6-auto-resume-after-compaction-re-queue-original-pro.md&quot;, &quot;.mana/archive/2026/03/16.7-file-read-tracking-and-staleness-detection.md&quot;, &quot;.mana/archive/2026/03/16.8-file-version-history-pre-edit-snapshots-for-rollba.md&quot;, &quot;.mana/archive/2026/03/17-imp-efficiency-enable-prompt-caching.md&quot;, &quot;.mana/archive/2026/03/19-imp-efficiency-in-session-file-content-cache.md&quot;, &quot;.mana/archive/2026/03/20-imp-efficiency-parallelize-grep-block-search-with.md&quot;, &quot;.mana/archive/2026/03/229-imp-rust-coding-agent-engine.md&quot;, &quot;.mana/archive/2026/03/229.1-workspace-setup-imp-llm-types.md&quot;, &quot;.mana/archive/2026/03/229.10-imp-llm-anthropic-oauth.md&quot;, &quot;.mana/archive/2026/03/229.11-imp-core-hook-system.md&quot;, &quot;.mana/archive/2026/03/229.12-imp-core-tree-sitter-tools-probesearch-probeextrac.md&quot;, &quot;.mana/archive/2026/03/229.13-imp-core-config-resource-discovery.md&quot;, &quot;.mana/archive/2026/03/229.14-imp-core-system-prompt-assembly.md&quot;, &quot;.mana/archive/2026/03/229.15-imp-lua-lua-extension-runtime.md&quot;, &quot;.mana/archive/2026/03/229.16-imp-core-shell-tool-loader.md&quot;, &quot;.mana/archive/2026/03/229.17-imp-tui-ratatui-interactive-mode.md&quot;, &quot;.mana/archive/2026/03/229.18-imp-cli-binary-entry-point.md&quot;, &quot;.mana/archive/2026/03/229.2-imp-llm-anthropic-provider.md&quot;, &quot;.mana/archive/2026/03/229.3-imp-core-tool-trait-file-tools-read-write-edit-mul.md&quot;, &quot;.mana/archive/2026/03/229.4-imp-core-bash-grep-find-tools.md&quot;, &quot;.mana/archive/2026/03/229.5-imp-core-ask-diff-tools.md&quot;, &quot;.mana/archive/2026/03/229.6-imp-core-agent-loop.md&quot;, &quot;.mana/archive/2026/03/229.7-imp-core-session-manager.md&quot;, &quot;.mana/archive/2026/03/229.8-imp-core-context-management-observation-masking-co.md&quot;, &quot;.mana/archive/2026/03/229.9-imp-llm-openai-google-providers.md&quot;, &quot;.mana/archive/2026/03/23-learning-loop-agent-curated-memory-skill-managemen.md&quot;, &quot;.mana/archive/2026/03/23.1-system-prompt-layer-6-wire-memory-into-prompt-asse.md&quot;, &quot;.mana/archive/2026/03/23.2-memory-store-and-memory-tool.md&quot;, &quot;.mana/archive/2026/03/23.3-skill-manage-tool-agent-creates-patches-and-delete.md&quot;, &quot;.mana/archive/2026/03/23.4-learning-nudges-system-prompt-text-and-onagentend.md&quot;, &quot;.mana/archive/2026/03/23.5-session-index-with-fts5-full-text-search.md&quot;, &quot;.mana/archive/2026/03/23.6-session-search-tool.md&quot;, &quot;.mana/archive/2026/03/24-tui-ux-overhaul-information-density-summaries-inte.md&quot;, &quot;.mana/archive/2026/03/24.1-turn-activity-tracker-foundation-for-progress-and.md&quot;, &quot;.mana/archive/2026/03/24.2-progress-indicator-in-status-bar-during-streaming.md&quot;, &quot;.mana/archive/2026/03/24.3-per-tool-call-expandcollapse-and-auto-expand-error.md&quot;, &quot;.mana/archive/2026/03/24.4-turn-end-summary-with-file-change-tracking.md&quot;, &quot;.mana/archive/2026/03/24.5-visual-separation-of-tool-activity-from-assistant.md&quot;, &quot;.mana/archive/2026/03/24.6-editor-polish-placeholder-model-indicator-keybindi.md&quot;, &quot;.mana/archive/2026/03/24.7-fix-context-window-tracking-use-actual-conversatio.md&quot;, &quot;.mana/archive/2026/03/24.8-approval-flow-wire-userinterface-for-tool-confirma.md&quot;, &quot;.mana/archive/2026/03/25-multi-provider-llm-support-with-data-driven-welcom.md&quot;, &quot;.mana/archive/2026/03/25.1-provider-metadata-registry-auth-generalization.md&quot;, &quot;.mana/archive/2026/03/25.2-openai-compatible-chat-completions-provider.md&quot;, &quot;.mana/archive/2026/03/25.3-add-builtin-models-for-new-providers.md&quot;, &quot;.mana/archive/2026/03/25.4-data-driven-welcome-flow.md&quot;, &quot;.mana/archive/2026/03/25.5-generalize-cli-login-for-all-providers.md&quot;, &quot;.mana/archive/2026/03/26-fix-imp-tui-compile-errors-around-toolcallorder-re.md&quot;, &quot;.mana/archive/2026/03/27.1-imp-core-mana-tool-add-native-orchestration-action.md&quot;, &quot;.mana/archive/2026/03/31-add-configurable-engineering-guardrails-to-imp.md&quot;, &quot;.mana/archive/2026/03/37.1-design-canonical-usage-schema-and-aggregation-help.md&quot;, &quot;.mana/archive/2026/03/37.2-persist-canonical-usage-entries-in-imp-core-sessio.md&quot;, &quot;.mana/archive/2026/03/37.3-unify-usage-persistence-across-imp-execution-paths.md&quot;, &quot;.mana/archive/2026/03/37.4-add-imp-usage-reporting-commands-and-export.md&quot;, &quot;.mana/archive/2026/04/.10-define-clean-mana-vs-imp-boundary-and-memory-conso.md&quot;, &quot;.mana/archive/2026/04/.10.1-define-imp-memory-layer-architecture-and-mana-ownership-boundaries.md&quot;, &quot;.mana/archive/2026/04/.10.2-design-a-mana-wiki-schema-and-knowledge-maintenance-workflow.md&quot;, &quot;.mana/archive/2026/04/.10.3-strengthen-mana-first-prompt-doctrine-for-durable-planning.md&quot;, &quot;.mana/archive/2026/04/.10.4-design-mana-aware-runtime-context-read-path-for-prompt-assembly.md&quot;, &quot;.mana/archive/2026/04/.10.5-design-inline-mana-state-and-knowledge-surfaces-for-imp-runtime.md&quot;, &quot;.mana/archive/2026/04/24.1-turn-activity-tracker-foundation-for-progress-and.md&quot;, &quot;.mana/archive/2026/04/266.4.3.4-fix-stale-secret-metadata-and-missing-keychain-dia.md&quot;, &quot;.mana/archive/2026/04/27.4-imp-promptingtool-guidance-prefer-native-mana-tool.md&quot;, &quot;.mana/archive/2026/04/272-add-kimi-model-compatibility-and-fix-ctrll-model-p.md&quot;, &quot;.mana/archive/2026/04/274-audit-and-simplify-imp-core-config-module.md&quot;, &quot;.mana/archive/2026/04/28-surface-built-in-features-already-implemented-in-i.md&quot;, &quot;.mana/archive/2026/04/28.1.1-specify-the-strengthened-imp-run-worker-contract-a.md&quot;, &quot;.mana/archive/2026/04/28.1.2-implement-reusable-imp-side-mana-unit-worker-runti.md&quot;, &quot;.mana/archive/2026/04/28.1.3-integrate-mana-run-with-the-strengthened-imp-run-w.md&quot;, &quot;.mana/archive/2026/04/28.1.5-fix-native-imp-delegate-worker-defaults-for-openai.md&quot;, &quot;.mana/archive/2026/04/28.1.5-make-imps-native-mana-tool-the-clear-first-class-o.md&quot;, &quot;.mana/archive/2026/04/28.1.5.2-fix-direct-imp-run-codexopenai-worker-request-defa.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.2-extract-shared-model-first-runtime-connection-reso.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.3-refactor-headless-worker-auth-to-normalize-empty-o.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.4-clarify-imp-to-imp-tool-vocabulary-and-align-docs.md&quot;, &quot;.mana/archive/2026/04/29.3-add-recent-session-previews-to-the-imp-startup-pan.md&quot;, &quot;.mana/archive/2026/04/29.4-add-context-aware-quickstart-guidance-and-health-s.md&quot;, &quot;.mana/archive/2026/04/29.6.1-implement-native-mana-scope-targeting-in-imp-tool.md&quot;, &quot;.mana/archive/2026/04/29.6.2-implement-safe-partial-mana-update-semantics-in-im.md&quot;, &quot;.mana/archive/2026/04/29.6.3-implement-append-style-mana-actions-for-conversati.md&quot;, &quot;.mana/archive/2026/04/30-render-compact-widgetstatus-surfaces-already-suppo.md&quot;, &quot;.mana/archive/2026/04/31.1-write-the-engineering-guardrails-design-note-for-i.md&quot;, &quot;.mana/archive/2026/04/32-productize-checkpoints-from-imps-existing-file-sna.md&quot;, &quot;.mana/archive/2026/04/32.1-checkpoint-foundation-shared-filehistory-wiring-an.md&quot;, &quot;.mana/archive/2026/04/32.2-checkpoint-persistence-session-custom-records-plus.md&quot;, &quot;.mana/archive/2026/04/32.3-checkpoint-ux-minimal-slash-command-list-and-resto.md&quot;, &quot;.mana/archive/2026/04/42-per-agent-cached-context-assembly-for-mana-dispatc.md&quot;, &quot;.mana/archive/2026/04/47.1.4-implement-the-first-shared-verifier-and-evidence-r.md&quot;, &quot;.mana/index.yaml.old&quot;, &quot;.mana/migration-conflicts/.3-add-secure-generic-credential-storage-and-lua-secr.md.txt&quot;, &quot;.mana/migration-conflicts/267-fix-native-imp-worker-openai-route-failure-when-sp.md.txt&quot;, &quot;.mana/migration-conflicts/27-native-mana-tool-overhaul-background-runs-lightwei.md.txt&quot;, &quot;.mana/migration-conflicts/270-make-uu-install-support-active-shell-binary-repair.md.txt&quot;, &quot;.mana/migration-conflicts/270.1-make-uu-install-complete-the-active-shell-imp-upgr.md.txt&quot;, &quot;.mana/migration-conflicts/271-harden-spawn-and-mana-tool-termination-so-closespa.md.txt&quot;, &quot;.mana/migration-conflicts/271.1-diagnose-hang-paths-in-imp-spawn-and-mana-closetoo.md.txt&quot;, &quot;.mana/migration-conflicts/273-make-pi-typescript-extensions-importable-into-imp.md.txt&quot;, &quot;.mana/migration-conflicts/275-rethink-imp-tui-tool-call-presentation-and-sidebar.md.txt&quot;, &quot;.mana/migration-conflicts/44-rethink-imp-extensions-as-guest-runtimes-with-opti.md.txt&quot;, &quot;.mana/migration-conflicts/44.1-plan-phased-implementation-of-imp-native-code-inte.md.txt&quot;, &quot;.mana/migration-conflicts/45-explore-ast-backed-symbolic-plan-layer-for-imp.md.txt&quot;, &quot;.mana/migration-conflicts/51-easy-fix-impmana-gaps-triaged-from-repo-scan.md.txt&quot;, &quot;Cargo.lock&quot;, &quot;Cargo.toml&quot;, &quot;README.md&quot;, &quot;crates/imp-cli/auth.json&quot;, &quot;crates/imp-cli/src/lib.rs&quot;, &quot;crates/imp-core/Cargo.toml&quot;, &quot;crates/imp-core/skills/lua-tools/SKILL.md&quot;, &quot;crates/imp-core/skills/writing-skills/REFERENCE.md&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/import.rs&quot;, &quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/sdk.rs&quot;, &quot;crates/imp-core/src/tools/extend.rs&quot;, &quot;crates/imp-core/src/tools/mod.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/bun_runner.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/discovery.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/mod.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/pi_compat.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/schema.rs&quot;], &quot;insertions&quot;: 21, &quot;deletions&quot;: 29398, &quot;risk_score&quot;: 10, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;mostly README&quot;, &quot;touches Cargo.lock&quot;, &quot;touches Cargo.toml&quot;, &quot;very high churn (29419 lines)&quot;]}, {&quot;sha&quot;: &quot;34f8be6671f5091d82792eff6ab9bba4ee34f6df&quot;, &quot;short&quot;: &quot;34f8be6&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T12:27:45-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;.gitleaks.toml&quot;, &quot;Cargo.toml&quot;, &quot;crates/imp-cli/.gitignore&quot;, &quot;crates/imp-cli/Cargo.toml&quot;], &quot;insertions&quot;: 15, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 10, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;risky subject keyword&quot;, &quot;touches Cargo.toml&quot;]}, {&quot;sha&quot;: &quot;2c50e9633a829dec714836848a9faa3da14c7014&quot;, &quot;short&quot;: &quot;2c50e96&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T11:55:43-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;.github/workflows/edge.yml&quot;, &quot;.github/workflows/release.yml&quot;], &quot;insertions&quot;: 2, &quot;deletions&quot;: 2, &quot;risk_score&quot;: 13, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;risky subject keyword&quot;, &quot;touches .github/workflows&quot;]}, {&quot;sha&quot;: &quot;d36a3c1142af4797684158f90dc65d1a44357655&quot;, &quot;short&quot;: &quot;d36a3c1&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T10:11:53-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;crates/imp-cli/src/lib.rs&quot;, &quot;crates/imp-cli/src/usage_report.rs&quot;, &quot;crates/imp-core/examples/sdk_session.rs&quot;, &quot;crates/imp-core/src/agent/events.rs&quot;, &quot;crates/imp-core/src/agent/mod.rs&quot;, &quot;crates/imp-core/src/agent/run_loop.rs&quot;, &quot;crates/imp-core/src/agent/tool_execution.rs&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/error_display.rs&quot;, &quot;crates/imp-core/src/mana_prompt_context.rs&quot;, &quot;crates/imp-core/src/personality.rs&quot;, &quot;crates/imp-core/src/reference_monitor.rs&quot;, &quot;crates/imp-core/src/session.rs&quot;, &quot;crates/imp-core/src/tools/mana.rs&quot;, &quot;crates/imp-core/src/tools/scan/mod.rs&quot;, &quot;crates/imp-core/src/tools/web/read.rs&quot;, &quot;crates/imp-core/src/trust.rs&quot;, &quot;crates/imp-core/src/workflow/verification.rs&quot;, &quot;crates/imp-core/src/workflow/verification_runner.rs&quot;, &quot;crates/imp-lua/src/lib.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;, &quot;crates/imp-tui/src/terminal.rs&quot;, &quot;crates/imp-tui/src/views/ask_bar.rs&quot;, &quot;crates/imp-tui/src/views/chat.rs&quot;, &quot;crates/imp-tui/src/views/editor.rs&quot;, &quot;crates/imp-tui/src/views/sidebar.rs&quot;, &quot;crates/imp-tui/src/views/startup.rs&quot;, &quot;crates/imp-tui/src/views/tool_output.rs&quot;], &quot;insertions&quot;: 209, &quot;deletions&quot;: 211, &quot;risk_score&quot;: 44, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;moderate churn (420 lines)&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-core/src/agent&quot;, &quot;touches crates/imp-core/src/reference_monitor&quot;, &quot;touches crates/imp-core/src/tools/mana&quot;, &quot;touches crates/imp-core/src/workflow&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;371150fdaca0c02e3140222f84c03c6135153840&quot;, &quot;short&quot;: &quot;371150f&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T09:19:21-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;README.md&quot;], &quot;insertions&quot;: 124, &quot;deletions&quot;: 311, &quot;risk_score&quot;: 7, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;moderate churn (435 lines)&quot;, &quot;mostly README&quot;, &quot;risky subject keyword&quot;]}, {&quot;sha&quot;: &quot;b472eadd5b6afbe7a4a06aa7ec603043031f578b&quot;, &quot;short&quot;: &quot;b472ead&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T07:52:46-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;Cargo.lock&quot;, &quot;Cargo.toml&quot;, &quot;README.md&quot;], &quot;insertions&quot;: 21, &quot;deletions&quot;: 21, &quot;risk_score&quot;: 12, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;mostly README&quot;, &quot;risky subject keyword&quot;, &quot;touches Cargo.lock&quot;, &quot;touches Cargo.toml&quot;]}, {&quot;sha&quot;: &quot;42634dbe7b8171671fcef2063b765fe8284f93c0&quot;, &quot;short&quot;: &quot;42634db&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-17T18:30:33-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;.gitignore&quot;, &quot;AGENTS.md&quot;, &quot;CHANGELOG.md&quot;, &quot;Cargo.lock&quot;, &quot;Cargo.toml&quot;, &quot;Cargo.workspace.toml&quot;, &quot;LICENSE&quot;, &quot;README.md&quot;, &quot;crates/imp-cli/src/lib.rs&quot;, &quot;crates/imp-core/Cargo.toml&quot;, &quot;crates/imp-core/examples/tool_surface_live.rs&quot;, &quot;crates/imp-core/skills/writing-skills/REFERENCE.md&quot;, &quot;crates/imp-core/src/agent/events.rs&quot;, &quot;crates/imp-core/src/agent/loop_policy.rs&quot;, &quot;crates/imp-core/src/agent/loop_state.rs&quot;, &quot;crates/imp-core/src/agent/mana_loop.rs&quot;, &quot;crates/imp-core/src/{agent.rs =&gt; agent/mod.rs}&quot;, &quot;crates/imp-core/src/agent/recovery.rs&quot;, &quot;crates/imp-core/src/agent/run_loop.rs&quot;, &quot;crates/imp-core/src/agent/tool_execution.rs&quot;, &quot;crates/imp-core/src/agent/turn_assessment.rs&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/config.rs&quot;, &quot;crates/imp-core/src/context_prefill.rs&quot;, &quot;crates/imp-core/src/contracts.rs&quot;, &quot;crates/imp-core/src/evidence.rs&quot;, &quot;crates/imp-core/src/guardrails.rs&quot;, &quot;crates/imp-core/src/imp_session.rs&quot;, &quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/mana_next/ledger.rs&quot;, &quot;crates/imp-core/src/mana_next/mod.rs&quot;, &quot;crates/imp-core/src/mana_prompt_context.rs&quot;, &quot;crates/imp-core/src/mana_run_state.rs&quot;, &quot;crates/imp-core/src/mana_worker.rs&quot;, &quot;crates/imp-core/src/policy.rs&quot;, &quot;crates/imp-core/src/reference_monitor.rs&quot;, &quot;crates/imp-core/src/resources.rs&quot;, &quot;crates/imp-core/src/retry.rs&quot;, &quot;crates/imp-core/src/roles.rs&quot;, &quot;crates/imp-core/src/run_evidence.rs&quot;, &quot;crates/imp-core/src/session.rs&quot;, &quot;crates/imp-core/src/storage.rs&quot;, &quot;crates/imp-core/src/system_prompt.rs&quot;, &quot;crates/imp-core/src/tools/ask.rs&quot;, &quot;crates/imp-core/src/tools/bash.rs&quot;, &quot;crates/imp-core/src/tools/edit.rs&quot;, &quot;crates/imp-core/src/tools/extend.rs&quot;, &quot;crates/imp-core/src/tools/git.rs&quot;, &quot;crates/imp-core/src/tools/imp.rs&quot;, &quot;crates/imp-core/src/tools/mana.rs&quot;, &quot;crates/imp-core/src/tools/memory.rs&quot;, &quot;crates/imp-core/src/tools/mod.rs&quot;, &quot;crates/imp-core/src/tools/multi_edit.rs&quot;, &quot;crates/imp-core/src/tools/read.rs&quot;, &quot;crates/imp-core/src/tools/scan/kotlin.rs&quot;, &quot;crates/imp-core/src/tools/scan/mod.rs&quot;, &quot;crates/imp-core/src/tools/session_search.rs&quot;, &quot;crates/imp-core/src/tools/shell.rs&quot;, &quot;crates/imp-core/src/tools/web/github.rs&quot;, &quot;crates/imp-core/src/tools/web/mod.rs&quot;, &quot;crates/imp-core/src/tools/web/read.rs&quot;, &quot;crates/imp-core/src/tools/web/search.rs&quot;, &quot;crates/imp-core/src/tools/web/types.rs&quot;, &quot;crates/imp-core/src/tools/web/youtube.rs&quot;, &quot;crates/imp-core/src/tools/worktree.rs&quot;, &quot;crates/imp-core/src/tools/write.rs&quot;, &quot;crates/imp-core/src/trace.rs&quot;, &quot;crates/imp-core/src/trust.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/mod.rs&quot;, &quot;crates/imp-core/src/ui.rs&quot;, &quot;crates/imp-core/src/workflow/contract.rs&quot;, &quot;crates/imp-core/src/workflow/mod.rs&quot;, &quot;crates/imp-core/src/workflow/verification.rs&quot;, &quot;crates/imp-core/src/workflow/verification_runner.rs&quot;, &quot;crates/imp-gui/Cargo.toml&quot;, &quot;crates/imp-gui/README.md&quot;, &quot;crates/imp-gui/src/lib.rs&quot;, &quot;crates/imp-gui/src/main.rs&quot;, &quot;crates/imp-llm/Cargo.toml&quot;, &quot;crates/imp-llm/src/lib.rs&quot;, &quot;crates/imp-llm/src/provider.rs&quot;, &quot;crates/imp-llm/src/providers/anthropic.rs&quot;, &quot;crates/imp-llm/src/providers/openai.rs&quot;, &quot;crates/imp-lua/src/bridge.rs&quot;, &quot;crates/imp-lua/src/lib.rs&quot;, &quot;crates/imp-lua/src/loader.rs&quot;, &quot;crates/imp-lua/src/sandbox.rs&quot;, &quot;crates/imp-tui/Cargo.toml&quot;, &quot;crates/imp-tui/src/animation.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;, &quot;crates/imp-tui/src/event_source.rs&quot;, &quot;crates/imp-tui/src/keybindings.rs&quot;, &quot;crates/imp-tui/src/lib.rs&quot;, &quot;crates/imp-tui/src/terminal.rs&quot;, &quot;crates/imp-tui/src/tui_interface.rs&quot;, &quot;crates/imp-tui/src/turn_tracker.rs&quot;, &quot;crates/imp-tui/src/views/ask_bar.rs&quot;, &quot;crates/imp-tui/src/views/chat.rs&quot;, &quot;crates/imp-tui/src/views/command_palette.rs&quot;, &quot;crates/imp-tui/src/views/editor.rs&quot;, &quot;crates/imp-tui/src/views/file_finder.rs&quot;, &quot;crates/imp-tui/src/views/mana_navigator.rs&quot;, &quot;crates/imp-tui/src/views/mod.rs&quot;, &quot;crates/imp-tui/src/views/session_picker.rs&quot;, &quot;crates/imp-tui/src/views/settings.rs&quot;, &quot;crates/imp-tui/src/views/sidebar.rs&quot;, &quot;crates/imp-tui/src/views/startup.rs&quot;, &quot;crates/imp-tui/src/views/tool_output.rs&quot;, &quot;crates/imp-tui/src/views/tools.rs&quot;, &quot;docs/autonomy-modes.md&quot;, &quot;docs/design/lua-programmatic-interactions.md&quot;, &quot;docs/imp-next-workflow-runtime.md&quot;, &quot;docs/mana-next-compatibility-adapter.md&quot;, &quot;docs/mana-next-examples.md&quot;, &quot;docs/mana-next-migration-test-plan.md&quot;, &quot;docs/mana-next-runtime-event-mapping.md&quot;, &quot;docs/mana-next-storage-strategy.md&quot;, &quot;docs/mana-next-ux.md&quot;, &quot;docs/mana-next-workflow-ledger.md&quot;, &quot;docs/reference-monitor-policy.md&quot;, &quot;docs/run-evidence.md&quot;, &quot;docs/trace-and-evidence-format.md&quot;, &quot;docs/trust-labels-and-provenance.md&quot;, &quot;docs/tui-workflow-wireframes.md&quot;, &quot;docs/verification-gates.md&quot;, &quot;docs/worktree-auto.md&quot;, &quot;imp-gui-wireframe.html&quot;], &quot;insertions&quot;: 39025, &quot;deletions&quot;: 5869, &quot;risk_score&quot;: 56, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;mostly CHANGELOG&quot;, &quot;mostly README&quot;, &quot;mostly docs/&quot;, &quot;risky subject keyword&quot;, &quot;touches Cargo.lock&quot;, &quot;touches Cargo.toml&quot;, &quot;touches crates/imp-core/src/agent&quot;, &quot;touches crates/imp-core/src/mana_worker&quot;, &quot;touches crates/imp-core/src/reference_monitor&quot;, &quot;touches crates/imp-core/src/tools/mana&quot;, &quot;touches crates/imp-core/src/workflow&quot;, &quot;touches crates/imp-llm/src/providers&quot;, &quot;touches crates/imp-tui/src/app&quot;, &quot;touches crates/imp-tui/src/event_source&quot;, &quot;very high churn (44894 lines)&quot;]}, {&quot;sha&quot;: &quot;eb3f46fb52a4b11228cf0df7d889a2d40e845980&quot;, &quot;short&quot;: &quot;eb3f46f&quot;, &quot;subject&quot;: &quot;Use published mana crates for release build&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-01T15:40:38-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;Cargo.lock&quot;, &quot;Cargo.toml&quot;, &quot;crates/imp-core/Cargo.toml&quot;], &quot;insertions&quot;: 8, &quot;deletions&quot;: 2, &quot;risk_score&quot;: 6, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;touches Cargo.lock&quot;, &quot;touches Cargo.toml&quot;]}, {&quot;sha&quot;: &quot;9e6cd9c85b0da3cc2b93bed18a476e265ad719bb&quot;, &quot;short&quot;: &quot;9e6cd9c&quot;, &quot;subject&quot;: &quot;Clean release branch artifacts&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-01T14:12:24-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;.gitignore&quot;, &quot;.mana/.3-set-up-harbor-adapter-and-terminal-bench-20-runner.md&quot;, &quot;.mana/.5-add-safe-automatic-context-compaction-for-long-run.md&quot;, &quot;.mana/.5.1-add-disabled-by-default-auto-compaction-config-sca.md&quot;, &quot;.mana/.6-hardening-pass-reduce-bugs-and-contract-mismatches.md&quot;, &quot;.mana/.6.6-enforce-lua-extension-capability-boundaries.md&quot;, &quot;.mana/.6.7-propagate-cancellation-into-active-tool-execution.md&quot;, &quot;.mana/.6.8-align-diff-tool-registration-with-mode-contracts.md&quot;, &quot;.mana/.9-upgrade-imp-mana-authoring-prompt-contract-for-orc.md&quot;, &quot;.mana/.gitignore&quot;, &quot;.mana/21-imp-efficiency-smarter-tool-output-truncation.md&quot;, &quot;.mana/245.1-define-manaimp-contract-implications-of-file-nativ.md&quot;, &quot;.mana/245.1.1-define-vnext-manaimp-subagent-handoff-packet-for-o.md&quot;, &quot;.mana/248-comprehensive-imp-uiux-review-upgrade-and-polish-a.md&quot;, &quot;.mana/248.14-implement-restrained-ansi-emphasis-for-shell-typog.md&quot;, &quot;.mana/248.16.5-create-svg-wireframes-for-candidate-imp-tui-layout.md&quot;, &quot;.mana/248.16.7-revise-imp-tui-wireframes-and-core-memo-after-user.md&quot;, &quot;.mana/248.17-design-terminal-emulator-native-coding-agent-cockp.md&quot;, &quot;.mana/248.18-harden-and-humanize-imp-error-streaming-across-pro.md&quot;, &quot;.mana/248.18.1-extract-shared-imp-core-streamed-error-normalizati.md&quot;, &quot;.mana/248.18.2-harden-imp-core-partial-stream-and-silent-eof-diag.md&quot;, &quot;.mana/248.18.3-design-stable-machine-facing-streamed-error-envelo.md&quot;, &quot;.mana/248.7-plan-shared-uxruntime-seams-for-shell-tui-and-view.md&quot;, &quot;.mana/248.9-capture-and-sequence-real-user-feedback-on-the-new.md&quot;, &quot;.mana/249-reduce-duplicate-verbose-mana-change-narration-in.md&quot;, &quot;.mana/250-shape-getimpdev-landing-page-direction-and-impleme.md&quot;, &quot;.mana/254-fresh-smoke-trial-for-native-imp-run-on-an-isolate.md&quot;, &quot;.mana/256-run-one-shot-native-imp-print-smoke-before-imp-run.md&quot;, &quot;.mana/257-draft-imp-ontologymd-for-shared-featureruntime-lan.md&quot;, &quot;.mana/259-audit-panic-usage-and-detached-task-failure-policy.md&quot;, &quot;.mana/263-audit-and-isolate-library-level-stderr-writes-that.md&quot;, &quot;.mana/263.2-classify-mana-core-stderr-writes-for-embedded-risk.md&quot;, &quot;.mana/264-normalize-imp-storage-topology-for-sessions-config.md&quot;, &quot;.mana/264.1-audit-current-imp-durable-storage-surfaces-and-pat.md&quot;, &quot;.mana/264.2-specify-normalized-imp-storage-contract-and-migrat.md&quot;, &quot;.mana/264.3.1-add-shared-imp-core-storage-path-module-for-canoni.md&quot;, &quot;.mana/264.3.2-migrate-config-auth-session-and-session-search-cal.md&quot;, &quot;.mana/264.3.3-migrate-instruction-discovery-to-canonical-impagen.md&quot;, &quot;.mana/264.3.4-implement-non-destructive-migration-from-legacy-im.md&quot;, &quot;.mana/264.4-audit-and-fix-imp-session-index-lifecycle-wiring-f.md&quot;, &quot;.mana/264.6-decide-canonical-imp-filesystem-roots-for-global-a.md&quot;, &quot;.mana/264.7-specify-precedence-and-merge-rules-between-imp-and.md&quot;, &quot;.mana/264.8-specify-migration-from-xdgmacos-legacy-paths-into.md&quot;, &quot;.mana/266-cross-codebase-review-compare-imp-and-hermes-agent.md&quot;, &quot;.mana/266.1-design-adoption-path-provider-resilience-and-auth.md&quot;, &quot;.mana/266.2-design-adoption-path-session-recall-memory-and-con.md&quot;, &quot;.mana/266.3-design-adoption-path-extension-seams-and-product-s.md&quot;, &quot;.mana/266.4-imp-vnext-ranked-roadmap-and-phased-execution-plan.md&quot;, &quot;.mana/266.4.7-phase-5-epic-selective-later-product-surface-expan.md&quot;, &quot;.mana/267-adopt-highest-value-product-lessons-from-opencode.md&quot;, &quot;.mana/268.1-diagnose-native-imp-mana-tool-divergence-from-cli.md&quot;, &quot;.mana/27-improve-mana-pool-competitive-grade-dispatch-engin.md&quot;, &quot;.mana/27.14-define-attempt-scoped-autonomy-observation-record.md&quot;, &quot;.mana/27.2-imp-ui-compact-mana-statusprogress-surface.md&quot;, &quot;.mana/271-add-native-youtube-video-interpretation-support-to.md&quot;, &quot;.mana/271.1-implement-pure-http-youtube-transcript-extraction.md&quot;, &quot;.mana/271.2-harden-imp-spawn-and-mana-closetool-execution-agai.md&quot;, &quot;.mana/272-add-native-video-context-ingestion-architecture-fo.md&quot;, &quot;.mana/272.1-implement-pure-http-youtube-transcript-extraction.md&quot;, &quot;.mana/272.2-design-richer-video-interpretation-beyond-transcri.md&quot;, &quot;.mana/273-diagnose-and-harden-kimi-code-oauth-model-routing.md&quot;, &quot;.mana/273.5-sprint-import-and-execute-pi-typescript-extensions.md&quot;, &quot;.mana/273.5.10-prove-bun-ts-adapter-against-local-pi-color-palett.md&quot;, &quot;.mana/273.5.11-add-official-pi-dynamic-tools-compatibility-fixtur.md&quot;, &quot;.mana/273.5.12-define-sprint-1-typescriptpi-extension-support-bou.md&quot;, &quot;.mana/273.5.13-probe-dependency-bearing-pi-extension-compatibilit.md&quot;, &quot;.mana/273.5.4-normalize-typeboxjson-schemas-from-typescript-exte.md&quot;, &quot;.mana/275-assess-and-sequence-next-llm-oauth-provider-integr.md&quot;, &quot;.mana/275.10-inventory-pi-and-imp-provideroauth-surfaces.md&quot;, &quot;.mana/275.11-sequence-pi-provideroauth-parity-implementation.md&quot;, &quot;.mana/275.6-assess-pi-google-antigravity-provider-route-for-im.md&quot;, &quot;.mana/275.9-research-unofficial-cursor-provider-support-for-im.md&quot;, &quot;.mana/276-investigate-and-harden-tui-esc-cancellation-for-hu.md&quot;, &quot;.mana/277-fix-imp-tui-clean-ui-corruption-and-string-join-ov.md&quot;, &quot;.mana/278-fix-inspector-mode-interaction-model.md&quot;, &quot;.mana/28.1-make-imp-run-the-canonical-mana-worker-runtime-whi.md&quot;, &quot;.mana/28.5.1-patch-imp-system-prompt-with-mana-first-planning-d.md&quot;, &quot;.mana/28.5.6-implement-turn-scoped-mana-review-packet-aggregati.md&quot;, &quot;.mana/28.5.7-render-between-turn-mana-review-packets-across-imp.md&quot;, &quot;.mana/28.5.7.1-add-shared-imp-core-turnmanadelta-renderer-and-man.md&quot;, &quot;.mana/28.5.7.2-render-compact-between-turn-mana-block-and-textual.md&quot;, &quot;.mana/28.5.7.3-render-between-turn-mana-review-packets-in-imp-cli.md&quot;, &quot;.mana/28.5.7.4-add-shared-manareviewmode-config-and-presentation.md&quot;, &quot;.mana/28.5.7.5-wire-imp-tui-compact-widget-tray-block-and-sidebar.md&quot;, &quot;.mana/280-review-project-gaps-that-would-make-imp-stronger-t.md&quot;, &quot;.mana/280.1-run-dirac-evals-with-imp-using-gemini-secret.md&quot;, &quot;.mana/280.2-adopt-dirac-inspired-code-intelligence-and-precise.md&quot;, &quot;.mana/280.2.1.1-decide-migration-safe-naming-strategy-for-imp-scan.md&quot;, &quot;.mana/280.2.2-implement-read-oriented-symbol-extraction-and-skel.md&quot;, &quot;.mana/280.2.3-add-anchor-backed-read-and-stale-safe-edit-flow-to.md&quot;, &quot;.mana/280.2.4-implement-edit-transaction-batching-with-combined.md&quot;, &quot;.mana/282-design-native-scoped-secret-injection-for-command.md&quot;, &quot;.mana/285-verify-installed-imp-binary-includes-latest-secret.md&quot;, &quot;.mana/290-complete-imp-codebase-quality-audit.md&quot;, &quot;.mana/290.1-split-imp-tui-apprs-by-responsibility.md&quot;, &quot;.mana/290.2-split-imp-core-agentrs-into-focused-runtime-module.md&quot;, &quot;.mana/290.3-split-imp-cli-librs-into-command-modules.md&quot;, &quot;.mana/290.4-split-native-mana-tool-implementation-into-focused.md&quot;, &quot;.mana/291-rewrite-imp-readme-around-product-features-mana-an.md&quot;, &quot;.mana/31.2-add-guardrail-config-types-and-profile-selection-t.md&quot;, &quot;.mana/31.3-integrate-guardrails-into-the-imp-system-prompt-an.md&quot;, &quot;.mana/31.4-add-the-initial-zig-guardrail-profile-and-document.md&quot;, &quot;.mana/33-chat-view-replace-duplicated-animation-logic-with.md&quot;, &quot;.mana/34-sidebar-detail-header-use-spinnerframe-and-respect.md&quot;, &quot;.mana/35-editor-remove-dead-tick-and-animationlevel-params.md&quot;, &quot;.mana/36-animation-config-reconcile-minimal-namingdocs-afte.md&quot;, &quot;.mana/37-add-first-class-usage-accounting-and-reporting-to.md&quot;, &quot;.mana/37.5-test-and-document-imp-usage-accountingreporting.md&quot;, &quot;.mana/41-anthropic-api-parity-adopt-claude-code-patterns-fo.md&quot;, &quot;.mana/44-define-memory-and-code-intelligence-architecture-f.md&quot;, &quot;.mana/44.1-author-guest-runtime-extension-substrate-proposal.md&quot;, &quot;.mana/44.1.10-implement-documentworkspace-symbols-with-ast-first.md&quot;, &quot;.mana/44.1.11-implement-hover-and-signature-help-on-the-phase-1.md&quot;, &quot;.mana/44.1.12-unify-code-intelligence-diagnostic-summaries-with.md&quot;, &quot;.mana/44.1.14-evaluate-whether-repeated-evidence-promotion-flows.md&quot;, &quot;.mana/44.1.5-plan-guarded-write-oriented-semantic-actions-and-p.md&quot;, &quot;.mana/44.1.5.5-specify-semantic-write-execution-contract-for-prev.md&quot;, &quot;.mana/44.1.6-sequence-phase-1-read-oriented-imp-code-intelligen.md&quot;, &quot;.mana/44.1.6.1-define-shared-normalization-envelopes-for-read-ori.md&quot;, &quot;.mana/44.1.6.2-plan-diagnostics-plus-ast-alignment-for-the-first.md&quot;, &quot;.mana/44.1.6.3-plan-document-symbols-and-go-to-definition-over-th.md&quot;, &quot;.mana/44.1.6.4-plan-references-and-workspace-symbol-browsing-for.md&quot;, &quot;.mana/44.1.6.5-plan-hover-and-signature-enrichment-after-core-rea.md&quot;, &quot;.mana/44.1.7-roll-out-phase-1-read-oriented-imp-code-intelligen.md&quot;, &quot;.mana/44.1.8-normalize-read-oriented-code-intelligence-queryres.md&quot;, &quot;.mana/44.1.9-implement-phase-1-diagnostics-go-to-definition-and.md&quot;, &quot;.mana/44.3-translate-guest-runtime-design-into-phased-impleme.md&quot;, &quot;.mana/45-tower-rebuild-around-explicit-contracts-durable-le.md&quot;, &quot;.mana/45.10.5-update-docs-for-mana-platform-substrate-and-imp-pr.md&quot;, &quot;.mana/45.11-capture-near-term-imp-execution-lanes-under-the-im.md&quot;, &quot;.mana/45.11.1-resolve-consequential-defaults-for-near-term-imp-i.md&quot;, &quot;.mana/45.11.1.1-clarify-whether-native-rust-not-lua-applies-to-imp.md&quot;, &quot;.mana/45.11.1.2-sequence-near-term-imp-implementation-order-from-s.md&quot;, &quot;.mana/45.4-phase-3-introduce-runner-protocol-and-local-adapte.md&quot;, &quot;.mana/45.4.2-plan-the-first-imp-local-runner-adapter-that-consu.md&quot;, &quot;.mana/45.4.4-plan-the-cutover-from-current-imp-run-plus-mana-ru.md&quot;, &quot;.mana/45.5-phase-4-rebuild-imp-around-stable-workerruntime-se.md&quot;, &quot;.mana/45.5.1-map-imp-core-hotspots-into-target-runtime-context.md&quot;, &quot;.mana/45.5.3-write-a-compact-imp-decomposition-order-for-post-c.md&quot;, &quot;.mana/45.7-phase-6-harden-policy-isolation-and-migration-surf.md&quot;, &quot;.mana/45.7.4-evaluate-whether-imp-should-add-a-native-gitrepo-t.md&quot;, &quot;.mana/46-broaden-imp-attention-beyond-toolsprompting-under.md&quot;, &quot;.mana/46.1-reconcile-long-session-runtime-safety-gaps-and-tur.md&quot;, &quot;.mana/46.2-reconcile-user-visible-discoverability-and-ux-brea.md&quot;, &quot;.mana/46.2.1-surface-usage-reporting-in-the-tui-commandhelpstar.md&quot;, &quot;.mana/47-rebuild-imp-around-explicit-runtime-boundaries-pro.md&quot;, &quot;.mana/47.1-contracts-and-ownership-boundary-for-mana-imp-rebu.md&quot;, &quot;.mana/47.6-sequence-the-imp-rebuild-as-an-incremental-migrati.md&quot;, &quot;.mana/50-define-cli-first-operator-surface-for-imp-with-tui.md&quot;, &quot;.mana/50.10-implement-guided-cli-parity-flows-for-settings-per.md&quot;, &quot;.mana/50.10.1-implement-terminal-native-imp-settings-flow-for-cl.md&quot;, &quot;.mana/50.10.1.2-let-imp-chat-no-tools-reach-the-shell-without-prov.md&quot;, &quot;.mana/50.10.2-implement-terminal-native-imp-personality-flow-for.md&quot;, &quot;.mana/50.11-implement-first-shell-to-view-handoff-for-sessions.md&quot;, &quot;.mana/50.11.2-align-imp-chat-view-handoff-with-explicit-imp-view.md&quot;, &quot;.mana/50.12-flip-plain-imp-to-imp-chat-after-shell-readiness-g.md&quot;, &quot;.mana/50.13-plan-extraction-of-shared-fullscreen-consumed-runt.md&quot;, &quot;.mana/50.14-specify-the-shared-imp-ui-request-and-runtime-even.md&quot;, &quot;.mana/50.16-follow-on-cli-native-affordance-stack-after-505-se.md&quot;, &quot;.mana/50.16.1-define-stable-human-vs-machine-output-modes-across.md&quot;, &quot;.mana/50.16.2-plan-cli-first-checkpoint-productization-after-out.md&quot;, &quot;.mana/50.16.3-plan-visible-cli-first-planning-artifacts-and-exec.md&quot;, &quot;.mana/50.16.4-plan-first-class-approval-policy-layer-for-cli-fir.md&quot;, &quot;.mana/50.16.5-surface-session-browsing-and-session-search-as-fir.md&quot;, &quot;.mana/50.16.5.1-audit-and-reconcile-imp-session-storage-and-sessio.md&quot;, &quot;.mana/50.16.6-plan-detachedbackground-local-execution-after-cli.md&quot;, &quot;.mana/50.17-define-stable-human-vs-machine-output-contracts-fo.md&quot;, &quot;.mana/50.18-define-cli-first-session-browsing-and-sessionsearc.md&quot;, &quot;.mana/50.19-define-stable-imp-human-vs-machine-output-contract.md&quot;, &quot;.mana/50.20-plan-first-cli-first-checkpoint-productization-ove.md&quot;, &quot;.mana/50.21-specify-visible-planning-artifacts-and-checklist-b.md&quot;, &quot;.mana/50.22-specify-the-first-visible-planning-workflow-and-pl.md&quot;, &quot;.mana/50.23-specify-cli-first-approval-policy-and-blocked-stat.md&quot;, &quot;.mana/50.24-define-the-first-cli-first-approval-policy-surface.md&quot;, &quot;.mana/50.25-specify-detachedbackground-local-execution-contrac.md&quot;, &quot;.mana/50.26-define-the-first-local-detachedbackground-executio.md&quot;, &quot;.mana/50.6-design-the-cli-first-interactive-shell-path-for-im.md&quot;, &quot;.mana/50.9-implement-the-first-cli-first-proving-slice-with-e.md&quot;, &quot;.mana/51.6.1-audit-current-mana-core-embedding-surface-against.md&quot;, &quot;.mana/65-root-mana-currently-lists-child-513-but-direct-sho.md&quot;, &quot;.mana/69-imp-cli-no-longer-contains-duplicate-mana-unit-loa.md&quot;, &quot;.mana/73-code-intelligence-outputs-are-transient-by-default.md&quot;, &quot;.mana/81-design-imp-native-delegation-tool-around-imp-run-a.md&quot;, &quot;.mana/81.10-define-codemap-backed-context-seam-for-imp-run-and.md&quot;, &quot;.mana/82-assess-gpt-54-pro-support-through-openai-chatgpt-o.md&quot;, &quot;.mana/82.2-add-gpt-54-pro-to-imp-model-registry-only-after-oa.md&quot;, &quot;.mana/83-harden-imp-tui-text-box-cursor-and-bounds-handling.md&quot;, &quot;.mana/RULES.md&quot;, &quot;.mana/archive/2026/03/.2-design-canonical-usage-schema-and-aggregation-help.md&quot;, &quot;.mana/archive/2026/03/16-imp-core-hardening-production-ready-agent-engine.md&quot;, &quot;.mana/archive/2026/03/16.1-wire-config-agent-agentbuilder-thresholds-hooks-re.md&quot;, &quot;.mana/archive/2026/03/16.2-tool-argument-validation-json-schema-before-execut.md&quot;, &quot;.mana/archive/2026/03/16.3-llm-retry-with-exponential-backoff-and-jitter.md&quot;, &quot;.mana/archive/2026/03/16.4-loop-detection-prevent-infinite-tool-call-retry-lo.md&quot;, &quot;.mana/archive/2026/03/16.5-file-not-found-suggestions-with-levenshtein-rankin.md&quot;, &quot;.mana/archive/2026/03/16.6-auto-resume-after-compaction-re-queue-original-pro.md&quot;, &quot;.mana/archive/2026/03/16.7-file-read-tracking-and-staleness-detection.md&quot;, &quot;.mana/archive/2026/03/16.8-file-version-history-pre-edit-snapshots-for-rollba.md&quot;, &quot;.mana/archive/2026/03/17-imp-efficiency-enable-prompt-caching.md&quot;, &quot;.mana/archive/2026/03/19-imp-efficiency-in-session-file-content-cache.md&quot;, &quot;.mana/archive/2026/03/20-imp-efficiency-parallelize-grep-block-search-with.md&quot;, &quot;.mana/archive/2026/03/229-imp-rust-coding-agent-engine.md&quot;, &quot;.mana/archive/2026/03/229.1-workspace-setup-imp-llm-types.md&quot;, &quot;.mana/archive/2026/03/229.10-imp-llm-anthropic-oauth.md&quot;, &quot;.mana/archive/2026/03/229.11-imp-core-hook-system.md&quot;, &quot;.mana/archive/2026/03/229.12-imp-core-tree-sitter-tools-probesearch-probeextrac.md&quot;, &quot;.mana/archive/2026/03/229.13-imp-core-config-resource-discovery.md&quot;, &quot;.mana/archive/2026/03/229.14-imp-core-system-prompt-assembly.md&quot;, &quot;.mana/archive/2026/03/229.15-imp-lua-lua-extension-runtime.md&quot;, &quot;.mana/archive/2026/03/229.16-imp-core-shell-tool-loader.md&quot;, &quot;.mana/archive/2026/03/229.17-imp-tui-ratatui-interactive-mode.md&quot;, &quot;.mana/archive/2026/03/229.18-imp-cli-binary-entry-point.md&quot;, &quot;.mana/archive/2026/03/229.2-imp-llm-anthropic-provider.md&quot;, &quot;.mana/archive/2026/03/229.3-imp-core-tool-trait-file-tools-read-write-edit-mul.md&quot;, &quot;.mana/archive/2026/03/229.4-imp-core-bash-grep-find-tools.md&quot;, &quot;.mana/archive/2026/03/229.5-imp-core-ask-diff-tools.md&quot;, &quot;.mana/archive/2026/03/229.6-imp-core-agent-loop.md&quot;, &quot;.mana/archive/2026/03/229.7-imp-core-session-manager.md&quot;, &quot;.mana/archive/2026/03/229.8-imp-core-context-management-observation-masking-co.md&quot;, &quot;.mana/archive/2026/03/229.9-imp-llm-openai-google-providers.md&quot;, &quot;.mana/archive/2026/03/23-learning-loop-agent-curated-memory-skill-managemen.md&quot;, &quot;.mana/archive/2026/03/23.1-system-prompt-layer-6-wire-memory-into-prompt-asse.md&quot;, &quot;.mana/archive/2026/03/23.2-memory-store-and-memory-tool.md&quot;, &quot;.mana/archive/2026/03/23.3-skill-manage-tool-agent-creates-patches-and-delete.md&quot;, &quot;.mana/archive/2026/03/23.4-learning-nudges-system-prompt-text-and-onagentend.md&quot;, &quot;.mana/archive/2026/03/23.5-session-index-with-fts5-full-text-search.md&quot;, &quot;.mana/archive/2026/03/23.6-session-search-tool.md&quot;, &quot;.mana/archive/2026/03/24-tui-ux-overhaul-information-density-summaries-inte.md&quot;, &quot;.mana/archive/2026/03/24.1-turn-activity-tracker-foundation-for-progress-and.md&quot;, &quot;.mana/archive/2026/03/24.2-progress-indicator-in-status-bar-during-streaming.md&quot;, &quot;.mana/archive/2026/03/24.3-per-tool-call-expandcollapse-and-auto-expand-error.md&quot;, &quot;.mana/archive/2026/03/24.4-turn-end-summary-with-file-change-tracking.md&quot;, &quot;.mana/archive/2026/03/24.5-visual-separation-of-tool-activity-from-assistant.md&quot;, &quot;.mana/archive/2026/03/24.6-editor-polish-placeholder-model-indicator-keybindi.md&quot;, &quot;.mana/archive/2026/03/24.7-fix-context-window-tracking-use-actual-conversatio.md&quot;, &quot;.mana/archive/2026/03/24.8-approval-flow-wire-userinterface-for-tool-confirma.md&quot;, &quot;.mana/archive/2026/03/25-multi-provider-llm-support-with-data-driven-welcom.md&quot;, &quot;.mana/archive/2026/03/25.1-provider-metadata-registry-auth-generalization.md&quot;, &quot;.mana/archive/2026/03/25.2-openai-compatible-chat-completions-provider.md&quot;, &quot;.mana/archive/2026/03/25.3-add-builtin-models-for-new-providers.md&quot;, &quot;.mana/archive/2026/03/25.4-data-driven-welcome-flow.md&quot;, &quot;.mana/archive/2026/03/25.5-generalize-cli-login-for-all-providers.md&quot;, &quot;.mana/archive/2026/03/26-fix-imp-tui-compile-errors-around-toolcallorder-re.md&quot;, &quot;.mana/archive/2026/03/27.1-imp-core-mana-tool-add-native-orchestration-action.md&quot;, &quot;.mana/archive/2026/03/31-add-configurable-engineering-guardrails-to-imp.md&quot;, &quot;.mana/archive/2026/03/37.1-design-canonical-usage-schema-and-aggregation-help.md&quot;, &quot;.mana/archive/2026/03/37.2-persist-canonical-usage-entries-in-imp-core-sessio.md&quot;, &quot;.mana/archive/2026/03/37.3-unify-usage-persistence-across-imp-execution-paths.md&quot;, &quot;.mana/archive/2026/03/37.4-add-imp-usage-reporting-commands-and-export.md&quot;, &quot;.mana/archive/2026/04/.10-define-clean-mana-vs-imp-boundary-and-memory-conso.md&quot;, &quot;.mana/archive/2026/04/.10.1-define-imp-memory-layer-architecture-and-mana-ownership-boundaries.md&quot;, &quot;.mana/archive/2026/04/.10.2-design-a-mana-wiki-schema-and-knowledge-maintenance-workflow.md&quot;, &quot;.mana/archive/2026/04/.10.3-strengthen-mana-first-prompt-doctrine-for-durable-planning.md&quot;, &quot;.mana/archive/2026/04/.10.4-design-mana-aware-runtime-context-read-path-for-prompt-assembly.md&quot;, &quot;.mana/archive/2026/04/.10.5-design-inline-mana-state-and-knowledge-surfaces-for-imp-runtime.md&quot;, &quot;.mana/archive/2026/04/24.1-turn-activity-tracker-foundation-for-progress-and.md&quot;, &quot;.mana/archive/2026/04/266.4.3.4-fix-stale-secret-metadata-and-missing-keychain-dia.md&quot;, &quot;.mana/archive/2026/04/27.4-imp-promptingtool-guidance-prefer-native-mana-tool.md&quot;, &quot;.mana/archive/2026/04/272-add-kimi-model-compatibility-and-fix-ctrll-model-p.md&quot;, &quot;.mana/archive/2026/04/274-audit-and-simplify-imp-core-config-module.md&quot;, &quot;.mana/archive/2026/04/28-surface-built-in-features-already-implemented-in-i.md&quot;, &quot;.mana/archive/2026/04/28.1.1-specify-the-strengthened-imp-run-worker-contract-a.md&quot;, &quot;.mana/archive/2026/04/28.1.2-implement-reusable-imp-side-mana-unit-worker-runti.md&quot;, &quot;.mana/archive/2026/04/28.1.3-integrate-mana-run-with-the-strengthened-imp-run-w.md&quot;, &quot;.mana/archive/2026/04/28.1.5-fix-native-imp-delegate-worker-defaults-for-openai.md&quot;, &quot;.mana/archive/2026/04/28.1.5-make-imps-native-mana-tool-the-clear-first-class-o.md&quot;, &quot;.mana/archive/2026/04/28.1.5.2-fix-direct-imp-run-codexopenai-worker-request-defa.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.2-extract-shared-model-first-runtime-connection-reso.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.3-refactor-headless-worker-auth-to-normalize-empty-o.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.4-clarify-imp-to-imp-tool-vocabulary-and-align-docs.md&quot;, &quot;.mana/archive/2026/04/29.3-add-recent-session-previews-to-the-imp-startup-pan.md&quot;, &quot;.mana/archive/2026/04/29.4-add-context-aware-quickstart-guidance-and-health-s.md&quot;, &quot;.mana/archive/2026/04/29.6.1-implement-native-mana-scope-targeting-in-imp-tool.md&quot;, &quot;.mana/archive/2026/04/29.6.2-implement-safe-partial-mana-update-semantics-in-im.md&quot;, &quot;.mana/archive/2026/04/29.6.3-implement-append-style-mana-actions-for-conversati.md&quot;, &quot;.mana/archive/2026/04/30-render-compact-widgetstatus-surfaces-already-suppo.md&quot;, &quot;.mana/archive/2026/04/31.1-write-the-engineering-guardrails-design-note-for-i.md&quot;, &quot;.mana/archive/2026/04/32-productize-checkpoints-from-imps-existing-file-sna.md&quot;, &quot;.mana/archive/2026/04/32.1-checkpoint-foundation-shared-filehistory-wiring-an.md&quot;, &quot;.mana/archive/2026/04/32.2-checkpoint-persistence-session-custom-records-plus.md&quot;, &quot;.mana/archive/2026/04/32.3-checkpoint-ux-minimal-slash-command-list-and-resto.md&quot;, &quot;.mana/archive/2026/04/42-per-agent-cached-context-assembly-for-mana-dispatc.md&quot;, &quot;.mana/archive/2026/04/47.1.4-implement-the-first-shared-verifier-and-evidence-r.md&quot;, &quot;.mana/index.yaml.old&quot;, &quot;.mana/migration-conflicts/.3-add-secure-generic-credential-storage-and-lua-secr.md.txt&quot;, &quot;.mana/migration-conflicts/267-fix-native-imp-worker-openai-route-failure-when-sp.md.txt&quot;, &quot;.mana/migration-conflicts/27-native-mana-tool-overhaul-background-runs-lightwei.md.txt&quot;, &quot;.mana/migration-conflicts/270-make-uu-install-support-active-shell-binary-repair.md.txt&quot;, &quot;.mana/migration-conflicts/270.1-make-uu-install-complete-the-active-shell-imp-upgr.md.txt&quot;, &quot;.mana/migration-conflicts/271-harden-spawn-and-mana-tool-termination-so-closespa.md.txt&quot;, &quot;.mana/migration-conflicts/271.1-diagnose-hang-paths-in-imp-spawn-and-mana-closetoo.md.txt&quot;, &quot;.mana/migration-conflicts/273-make-pi-typescript-extensions-importable-into-imp.md.txt&quot;, &quot;.mana/migration-conflicts/275-rethink-imp-tui-tool-call-presentation-and-sidebar.md.txt&quot;, &quot;.mana/migration-conflicts/44-rethink-imp-extensions-as-guest-runtimes-with-opti.md.txt&quot;, &quot;.mana/migration-conflicts/44.1-plan-phased-implementation-of-imp-native-code-inte.md.txt&quot;, &quot;.mana/migration-conflicts/45-explore-ast-backed-symbolic-plan-layer-for-imp.md.txt&quot;, &quot;.mana/migration-conflicts/51-easy-fix-impmana-gaps-triaged-from-repo-scan.md.txt&quot;, &quot;.tmp/imp-run-trial/one-shot-print.txt&quot;, &quot;.vibecheck/vibecheck.db&quot;, &quot;.vibecheck/vibecheck.db-shm&quot;, &quot;.vibecheck/vibecheck.db-wal&quot;, &quot;=&quot;, &quot;AGENTS copy.md&quot;, &quot;art.html&quot;, &quot;art.html.bak&quot;, &quot;art.md&quot;, &quot;art_original.html&quot;, &quot;art_test.txt&quot;, &quot;crates/imp-cli/auth.json&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/mod.rs&quot;, &quot;crates/imp-tui/src/views/editor.rs&quot;, &quot;crates/imp-tui/src/views/top_bar.rs&quot;, &quot;draft.html&quot;, &quot;evals/dirac-comparison/tasks/DynamicCache.json&quot;, &quot;evals/dirac-comparison/tasks/IOverlayWidget.json&quot;, &quot;evals/dirac-comparison/tasks/addLogging.json&quot;, &quot;evals/dirac-comparison/tasks/datadict.json&quot;, &quot;evals/dirac-comparison/tasks/extensionswb_service.json&quot;, &quot;evals/dirac-comparison/tasks/latency.json&quot;, &quot;evals/dirac-comparison/tasks/sendRequest.json&quot;, &quot;evals/dirac-comparison/tasks/stoppingcriteria.json&quot;, &quot;gen_art.py&quot;, &quot;tmp-find-django.sh&quot;, &quot;tools/imp-fix-signature.sh&quot;], &quot;insertions&quot;: 22, &quot;deletions&quot;: 30014, &quot;risk_score&quot;: 5, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;very high churn (30036 lines)&quot;]}, {&quot;sha&quot;: &quot;31e1a04ab84b95d91e150b6600bf0f5e4523c3cd&quot;, &quot;short&quot;: &quot;31e1a04&quot;, &quot;subject&quot;: &quot;Build workflow runtime foundations&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T15:31:11-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-cli/src/lib.rs&quot;, &quot;crates/imp-core/src/agent/events.rs&quot;, &quot;crates/imp-core/src/agent/loop_state.rs&quot;, &quot;crates/imp-core/src/agent/mod.rs&quot;, &quot;crates/imp-core/src/agent/run_loop.rs&quot;, &quot;crates/imp-core/src/agent/tool_execution.rs&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/context_prefill.rs&quot;, &quot;crates/imp-core/src/evidence.rs&quot;, &quot;crates/imp-core/src/imp_session.rs&quot;, &quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/mana_prompt_context.rs&quot;, &quot;crates/imp-core/src/mana_worker.rs&quot;, &quot;crates/imp-core/src/reference_monitor.rs&quot;, &quot;crates/imp-core/src/tools/ask.rs&quot;, &quot;crates/imp-core/src/tools/bash.rs&quot;, &quot;crates/imp-core/src/tools/edit.rs&quot;, &quot;crates/imp-core/src/tools/extend.rs&quot;, &quot;crates/imp-core/src/tools/git.rs&quot;, &quot;crates/imp-core/src/tools/mana.rs&quot;, &quot;crates/imp-core/src/tools/memory.rs&quot;, &quot;crates/imp-core/src/tools/mod.rs&quot;, &quot;crates/imp-core/src/tools/multi_edit.rs&quot;, &quot;crates/imp-core/src/tools/read.rs&quot;, &quot;crates/imp-core/src/tools/scan/mod.rs&quot;, &quot;crates/imp-core/src/tools/session_search.rs&quot;, &quot;crates/imp-core/src/tools/shell.rs&quot;, &quot;crates/imp-core/src/tools/web/mod.rs&quot;, &quot;crates/imp-core/src/tools/worktree.rs&quot;, &quot;crates/imp-core/src/tools/write.rs&quot;, &quot;crates/imp-core/src/trust.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/mod.rs&quot;, &quot;crates/imp-core/src/workflow/contract.rs&quot;, &quot;crates/imp-core/src/workflow/mod.rs&quot;, &quot;crates/imp-core/src/workflow/verification.rs&quot;, &quot;crates/imp-core/src/workflow/verification_runner.rs&quot;, &quot;crates/imp-lua/src/sandbox.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;, &quot;crates/imp-tui/src/turn_tracker.rs&quot;, &quot;crates/imp-tui/src/views/command_palette.rs&quot;, &quot;docs/autonomy-modes.md&quot;, &quot;docs/imp-next-workflow-runtime.md&quot;, &quot;docs/reference-monitor-policy.md&quot;, &quot;docs/trace-and-evidence-format.md&quot;, &quot;docs/trust-labels-and-provenance.md&quot;, &quot;docs/verification-gates.md&quot;, &quot;docs/worktree-auto.md&quot;], &quot;insertions&quot;: 8086, &quot;deletions&quot;: 108, &quot;risk_score&quot;: 45, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;mostly docs/&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-core/src/agent&quot;, &quot;touches crates/imp-core/src/mana_worker&quot;, &quot;touches crates/imp-core/src/reference_monitor&quot;, &quot;touches crates/imp-core/src/tools/mana&quot;, &quot;touches crates/imp-core/src/workflow&quot;, &quot;touches crates/imp-tui/src/app&quot;, &quot;very high churn (8194 lines)&quot;]}, {&quot;sha&quot;: &quot;424795c9063683de1bce9fee5866bf69028c3599&quot;, &quot;short&quot;: &quot;424795c&quot;, &quot;subject&quot;: &quot;Trace TUI agent startup phases&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T12:26:38-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 72, &quot;deletions&quot;: 15, &quot;risk_score&quot;: 6, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;risky subject keyword&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;d89bafd65ac2f18f6d453f0be3a57df0e0b7b8c3&quot;, &quot;short&quot;: &quot;d89bafd&quot;, &quot;subject&quot;: &quot;Keep title spinner active during agent startup&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T11:56:18-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 42, &quot;deletions&quot;: 4, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;4543ff22420bf6fdb6a4e03055ac370499baa6f0&quot;, &quot;short&quot;: &quot;4543ff2&quot;, &quot;subject&quot;: &quot;Animate chat waiting placeholder each tick&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T11:28:10-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 30, &quot;deletions&quot;: 7, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;ef9ccdd138fd69da5959be53846f883c64d6f8f8&quot;, &quot;short&quot;: &quot;ef9ccdd&quot;, &quot;subject&quot;: &quot;Start TUI agents off the input path&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T08:57:59-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 214, &quot;deletions&quot;: 154, &quot;risk_score&quot;: 7, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;moderate churn (368 lines)&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;79665f4209ab43760f14f3f635a74434826c069d&quot;, &quot;short&quot;: &quot;79665f4&quot;, &quot;subject&quot;: &quot;Restore faster title spinner cadence&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T11:13:19-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/animation.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 8, &quot;deletions&quot;: 8, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;ceb3ef3abdfe6361fbe6daec3b24ce328d52690c&quot;, &quot;short&quot;: &quot;ceb3ef3&quot;, &quot;subject&quot;: &quot;Use clearer title spinner cadence&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T11:11:18-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/animation.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 14, &quot;deletions&quot;: 14, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;98cbab62f34389479859bde907fc5b78ddf3e537&quot;, &quot;short&quot;: &quot;98cbab6&quot;, &quot;subject&quot;: &quot;Reuse rendered tool click map for inspector&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T10:24:33-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;, &quot;crates/imp-tui/src/views/chat.rs&quot;], &quot;insertions&quot;: 67, &quot;deletions&quot;: 16, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;b6c301ec1dcb1b0519bbc0d74883885f14b63a48&quot;, &quot;short&quot;: &quot;b6c301e&quot;, &quot;subject&quot;: &quot;Use spinner for TUI working title&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T10:10:56-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/animation.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 18, &quot;deletions&quot;: 18, &quot;risk_score&quot;: 6, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;risky subject keyword&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;2b6ef71be3d20f628223b9be70bd28ce55290892&quot;, &quot;short&quot;: &quot;2b6ef71&quot;, &quot;subject&quot;: &quot;Document TUI workflow wireframes&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T12:24:20-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;docs/tui-workflow-wireframes.md&quot;], &quot;insertions&quot;: 753, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 3, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;high churn (753 lines)&quot;, &quot;mostly docs/&quot;, &quot;risky subject keyword&quot;]}, {&quot;sha&quot;: &quot;79b49633d66ee8280af9682c945cab5425a7c428&quot;, &quot;short&quot;: &quot;79b4963&quot;, &quot;subject&quot;: &quot;Add trace and evidence artifacts&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T10:26:51-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/agent/events.rs&quot;, &quot;crates/imp-core/src/evidence.rs&quot;, &quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/storage.rs&quot;, &quot;crates/imp-core/src/trace.rs&quot;, &quot;docs/trace-and-evidence-format.md&quot;], &quot;insertions&quot;: 1397, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 5, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;high churn (1397 lines)&quot;, &quot;mostly docs/&quot;, &quot;touches crates/imp-core/src/agent&quot;]}, {&quot;sha&quot;: &quot;e2dba93ca9660c2a24a6256e750773de30e67601&quot;, &quot;short&quot;: &quot;e2dba93&quot;, &quot;subject&quot;: &quot;Add mana workflow ledger model&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T10:26:27-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/mana_next/ledger.rs&quot;, &quot;crates/imp-core/src/mana_next/mod.rs&quot;, &quot;docs/mana-next-compatibility-adapter.md&quot;, &quot;docs/mana-next-examples.md&quot;, &quot;docs/mana-next-migration-test-plan.md&quot;, &quot;docs/mana-next-runtime-event-mapping.md&quot;, &quot;docs/mana-next-storage-strategy.md&quot;, &quot;docs/mana-next-ux.md&quot;, &quot;docs/mana-next-workflow-ledger.md&quot;], &quot;insertions&quot;: 2578, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 0, &quot;risk_label&quot;: &quot;low&quot;, &quot;risk_reasons&quot;: [&quot;mostly docs/&quot;, &quot;risky subject keyword&quot;, &quot;very high churn (2578 lines)&quot;]}, {&quot;sha&quot;: &quot;c483434eba3b7434ae4c6f8739afbceeef9567e2&quot;, &quot;short&quot;: &quot;c483434&quot;, &quot;subject&quot;: &quot;Add workflow contract model&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T10:25:46-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/lib.rs&quot;], &quot;insertions&quot;: 1, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 2, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;risky subject keyword&quot;]}, {&quot;sha&quot;: &quot;0184de68d2fc157f6127826c7e1743799a19d7df&quot;, &quot;short&quot;: &quot;0184de6&quot;, &quot;subject&quot;: &quot;Add workflow contract model&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T10:23:35-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/agent/mod.rs&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/workflow/contract.rs&quot;, &quot;crates/imp-core/src/workflow/mod.rs&quot;, &quot;docs/imp-next-workflow-runtime.md&quot;], &quot;insertions&quot;: 1252, &quot;deletions&quot;: 2, &quot;risk_score&quot;: 15, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;high churn (1254 lines)&quot;, &quot;mostly docs/&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-core/src/agent&quot;, &quot;touches crates/imp-core/src/workflow&quot;]}]</script>
docs/rpc.md:16:`--runtime-json` emits the shared runtime event/state shape alongside legacy JSON fields.
docs/trace-and-evidence-format.md:492:- Experimental events can use an `experimental.` prefix if needed.
crates/imp-llm/src/oauth/kimi_code.rs:236:    #[allow(dead_code)]
crates/imp-llm/src/oauth/kimi_code.rs:239:    #[allow(dead_code)]
docs/mana-next-workflow-ledger.md:79:- `planned`
docs/runtime-event-state-api.md:168:| tool output previews | core semantic preview + TUI rendering | `RuntimeToolCall.output_preview` |
docs/plans/pi-provider-oauth-parity.md:65:| 1 | Z.AI | go | Inventory shows API-key/env based support (`ZAI_API_KEY`), not OAuth. This is the smallest missing provider surface. | Adds GLM/Z.AI model access with low auth complexity. | Low/medium: endpoint/model metadata and OpenAI-compatible routing need validation. | Add provider secret/env mapping; no OAuth credential store. | `imp-llm/src/model.rs`, provider registry, auth/env resolution, CLI/TUI setup lists. | provider registry/model tests, env-secret auth tests, provider construction/base URL tests. | Keep/update `275.7`; prior closed zero-test state should be treated as stale until real tests/code exist. |
docs/plans/pi-provider-oauth-parity.md:67:| 3 | GitHub Copilot | research then go | Pi has strong reference for device flow and Copilot token exchange, but request routing/model enablement is more complex. | Unlocks existing Copilot subscription models. | High: device polling, enterprise domains, Copilot internal token/base URL, model availability. | New OAuth/device credentials, Copilot token refresh, optional enterprise domain metadata. | OAuth module, auth store, provider routing/headers, model registry, CLI/TUI login. | device-flow parsing/polling tests, token/base URL extraction tests, small model routing fixture. | Keep/update `275.8`; old closed zero-test gate is stale. |
docs/plans/pi-provider-oauth-parity.md:77:1. Reopen or replace stale zero-test implementation units for Z.AI and Google Gemini CLI with real gates.
docs/plans/pi-provider-oauth-parity.md:88:- `275.7` Z.AI: keep as implementation target, but current closed zero-test state is stale. Reopen or create a replacement with real tests before implementation.
docs/plans/pi-provider-oauth-parity.md:89:- `275.8` GitHub Copilot: keep as implementation target after research/sequence, but current closed zero-test state is stale.
docs/design/droid-mission-mode-vs-imp-work-plan.md:92:- context staleness after outcome
docs/design/droid-mission-mode-vs-imp-work-plan.md:356:- `wave_planned`
docs/design/droid-mission-mode-vs-imp-work-plan.md:382:- `work(action="runs")` can show orchestration runs and legacy single-task runs.
docs/design/droid-mission-mode-vs-imp-work-plan.md:460:- Ready tasks with stale/missing context can be blocked if `require_context=true`.
docs/design/droid-mission-mode-vs-imp-work-plan.md:552:### 1. Double-claim and stale leases
docs/design/droid-mission-mode-vs-imp-work-plan.md:559:- stale `Active` tasks remain stuck
docs/design/droid-mission-mode-vs-imp-work-plan.md:567:- add stale lease recovery
docs/mana-next-compatibility-adapter.md:127:- Facts with stale TTL should not be promoted to workflow truth without verification.
docs/mana-next-compatibility-adapter.md:272:| old fact with TTL | mana_fact_staleable |
docs/mana-next-compatibility-adapter.md:274:Low-trust or stale content should not become a high-trust workflow authorization source.
docs/rebuild/mana-embedding-surface-audit.md:98:No inspected API exposes lease acquisition, holder identity, heartbeat deadline, capabilities, or resolution.
docs/rebuild/imp-workflow-feature-inventory.md:32:| mana-first 365 child specs | Prior target architecture around mana harness | Defer/supersede for 0.3 | The active workflow artifacts contradict mana-first acceptance. Continuing those specs would create stale product direction. | Create a superseding workflow epic or rewrite 365 before doing more mana-harness spec work. |
docs/rebuild/imp-workflow-feature-inventory.md:56:3. Keep legacy `imp run` / WorkRun command absent from the default CLI.
docs/rebuild/imp-workflow-feature-inventory.md:96:   - Remove stale imp-work-as-active-system language.
docs/rebuild/imp-workflow-feature-inventory.md:130:- real bounded subagent execution or clear experimental marking;
docs/rebuild/imp-session-index-lifecycle-audit.md:41:2. legacy data roots with `root.join("session_index.db")`
docs/rebuild/imp-session-index-lifecycle-audit.md:90:- `rebuild_global_session_index` scans `storage::global_sessions_dir()` plus legacy session dirs and indexes all valid `.jsonl` sessions.
docs/design/imp-semantic-write-execution-contract.md:17:3. **Freshness check before preview**
docs/design/imp-semantic-write-execution-contract.md:19:   - If stale, refresh once; if still stale, reject with a stale-preview error.
docs/design/imp-semantic-write-execution-contract.md:21:   - Ask the hosted semantic adapter for an edit preview only.
docs/design/imp-semantic-write-execution-contract.md:22:   - Convert backend edits into a normalized preview envelope with affected paths, ranges, summary, risk flags, and size estimates.
docs/design/imp-semantic-write-execution-contract.md:26:   - Reject previews that are too large, ambiguous, partial, or include unsupported operations.
docs/design/imp-semantic-write-execution-contract.md:29:   - If approval is required, show the operator-visible preview and require explicit acknowledgement tied to the preview fingerprint.
docs/design/imp-semantic-write-execution-contract.md:32:   - Label it with action kind, target, preview fingerprint, timestamp, and verify command.
docs/design/imp-semantic-write-execution-contract.md:35:   - Re-check that target files have not drifted since preview.
docs/design/imp-semantic-write-execution-contract.md:43:    - Emit a `SemanticWriteResult` receipt with preview, approval, checkpoint, apply, refresh, and verify outcomes.
docs/design/imp-semantic-write-execution-contract.md:48:- A preview is bound to a workspace root, file content hashes, backend generation, action kind, and target location.
docs/design/imp-semantic-write-execution-contract.md:49:- Recompute preview when any affected file changes, the backend reports a newer semantic generation, or the operator changes action parameters.
docs/design/imp-semantic-write-execution-contract.md:50:- Reject instead of recomputing when the backend cannot prove freshness, preview generation repeatedly changes affected paths, or the target symbol/range no longer resolves.
docs/design/imp-semantic-write-execution-contract.md:51:- Approval is invalidated by any recomputed preview. The operator must acknowledge the new preview fingerprint.
docs/design/imp-semantic-write-execution-contract.md:56:- Capture all files that may be edited by the normalized preview, plus minimal metadata needed to explain/restore the action.
docs/design/imp-semantic-write-execution-contract.md:57:- Label format should include: `semantic-write:<action-kind>:<target>:<preview-fingerprint>`.
docs/design/imp-semantic-write-execution-contract.md:63:- `ApprovalPosture::AutoAllow` may proceed only for allowlisted low-risk actions with bounded preview size and fresh backend state.
docs/design/imp-semantic-write-execution-contract.md:64:- `ApprovalPosture::PreviewRequired` requires rendering the preview but may continue after explicit model/operator acknowledgement if policy allows.
docs/design/imp-semantic-write-execution-contract.md:65:- `ApprovalPosture::OperatorRequired` requires a human-visible acknowledgement of the exact preview fingerprint.
docs/design/imp-semantic-write-execution-contract.md:67:- Any preview drift, stale state, policy change, or target change clears approval.
docs/design/imp-semantic-write-execution-contract.md:79:- **Apply drift**: abort before writing, recompute preview if safe, otherwise require new approval.
docs/design/imp-semantic-write-execution-contract.md:83:- **Approval rejection**: no checkpoint or apply; record rejected preview as non-durable runtime output unless explicitly promoted.
docs/design/imp-semantic-write-execution-contract.md:92:- preview fingerprint and affected paths;
docs/design/imp-semantic-write-execution-contract.md:107:2. Capability lookup allows rename for the language/backend but requires operator preview.
docs/design/imp-semantic-write-execution-contract.md:109:4. Operator approves preview fingerprint.
docs/design/imp-semantic-write-execution-contract.md:116:2. Policy allows auto-apply only if preview affects that file and contains import-order edits only.
docs/design/imp-semantic-write-execution-contract.md:125:3. If preview includes unrelated edits, reject as policy drift.
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:3:This audit resolves mana unit `264.9`, a follow-up from the broader storage topology audit. It determines whether prompt-template files and TOML-defined shell-tool roots are active production storage surfaces or should be treated as experimental/unwired before the storage contract preserves them.
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:63:Prompt templates are **defined but unwired/experimental** in the current shipped path.
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:74:But document status as **experimental/unwired** and avoid promising runtime discovery semantics beyond the helper itself.
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:124:TOML shell tools are **implemented as a loader and executable tool type, but unwired/experimental** in the current shipped path.
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:130:Keep roots reserved only as experimental candidate roots:
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:150:- prompts and shell tools should be listed as **reserved/experimental surfaces**, not core active surfaces;
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:155:This avoids preserving dead baggage as if it were shipped behavior while still recording the intended extension points.
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:173:Both prompt templates and TOML shell tools are source-defined and tested, but neither has a production call site in the inspected shipped paths. Treat them as experimental/reserved storage surfaces in the topology. Do not migrate or promise them as active runtime behavior until separate product and policy work wires them deliberately.
docs/design/imp-work-mana-removal-ledger.md:24:| `.mana` support narrowed to import-only/legacy | Not yet implemented | open before destructive removal |
docs/design/imp-work-mana-removal-ledger.md:33:5. Mark `.mana` support import-only/legacy.
docs/rebuild/imp-durable-storage-surface-audit.md:14:- legacy roots remain compatibility inputs.
docs/rebuild/imp-durable-storage-surface-audit.md:25:| raw sessions | user global durable data | `~/.imp/sessions/*.jsonl`; historical `~/.local/share/imp/sessions/*.jsonl` | session/imp_session/storage helpers | current canonical root plus legacy recovery inputs | many raw sessions can exist without FTS index |
docs/rebuild/imp-durable-storage-surface-audit.md:32:| prompts | user global + project local templates | `~/.imp/prompts/**`, `<project>/.imp/prompts/**` | `resources.rs` prompt discovery | user + project discovery | historically defined but production wiring may be sparse; avoid preserving dead surfaces blindly |
docs/rebuild/imp-durable-storage-surface-audit.md:34:| shell tool TOML | possible tool extension config | caller-provided dirs to `load_shell_tools` | `tools/shell.rs` | production roots not clearly wired in current shipped path | may be an unwired/dead surface; should be confirmed before normalizing |
docs/rebuild/imp-durable-storage-surface-audit.md:54:- make legacy roots migration inputs rather than continuing write destinations;
docs/rebuild/imp-durable-storage-surface-audit.md:65:5. **Potential dead surfaces** — prompts and shell-tool TOML need proof of runtime wiring or explicit deprecation.
docs/rebuild/imp-durable-storage-surface-audit.md:76:- `264.8`: migration from legacy XDG/macOS paths.
docs/rebuild/imp-durable-storage-surface-audit.md:84:The current topology has converged toward `~/.imp` plus project `.imp`, but durable surfaces still need class-specific policy. Session recovery failure is not just path mismatch; it also requires indexing lifecycle work. The next storage work should preserve the config/data/secret/index distinctions while eliminating ad hoc path assembly and ambiguity around legacy roots.
crates/imp-core/src/mana_run_state.rs:30:    #[allow(dead_code)]
crates/imp-core/src/mana_run_state.rs:107:    classify_stale_unfinished_runs(&mut store);
crates/imp-core/src/mana_run_state.rs:122:fn classify_stale_unfinished_runs(store: &mut PersistedRunStore) {
crates/imp-core/src/mana_run_state.rs:132:                "Run state is stale after process restart or lost background worker; inspect logs before rerun"
docs/rebuild/imp-normalized-storage-contract.md:27:- `prompts/` — reserved/experimental prompt templates.
docs/rebuild/imp-normalized-storage-contract.md:28:- `tools/` — reserved/experimental shell-tool definitions; do not auto-enable without policy.
docs/rebuild/imp-normalized-storage-contract.md:43:- `prompts/` — reserved/experimental prompt templates.
docs/rebuild/imp-normalized-storage-contract.md:44:- `tools/` — reserved/experimental shell-tool definitions; policy-gated before activation.
docs/rebuild/imp-normalized-storage-contract.md:69:1. Read legacy roots as migration/recovery sources.
docs/rebuild/imp-normalized-storage-contract.md:71:3. Do not delete legacy data automatically.
docs/rebuild/imp-normalized-storage-contract.md:91:- legacy-root discovery helpers for read-only migration/recovery.
docs/rebuild/imp-normalized-storage-contract.md:95:- No immediate deletion of legacy data.
docs/rebuild/imp-attach-path-cutover.md:183:- If no lease API is available, current path can still run behind an experimental flag.
docs/rebuild/imp-attach-path-cutover.md:217:- Template mode may stay temporarily but should be documented as legacy.
docs/rebuild/imp-attach-path-cutover.md:260:2. Add an experimental imp path that calls those APIs around the existing `mana_worker` execution.
docs/rebuild/imp-session-storage-search-recovery-audit.md:34:`session_search` (`recall`) looks for an existing global index first through `storage::existing_global_file(storage::global_session_index_path, "session_index.db")`, then legacy data roots, and finally falls back to `storage::global_session_index_path()`.
docs/rebuild/imp-session-storage-search-recovery-audit.md:141:3. Decide whether to one-time import legacy `/Users/asher/.local/share/imp/sessions` into current `~/.imp/sessions` or teach indexing to scan both.
crates/imp-core/src/agent/loop_state.rs:51:    pub planned_tools: usize,
crates/imp-core/src/agent/loop_state.rs:61:            planned_tools: 0,
crates/imp-core/src/agent/loop_state.rs:74:    pub fn record_tool_plan(&mut self, planned_tools: usize) {
crates/imp-core/src/agent/loop_state.rs:75:        self.planned_tools = planned_tools;
crates/imp-core/src/agent/loop_state.rs:271:#[allow(dead_code)]
docs/rebuild/imp-machine-streamed-error-envelope.md:9:- Headless JSON legacy agent errors: `{"type":"error","error":"..."}`.
docs/rebuild/imp-machine-streamed-error-envelope.md:10:- Headless JSON legacy stream errors: `{"type":"stream_error","error":"..."}`.
docs/rebuild/imp-machine-streamed-error-envelope.md:11:- RPC legacy agent errors: `{"type":"error","error":"..."}`.
docs/rebuild/imp-machine-streamed-error-envelope.md:12:- RPC legacy stream errors: `{"type":"stream_error","error":"..."}`.
docs/rebuild/imp-machine-streamed-error-envelope.md:13:- RPC and runtime-json paths already attach `runtime_event` and `runtime_state` alongside the legacy payload.
docs/rebuild/imp-machine-streamed-error-envelope.md:79:- keep `runtime_event` additive, not a replacement for legacy fields until a versioned protocol migration exists.
docs/rebuild/imp-machine-streamed-error-envelope.md:89:- have `legacy_json_event_value`, `stream_event_to_json`, `rpc_agent_event_legacy_json`, and `rpc_stream_event_to_json` include `error_info` consistently;
docs/rebuild/imp-machine-streamed-error-envelope.md:90:- optionally enrich `RuntimeEventKind::Error` later, but avoid making that a prerequisite if the additive legacy payload is the smallest safe slice.
docs/rebuild/imp-machine-streamed-error-envelope.md:112:Add structured machine error metadata as an additive compatibility field. Preserve legacy flat strings until a separate versioned machine-output migration removes them.
crates/imp-core/src/config.rs:247:    /// Allow overwrites but return warnings for unread/stale files.
crates/imp-core/src/config.rs:250:    /// Block overwrites unless the file was read in this session and is not stale.
crates/imp-core/src/config.rs:252:    /// Block only stale overwrites; unread overwrites still warn.
crates/imp-core/src/agent/events.rs:284:                        args_preview: Some(arguments.to_string()),
crates/imp-core/src/agent/events.rs:312:                    args_preview: Some(args.to_string()),
crates/imp-core/src/agent/events.rs:333:                    output_preview: tool_result_summary(result),
crates/imp-core/src/agent/events.rs:781:                    .args_preview
crates/imp-core/src/agent/tool_execution.rs:22:fn legacy_policy_error_message(
crates/imp-core/src/agent/tool_execution.rs:48:fn legacy_policy_checkpoint_reason(reason: &PolicyReason) -> &'static str {
crates/imp-core/src/agent/tool_execution.rs:365:            let mut result = crate::tools::ToolOutput::error(legacy_policy_error_message(
crates/imp-core/src/agent/tool_execution.rs:383:                Some(legacy_policy_checkpoint_reason(&reason).to_string()),
crates/imp-core/src/storage.rs:289:pub fn legacy_config_roots() -> Vec<PathBuf> {
crates/imp-core/src/storage.rs:297:pub fn legacy_data_roots() -> Vec<PathBuf> {
crates/imp-core/src/storage.rs:312:    roots.extend(legacy_config_roots());
crates/imp-core/src/storage.rs:318:    roots.extend(legacy_data_roots());
crates/imp-core/src/storage.rs:322:pub fn existing_global_file(path_fn: fn() -> PathBuf, legacy_subpath: &str) -> Option<PathBuf> {
crates/imp-core/src/storage.rs:329:        let path = root.join(legacy_subpath);
crates/imp-core/src/storage.rs:336:        let path = root.join(legacy_subpath);
crates/imp-core/src/storage.rs:350:    legacy_config_roots()
crates/imp-core/src/storage.rs:361:    legacy_config_roots()
crates/imp-core/src/storage.rs:367:pub fn reconcile_legacy_into_global_root() -> io::Result<Vec<PathBuf>> {
crates/imp-core/src/storage.rs:372:        legacy_config_roots()
crates/imp-core/src/storage.rs:379:        legacy_config_roots()
crates/imp-core/src/storage.rs:386:        legacy_config_roots()
crates/imp-core/src/storage.rs:393:        legacy_config_roots()
crates/imp-core/src/storage.rs:400:        legacy_config_roots()
crates/imp-core/src/storage.rs:407:        legacy_config_roots()
crates/imp-core/src/storage.rs:420:        legacy_config_roots()
crates/imp-core/src/storage.rs:427:        legacy_config_roots()
crates/imp-core/src/storage.rs:434:        legacy_config_roots()
crates/imp-core/src/storage.rs:441:        legacy_config_roots()
crates/imp-core/src/storage.rs:448:        legacy_data_roots()
crates/imp-core/src/storage.rs:455:        legacy_data_roots()
crates/imp-core/src/storage.rs:681:    fn reconcile_file_candidates_copies_first_existing_legacy_file() {
crates/imp-core/src/storage.rs:684:        let legacy = temp.path().join("legacy").join("config.toml");
crates/imp-core/src/storage.rs:685:        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
crates/imp-core/src/storage.rs:686:        fs::write(&legacy, "model = \"sonnet\"\n").unwrap();
crates/imp-core/src/storage.rs:688:        let migrated = reconcile_file_candidates(target.clone(), vec![legacy.clone()]).unwrap();
crates/imp-core/src/storage.rs:697:        let legacy = temp.path().join("legacy").join("skills").join("my-skill");
crates/imp-core/src/storage.rs:698:        fs::create_dir_all(&legacy).unwrap();
crates/imp-core/src/storage.rs:699:        fs::write(legacy.join("SKILL.md"), "# Skill\n").unwrap();
crates/imp-core/src/storage.rs:703:            vec![temp.path().join("legacy").join("skills")],
crates/imp-core/src/mana_worker.rs:10://! - legacy `mana run` compatibility flows — transitional dispatch into imp workers
crates/imp-core/src/mana_worker.rs:18://! legacy mana run compatibility = transitional parallel dispatch into imp workers
crates/imp-core/src/mana_worker.rs:882:        "Stay inside this unit's scope. Update mana notes with discoveries or blockers. Run the verify command or equivalent focused checks before claiming completion. If the stored verify is stale/invalid, record equivalent evidence and close with force only with an explicit reason. Do not retry a failed approach unchanged.",
crates/imp-core/src/usage.rs:378:                        request_id: legacy_request_id(id),
crates/imp-core/src/usage.rs:436:                        request_id: legacy_request_id(id),
crates/imp-core/src/usage.rs:459:/// Records are sorted so canonical rows win over legacy fallbacks, then the
crates/imp-core/src/usage.rs:601:fn legacy_request_id(assistant_message_id: &str) -> String {
crates/imp-core/src/usage.rs:602:    format!("legacy-assistant:{assistant_message_id}")
crates/imp-core/src/usage.rs:622:    fn legacy_assistant_entry(id: &str, timestamp: u64, usage: Usage) -> SessionEntry {
crates/imp-core/src/usage.rs:694:    fn usage_reader_falls_back_to_legacy_assistant_usage() {
crates/imp-core/src/usage.rs:695:        let entries = vec![legacy_assistant_entry(
crates/imp-core/src/usage.rs:696:            "assistant-legacy",
crates/imp-core/src/usage.rs:709:        assert_eq!(record.request_id, "legacy-assistant:assistant-legacy");
crates/imp-core/src/usage.rs:719:    fn canonical_record_suppresses_legacy_fallback_for_same_assistant_message() {
crates/imp-core/src/usage.rs:727:            legacy_assistant_entry("assistant-1", 100, usage.clone()),
crates/imp-core/src/usage.rs:853:    fn aggregate_usage_keeps_distinct_legacy_records() {
crates/imp-core/src/usage.rs:855:            legacy_assistant_entry(
crates/imp-core/src/usage.rs:865:            legacy_assistant_entry(
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:164:        // Native imp-work names that do not exist in legacy mana but mutate or
crates/imp-core/src/agent/mod.rs:107:    /// Tracks which files have been read; used for staleness and unread-edit warnings.
crates/imp-core/src/agent/mod.rs:148:    /// Runtime-owned autonomy TODO list used to continue until obligations resolve.
crates/imp-core/src/agent/mod.rs:619:    "You have recorded or planned work, but the requested outcome is not satisfied yet. Continue working until the user's requested outcome is satisfied, or until concrete evidence shows it cannot be completed. Do not stop merely because you recorded a plan, updated a workflow, or completed one intermediate step."
crates/imp-core/src/agent/mod.rs:1377:    #[allow(dead_code)]
crates/imp-core/src/agent/mod.rs:3028:                        serde_json::json!({"action": "update", "id": "task-1", "summary": "planned"}),
crates/imp-core/src/agent/mod.rs:3075:                text.contains("You have recorded or planned work")
crates/imp-core/src/system_prompt.rs:847:    fn system_prompt_no_legacy_mana_guidance_or_delegation_in_prompt() {
crates/imp-core/src/system_prompt.rs:849:        // Verify large legacy prompt blocks no longer appear regardless of tool availability.
crates/imp-core/src/system_prompt.rs:1280:            "Project memory status:\nWarnings:\n- stale fact\n\nWorking on:\n- [7] Fix auth flow";
crates/imp-core/src/agent/recovery.rs:108:                    state.planned = true;
crates/imp-core/src/agent/recovery.rs:112:                    state.planned = true;
crates/imp-core/src/agent/recovery.rs:142:            } else if state.planned {
crates/imp-core/src/agent/recovery.rs:180:    planned: bool,
crates/imp-core/src/agent/recovery.rs:379:    fn read_only_planned_not_started_is_retryable() {
crates/imp-core/src/reference_monitor.rs:264:            Some("Use the native workflow tool instead of shelling out to legacy mana".into());
crates/imp-core/src/reference_monitor.rs:1464:    fn policy_trace_records_cover_legacy_mana_policy_outcomes() {
crates/imp-core/src/workflow/child_workflow.rs:124:    pub stale: Option<ChildStaleState>,
crates/imp-core/src/workflow/child_workflow.rs:141:                message: Some("child workflow planned".into()),
crates/imp-core/src/workflow/child_workflow.rs:145:            stale: None,
crates/imp-core/src/workflow/child_workflow.rs:184:    pub fn mark_stale(&mut self, reason: impl Into<String>, idle_timeout_secs: u64) {
crates/imp-core/src/workflow/child_workflow.rs:186:        self.stale = Some(ChildStaleState {
crates/imp-core/src/workflow/child_workflow.rs:194:            Some("child workflow is stale".into()),
crates/imp-core/src/workflow/child_workflow.rs:211:            } => self.mark_stale(reason, idle_timeout_secs),
crates/imp-core/src/workflow/child_workflow.rs:218:    pub fn stale_policy_decision(
crates/imp-core/src/workflow/child_workflow.rs:907:    fn child_workflow_cancellation_and_stale_metadata_are_recorded() {
crates/imp-core/src/workflow/child_workflow.rs:917:        run.mark_stale("idle timeout", 300);
crates/imp-core/src/workflow/child_workflow.rs:919:        assert_eq!(run.stale.as_ref().unwrap().idle_timeout_secs, 300);
crates/imp-core/src/workflow/child_workflow.rs:920:        assert_eq!(run.stale.as_ref().unwrap().reason, "idle timeout");
crates/imp-core/src/workflow/child_workflow.rs:924:    fn stale_detection_marks_idle_child_as_stale() {
crates/imp-core/src/workflow/child_workflow.rs:935:            run.stale_policy_decision(&policy, Utc::now(), &ChildWorkflowHealth::default());
crates/imp-core/src/workflow/child_workflow.rs:942:        assert!(run.stale.as_ref().unwrap().reason.contains("idle"));
crates/imp-core/src/workflow/child_workflow.rs:946:    fn stale_detection_can_cancel_on_repeated_failures() {
crates/imp-core/src/workflow/child_workflow.rs:959:        let decision = run.stale_policy_decision(&policy, Utc::now(), &health);
crates/imp-core/src/workflow/child_workflow.rs:973:    fn stale_detection_notifies_parent_for_waiting_states() {
crates/imp-core/src/workflow/child_workflow.rs:979:        let decision = run.stale_policy_decision(&ChildStalePolicy::default(), Utc::now(), &health);
crates/imp-core/src/workflow/child_workflow.rs:987:    fn stale_detection_ignores_terminal_children() {
crates/imp-core/src/workflow/child_workflow.rs:991:        let decision = run.stale_policy_decision(
crates/imp-core/src/contracts.rs:4://! and future runner surfaces. They used to live in the experimental
crates/imp-core/src/contracts.rs:5://! the earlier experimental contracts crate, but currently only imp consumes
crates/imp-core/src/imp_session.rs:264:        let _ = storage::reconcile_legacy_into_global_root();
crates/imp-core/src/imp_session.rs:881:            | "kimi-k2-0905-preview"
crates/imp-core/src/imp_session.rs:882:            | "kimi-k2-turbo-preview"
crates/imp-core/src/resources.rs:455:        fs::write(project.join("AGENTS.md"), "project-legacy").unwrap();
crates/imp-core/src/resources.rs:462:        let legacy_idx = results
crates/imp-core/src/resources.rs:464:            .position(|a| a.content == "project-legacy")
crates/imp-core/src/resources.rs:466:        assert!(canonical_idx < legacy_idx);
crates/imp-core/src/resources.rs:470:    fn resource_discover_agents_md_dedupes_legacy_global_copy() {
crates/imp-core/src/mana_prompt_context.rs:158:            if let Some(stale_after) = unit.stale_after {
crates/imp-core/src/mana_prompt_context.rs:159:                if now > stale_after {
crates/imp-core/src/mana_prompt_context.rs:160:                    let days_stale = (now - stale_after).num_days();
crates/imp-core/src/mana_prompt_context.rs:163:                        unit.title, days_stale
crates/imp-core/src/mana_prompt_context.rs:177:            if let Some(stale_after) = unit.stale_after {
crates/imp-core/src/mana_prompt_context.rs:178:                if now > stale_after {
crates/imp-core/src/mana_prompt_context.rs:179:                    let days_stale = (now - stale_after).num_days();
crates/imp-core/src/mana_prompt_context.rs:182:                        unit.title, days_stale
crates/imp-core/src/mana_prompt_context.rs:421:        let mut stale = Unit::new(
crates/imp-core/src/mana_prompt_context.rs:425:        stale.last_verified = None;
crates/imp-core/src/mana_prompt_context.rs:436:                    unit: stale,
crates/imp-core/src/mana_prompt_context.rs:511:            "A very long working unit title that should be truncated before it reaches the prompt because startup context should stay compact and preview oriented",
crates/imp-core/examples/tool_ab_harness.rs:164:fn create_agent_legacy(provider: Arc<dyn Provider>, cwd: PathBuf) -> (Agent, AgentHandle) {
crates/imp-core/examples/tool_ab_harness.rs:340:    if variant_name == "legacy" {
crates/imp-core/examples/tool_ab_harness.rs:374:fn responses_search_legacy() -> Vec<Vec<StreamEvent>> {
crates/imp-core/examples/tool_ab_harness.rs:406:fn responses_list_legacy() -> Vec<Vec<StreamEvent>> {
crates/imp-core/examples/tool_ab_harness.rs:426:fn responses_find_legacy() -> Vec<Vec<StreamEvent>> {
crates/imp-core/examples/tool_ab_harness.rs:461:fn responses_scan_extract_legacy() -> Vec<Vec<StreamEvent>> {
crates/imp-core/examples/tool_ab_harness.rs:495:fn responses_search_then_read_legacy() -> Vec<Vec<StreamEvent>> {
crates/imp-core/examples/tool_ab_harness.rs:531:fn responses_search_then_edit_legacy() -> Vec<Vec<StreamEvent>> {
crates/imp-core/examples/tool_ab_harness.rs:575:fn responses_repeat_legacy() -> Vec<Vec<StreamEvent>> {
crates/imp-core/examples/tool_ab_harness.rs:586:    responses_repeat_legacy()
crates/imp-core/examples/tool_ab_harness.rs:603:            responses_search_legacy as fn() -> Vec<Vec<StreamEvent>>,
crates/imp-core/examples/tool_ab_harness.rs:610:            responses_list_legacy,
crates/imp-core/examples/tool_ab_harness.rs:617:            responses_find_legacy,
crates/imp-core/examples/tool_ab_harness.rs:624:            responses_scan_extract_legacy,
crates/imp-core/examples/tool_ab_harness.rs:631:            responses_search_then_read_legacy,
crates/imp-core/examples/tool_ab_harness.rs:638:            responses_search_then_edit_legacy,
crates/imp-core/examples/tool_ab_harness.rs:645:            responses_repeat_legacy,
crates/imp-core/examples/tool_ab_harness.rs:652:    for (name, prompt, setup, legacy_responses, reduced_responses) in scenarios {
crates/imp-core/examples/tool_ab_harness.rs:653:        if variant == "both" || variant == "legacy" {
crates/imp-core/examples/tool_ab_harness.rs:655:                "live" => run_live_variant(&mut summaries, "legacy", name, prompt, setup).await?,
crates/imp-core/examples/tool_ab_harness.rs:659:                        "legacy",
crates/imp-core/examples/tool_ab_harness.rs:663:                        legacy_responses,
crates/imp-core/examples/tool_ab_harness.rs:664:                        create_agent_legacy,
crates/imp-core/src/tools/mana.rs:45:    // Transitional compatibility: runtime still accepts legacy alias fields even though
crates/imp-core/src/tools/mana.rs:397:        store.classify_stale_unfinished_runs();
crates/imp-core/src/tools/mana.rs:410:    fn classify_stale_unfinished_runs(&mut self) {
crates/imp-core/src/tools/mana.rs:420:                    "Run state is stale after process restart or lost background worker; inspect logs before rerun".to_string(),
crates/imp-core/src/tools/mana.rs:424:                    "Run marked interrupted: stale persisted running state after reload"
crates/imp-core/src/tools/mana.rs:650:            Some(format!("Dry run: {} planned wave(s)", rounds.len()))
crates/imp-core/src/tools/mana.rs:949:            "hint": "When stored verify is stale or invalid, rerun equivalent checks and close with force=true plus a reason that names the passing commands/evidence.",
crates/imp-core/src/tools/mana.rs:965:            "If the stored verify command is stale or invalid, run equivalent focused checks, then close with force=true and reason containing the passing commands/evidence.",
crates/imp-core/src/tools/mana.rs:1219:        // Transitional compatibility: accept legacy `job` at runtime, but keep it out
crates/imp-core/src/tools/mana.rs:1224:            "kind must be one of: epic, task, fact (legacy runtime alias: job; got {other})"
crates/imp-core/src/tools/mana.rs:2103:            "Run was marked interrupted/stale after reload; inspect logs before rerun".to_string(),
crates/imp-core/src/tools/mana.rs:2118:        next_actions.push("Verify candidate-complete units or close with equivalent evidence if stored verify is stale".to_string());
crates/imp-core/src/tools/mana.rs:2121:        next_actions.push("Inspect logs/agents before assuming the run is stale".to_string());
crates/imp-core/src/tools/mana.rs:2132:        "stale_workers": if interrupted { json!([{"run_id": state.run_id, "status": state.status}]) } else { json!([]) },
crates/imp-core/src/tools/mana.rs:2168:            "Interrupted: persisted running state is stale; inspect logs before rerun".to_string(),
crates/imp-core/src/tools/mana.rs:2173:        let preview = state
crates/imp-core/src/tools/mana.rs:2180:        lines.push(format!("Units: {preview}"));
crates/imp-core/src/tools/mana.rs:3195:                // Transitional compatibility: runtime still accepts legacy `paths_csv`, but
crates/imp-core/src/tools/mana.rs:3231:                                    "stale_after": result.unit.stale_after,
crates/imp-core/src/tools/mana.rs:3242:                        "Verified {}/{} facts · {} stale · {} failing · {} suspect",
crates/imp-core/src/tools/mana.rs:3245:                        result.stale_count,
crates/imp-core/src/tools/mana.rs:3252:                        "stale_count": result.stale_count,
crates/imp-core/src/tools/mana.rs:5527:    fn mana_run_state_marks_stale_running_runs_interrupted() {
crates/imp-core/src/tools/mana.rs:5540:        store.classify_stale_unfinished_runs();
crates/imp-core/src/tools/mana.rs:5544:        assert!(state.error.as_deref().unwrap_or_default().contains("stale"));
crates/imp-core/src/tools/mana.rs:5549:            output.details["recovery"]["stale_workers"][0]["run_id"],
crates/imp-core/src/tools/mana.rs:5559:                .contains("interrupted/stale")));
crates/imp-core/src/codeintel/mod.rs:612:        let stale = CodeIntelEnvelope {
crates/imp-core/src/codeintel/mod.rs:618:        assert!(!stale.is_complete_and_fresh());
crates/imp-core/examples/tool_surface_live.rs:38:        for variant in ["legacy", "reduced"] {
crates/imp-core/examples/tool_surface_live.rs:55:            if variant == "legacy" {
crates/imp-core/src/runtime.rs:266:                    tool.output_preview = Some(match &tool.output_preview {
crates/imp-core/src/runtime.rs:575:    pub args_preview: Option<String>,
crates/imp-core/src/runtime.rs:576:    pub output_preview: Option<String>,
crates/imp-core/src/runtime.rs:901:                        args_preview: Some("cargo test".into()),
crates/imp-core/src/runtime.rs:915:                        output_preview: Some("ok".into()),
crates/imp-core/src/runtime.rs:1019:                    output_preview: Some("ok".into()),
crates/imp-core/src/tools/multi_edit.rs:254:    } else if tracker.is_stale(path) {
crates/imp-core/src/tools/prototype.rs:153:                "if command -v node >/dev/null 2>&1 && node --experimental-strip-types {file_name}; then :; elif command -v bun >/dev/null 2>&1; then bun run {file_name}; elif command -v deno >/dev/null 2>&1; then deno run --quiet {file_name}; else echo 'typescript prototypes require node with --experimental-strip-types, bun, or deno' >&2; exit 127; fi"
crates/imp-core/src/tools/prototype.rs:209:                    "description": "Scratch language/runtime for the experiment. JavaScript uses node; TypeScript prefers node --experimental-strip-types with bun/deno fallback; Go uses go run; Elixir uses elixir; other languages use their standard CLI when installed."
crates/imp-core/src/tools/prototype.rs:361:            content: output_preview,
crates/imp-core/src/tools/prototype.rs:458:        text.push_str(&output_preview);
crates/imp-core/src/tools/prototype.rs:664:    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
crates/imp-core/src/tools/prototype.rs:673:            _ = tokio::time::sleep_until(deadline) => {
crates/imp-core/src/tools/edit.rs:141:        // Check for unread or stale file — warn but don't block.
crates/imp-core/src/tools/edit.rs:149:                Some(t) if t.is_stale(&path) => Some(format!(
crates/imp-core/src/tools/edit.rs:296:    legacy: &str,
crates/imp-core/src/tools/edit.rs:301:        .or_else(|| params.get(legacy).and_then(|v| v.as_str()))
crates/imp-core/src/tools/edit.rs:304:fn get_bool_param(params: &serde_json::Value, primary: &str, legacy: &str) -> Option<bool> {
crates/imp-core/src/tools/edit.rs:308:        .or_else(|| params.get(legacy).and_then(|v| v.as_bool()))
crates/imp-core/src/tools/edit.rs:491:    let preview = truncate_chars_with_suffix(content, 200, "");
crates/imp-core/src/tools/edit.rs:494:         First 200 chars of file:\n{preview}"
crates/imp-core/src/tools/edit.rs:1080:    async fn anchored_edit_rejects_stale_anchor_without_writing() {
crates/imp-core/src/tools/edit.rs:1082:        let file = dir.path().join("stale.txt");
crates/imp-core/src/tools/edit.rs:1096:                "c-anchor-stale",
crates/imp-core/src/tools/edit.rs:1098:                    "path": "stale.txt",
crates/imp-core/src/tools/bash.rs:558:    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
crates/imp-core/src/tools/bash.rs:564:            _ = tokio::time::sleep_until(deadline) => {
crates/imp-core/src/tools/write.rs:250:    let is_stale = tracker.is_stale(path);
crates/imp-core/src/tools/write.rs:276:    ) && is_stale
crates/imp-core/src/tools/write.rs:280:                "Write overwrite policy blocks overwriting stale files. Re-read before overwriting: {}",
crates/imp-core/src/tools/write.rs:294:    } else if is_stale {
crates/imp-core/src/tools/write.rs:295:        check.warning_codes.push("stale_overwrite");
crates/imp-core/src/tools/write.rs:583:    async fn write_block_stale_policy_blocks_stale_overwrite() {
crates/imp-core/src/tools/write.rs:596:                "c-block-stale",
crates/imp-core/src/tools/write.rs:797:                serde_json::json!({"path": "preview.rs", "content": "fn main() {\n    println!(\"hi\");\n}\n"}),
crates/imp-core/src/tools/write.rs:807:            .ends_with("preview.rs"));
crates/imp-core/src/tools/write.rs:811:            .ends_with("preview.rs: 34 bytes created"));
crates/imp-core/src/tools/read.rs:46:                    "description": "When true, include opaque per-line anchors for stale-safe anchored edits. Anchors are session-local integrity markers, not security tokens."
crates/imp-core/src/tools/read.rs:238:        // Record that this file was read (for staleness and unread-edit detection).
crates/imp-core/src/tools/mod.rs:125:    pub fn is_stale(&self, path: &Path) -> bool {
crates/imp-core/src/tools/mod.rs:216:    /// Tracks file reads for staleness detection and unread-edit warnings.
crates/imp-core/src/tools/mod.rs:582:    /// Compatibility aliases such as legacy `multi_edit` are intentionally hidden
crates/imp-core/src/tools/mod.rs:1250:    fn file_track_is_stale_false_for_unread_file() {
crates/imp-core/src/tools/mod.rs:1256:        // Unread file is never stale (no baseline to compare against)
crates/imp-core/src/tools/mod.rs:1257:        assert!(!tracker.is_stale(&file));
crates/imp-core/src/tools/mod.rs:1261:    fn file_track_is_stale_false_immediately_after_read() {
crates/imp-core/src/tools/mod.rs:1268:        // No modification since read — should not be stale
crates/imp-core/src/tools/mod.rs:1269:        assert!(!tracker.is_stale(&file));
crates/imp-core/src/tools/mod.rs:1273:    fn file_track_is_stale_detects_external_modification() {
crates/imp-core/src/tools/mod.rs:1289:            tracker.is_stale(&file),
crates/imp-core/src/tools/mod.rs:1290:            "file with advanced mtime should be stale"
crates/imp-core/src/tools/web/mod.rs:454:    fn max_results_accepts_legacy_camel_case() {
crates/imp-core/src/tools/web/mod.rs:456:        let legacy = serde_json::json!({"maxResults": 8});
crates/imp-core/src/tools/web/mod.rs:460:        assert_eq!(max_results_from_params(&legacy), 8);

## Rust compile reachability rough scan (pub modules)
crates/imp-core/src/lib.rs:3:pub mod agent;
crates/imp-core/src/lib.rs:4:pub mod builder;
crates/imp-core/src/lib.rs:5:pub mod codeintel;
crates/imp-core/src/lib.rs:6:pub mod compaction;
crates/imp-core/src/lib.rs:7:pub mod config;
crates/imp-core/src/lib.rs:8:pub mod context;
crates/imp-core/src/lib.rs:9:pub mod context_prefill;
crates/imp-core/src/lib.rs:10:pub mod contracts;
crates/imp-core/src/lib.rs:11:pub mod error;
crates/imp-core/src/lib.rs:12:pub mod error_display;
crates/imp-core/src/lib.rs:13:pub mod eval_candidate;
crates/imp-core/src/lib.rs:14:pub mod eval_candidate_closeout;
crates/imp-core/src/lib.rs:15:pub mod evidence;
crates/imp-core/src/lib.rs:16:pub mod guardrails;
crates/imp-core/src/lib.rs:17:pub mod hooks;
crates/imp-core/src/lib.rs:18:pub mod imp_session;
crates/imp-core/src/lib.rs:19:pub mod import;
crates/imp-core/src/lib.rs:20:pub mod learning;
crates/imp-core/src/lib.rs:22:pub mod mana_next;
crates/imp-core/src/lib.rs:24:pub mod mana_prompt_context;
crates/imp-core/src/lib.rs:25:pub mod mana_review;
crates/imp-core/src/lib.rs:27:pub mod mana_run_state;
crates/imp-core/src/lib.rs:29:pub mod mana_worker;
crates/imp-core/src/lib.rs:30:pub mod memory;
crates/imp-core/src/lib.rs:31:pub mod personality;
crates/imp-core/src/lib.rs:32:pub mod policy;
crates/imp-core/src/lib.rs:33:pub mod reference_monitor;
crates/imp-core/src/lib.rs:34:pub mod repo_intelligence;
crates/imp-core/src/lib.rs:35:pub mod resources;
crates/imp-core/src/lib.rs:36:pub mod retry;
crates/imp-core/src/lib.rs:37:pub mod roles;
crates/imp-core/src/lib.rs:38:pub mod run_evidence;
crates/imp-core/src/lib.rs:39:pub mod runtime;
crates/imp-core/src/lib.rs:40:pub mod sdk;
crates/imp-core/src/lib.rs:41:pub mod session;
crates/imp-core/src/lib.rs:42:pub mod session_index;
crates/imp-core/src/lib.rs:43:pub mod storage;
crates/imp-core/src/lib.rs:44:pub mod system_prompt;
crates/imp-core/src/lib.rs:45:pub mod tools;
crates/imp-core/src/lib.rs:46:pub mod trace;
crates/imp-core/src/lib.rs:47:pub mod trust;
crates/imp-core/src/lib.rs:48:pub mod ui;
crates/imp-core/src/lib.rs:49:pub mod usage;
crates/imp-core/src/lib.rs:50:pub mod workflow;
crates/imp-core/src/lib.rs:51:pub mod workflow_profiles;
crates/imp-lua/src/lib.rs:1:pub mod bridge;
crates/imp-lua/src/lib.rs:2:pub mod loader;
crates/imp-lua/src/lib.rs:3:pub mod sandbox;
crates/imp-lua/src/lib.rs:61:mod tests {
crates/imp-core/src/contracts.rs:186:pub mod worker {
crates/imp-core/src/contracts.rs:191:pub mod runner {}
crates/imp-core/src/contracts.rs:194:pub mod evidence {
crates/imp-core/src/contracts.rs:199:mod tests {
crates/imp-llm/src/lib.rs:7:pub mod auth;
crates/imp-llm/src/lib.rs:8:pub mod error;
crates/imp-llm/src/lib.rs:9:pub mod message;
crates/imp-llm/src/lib.rs:10:pub mod model;
crates/imp-llm/src/lib.rs:11:pub mod oauth;
crates/imp-llm/src/lib.rs:12:pub mod provider;
crates/imp-llm/src/lib.rs:13:pub mod providers;
crates/imp-llm/src/lib.rs:14:pub mod stream;
crates/imp-llm/src/lib.rs:15:pub mod text;
crates/imp-llm/src/lib.rs:16:pub mod usage;
crates/imp-core/src/builder.rs:465:mod tests {
crates/imp-gui/src/lib.rs:549:mod tests {
crates/imp-core/src/imp_session.rs:933:mod tests {
crates/imp-tui/src/lib.rs:1:pub mod animation;
crates/imp-tui/src/lib.rs:2:pub mod app;
crates/imp-tui/src/lib.rs:3:pub mod event_source;
crates/imp-tui/src/lib.rs:4:pub mod highlight;
crates/imp-tui/src/lib.rs:5:pub mod interactive;
crates/imp-tui/src/lib.rs:6:pub mod keybindings;
crates/imp-tui/src/lib.rs:7:pub mod markdown;
crates/imp-tui/src/lib.rs:8:mod repo_stats;
crates/imp-tui/src/lib.rs:9:pub mod selection;
crates/imp-tui/src/lib.rs:10:pub mod terminal;
crates/imp-tui/src/lib.rs:11:pub mod theme;
crates/imp-tui/src/lib.rs:12:pub mod tui_interface;
crates/imp-tui/src/lib.rs:13:pub mod turn_tracker;
crates/imp-tui/src/lib.rs:14:pub mod views;
crates/imp-cli/src/usage_report.rs:858:mod tests {
crates/imp-cli/src/lib.rs:124:pub(crate) mod acp;
crates/imp-cli/src/lib.rs:125:mod stats_report;
crates/imp-cli/src/lib.rs:126:mod usage_report;
crates/imp-cli/src/lib.rs:4212:mod tests {
crates/imp-core/src/eval_candidate_closeout.rs:249:mod tests {
crates/imp-core/src/learning.rs:39:mod tests {
crates/imp-core/src/context_prefill.rs:375:mod tests {
crates/imp-core/src/config.rs:1069:mod tests {
crates/imp-core/src/compaction.rs:510:mod tests {
crates/imp-core/src/policy.rs:209:mod tests {
crates/imp-cli/src/stats_report.rs:862:mod tests {
crates/imp-core/src/lib.rs:3:pub mod agent;
crates/imp-core/src/lib.rs:4:pub mod builder;
crates/imp-core/src/lib.rs:5:pub mod codeintel;
crates/imp-core/src/lib.rs:6:pub mod compaction;
crates/imp-core/src/lib.rs:7:pub mod config;
crates/imp-core/src/lib.rs:8:pub mod context;
crates/imp-core/src/lib.rs:9:pub mod context_prefill;
crates/imp-core/src/lib.rs:10:pub mod contracts;
crates/imp-core/src/lib.rs:11:pub mod error;
crates/imp-core/src/lib.rs:12:pub mod error_display;
crates/imp-core/src/lib.rs:13:pub mod eval_candidate;
crates/imp-core/src/lib.rs:14:pub mod eval_candidate_closeout;
crates/imp-core/src/lib.rs:15:pub mod evidence;
crates/imp-core/src/lib.rs:16:pub mod guardrails;
crates/imp-core/src/lib.rs:17:pub mod hooks;
crates/imp-core/src/lib.rs:18:pub mod imp_session;
crates/imp-core/src/lib.rs:19:pub mod import;
crates/imp-core/src/lib.rs:20:pub mod learning;
crates/imp-core/src/lib.rs:22:pub mod mana_next;
crates/imp-core/src/lib.rs:24:pub mod mana_prompt_context;
crates/imp-core/src/lib.rs:25:pub mod mana_review;
crates/imp-core/src/lib.rs:27:pub mod mana_run_state;
crates/imp-core/src/lib.rs:29:pub mod mana_worker;
crates/imp-core/src/lib.rs:30:pub mod memory;
crates/imp-core/src/lib.rs:31:pub mod personality;
crates/imp-core/src/lib.rs:32:pub mod policy;
crates/imp-core/src/lib.rs:33:pub mod reference_monitor;
crates/imp-core/src/lib.rs:34:pub mod repo_intelligence;
crates/imp-core/src/lib.rs:35:pub mod resources;
crates/imp-core/src/lib.rs:36:pub mod retry;
crates/imp-core/src/lib.rs:37:pub mod roles;
crates/imp-core/src/lib.rs:38:pub mod run_evidence;
crates/imp-core/src/lib.rs:39:pub mod runtime;
crates/imp-core/src/lib.rs:40:pub mod sdk;
crates/imp-core/src/lib.rs:41:pub mod session;
crates/imp-core/src/lib.rs:42:pub mod session_index;
crates/imp-core/src/lib.rs:43:pub mod storage;
crates/imp-core/src/lib.rs:44:pub mod system_prompt;
crates/imp-core/src/lib.rs:45:pub mod tools;
crates/imp-core/src/lib.rs:46:pub mod trace;
crates/imp-core/src/lib.rs:47:pub mod trust;
crates/imp-core/src/lib.rs:48:pub mod ui;
crates/imp-core/src/lib.rs:49:pub mod usage;
crates/imp-core/src/lib.rs:50:pub mod workflow;
crates/imp-core/src/lib.rs:51:pub mod workflow_profiles;
crates/imp-core/src/memory.rs:346:mod tests {
crates/imp-core/src/evidence.rs:282:mod tests {
crates/imp-core/src/reference_monitor.rs:1205:mod reference_monitor_types_tests {
crates/imp-cli/src/lib.rs:124:pub(crate) mod acp;
crates/imp-cli/src/lib.rs:125:mod stats_report;
crates/imp-cli/src/lib.rs:126:mod usage_report;
crates/imp-cli/src/lib.rs:4212:mod tests {
crates/imp-core/src/usage.rs:606:mod tests {
crates/imp-core/src/error_display.rs:120:mod tests {
crates/imp-core/src/guardrails.rs:431:mod tests {
crates/imp-core/src/session_index.rs:203:mod tests {
crates/imp-llm/src/model.rs:1126:mod tests {
crates/imp-core/src/context.rs:105:mod tests {
crates/imp-core/src/run_evidence.rs:403:mod tests {
crates/imp-core/src/mana_review.rs:1022:mod tests {
crates/imp-core/src/workflow_profiles.rs:414:mod tests {
crates/imp-core/src/import.rs:344:mod tests {
crates/imp-tui/src/markdown.rs:693:mod tests {
crates/imp-core/src/eval_candidate.rs:321:mod tests {
crates/imp-llm/src/provider.rs:233:mod transport_capability_tests {
crates/imp-core/src/retry.rs:174:mod tests {
crates/imp-core/src/trust.rs:370:mod tests {
crates/imp-core/src/resources.rs:286:mod tests {
crates/imp-llm/src/message.rs:119:mod tests {
crates/imp-core/src/personality.rs:807:mod tests {
crates/imp-core/src/hooks.rs:619:mod tests {
crates/imp-llm/src/lib.rs:7:pub mod auth;
crates/imp-llm/src/lib.rs:8:pub mod error;
crates/imp-llm/src/lib.rs:9:pub mod message;
crates/imp-llm/src/lib.rs:10:pub mod model;
crates/imp-llm/src/lib.rs:11:pub mod oauth;
crates/imp-llm/src/lib.rs:12:pub mod provider;
crates/imp-llm/src/lib.rs:13:pub mod providers;
crates/imp-llm/src/lib.rs:14:pub mod stream;
crates/imp-llm/src/lib.rs:15:pub mod text;
crates/imp-llm/src/lib.rs:16:pub mod usage;
crates/imp-lua/src/loader.rs:92:mod tests {
crates/imp-core/src/storage.rs:561:mod tests {
crates/imp-core/src/mana_prompt_context.rs:360:mod tests {
crates/imp-core/src/mana_worker.rs:982:mod tests {
crates/imp-lua/src/lib.rs:1:pub mod bridge;
crates/imp-lua/src/lib.rs:2:pub mod loader;
crates/imp-lua/src/lib.rs:3:pub mod sandbox;
crates/imp-lua/src/lib.rs:61:mod tests {
crates/imp-tui/src/lib.rs:1:pub mod animation;
crates/imp-tui/src/lib.rs:2:pub mod app;
crates/imp-tui/src/lib.rs:3:pub mod event_source;
crates/imp-tui/src/lib.rs:4:pub mod highlight;
crates/imp-tui/src/lib.rs:5:pub mod interactive;
crates/imp-tui/src/lib.rs:6:pub mod keybindings;
crates/imp-tui/src/lib.rs:7:pub mod markdown;
crates/imp-tui/src/lib.rs:8:mod repo_stats;
crates/imp-tui/src/lib.rs:9:pub mod selection;
crates/imp-tui/src/lib.rs:10:pub mod terminal;
crates/imp-tui/src/lib.rs:11:pub mod theme;
crates/imp-tui/src/lib.rs:12:pub mod tui_interface;
crates/imp-tui/src/lib.rs:13:pub mod turn_tracker;
crates/imp-tui/src/lib.rs:14:pub mod views;
crates/imp-llm/src/text.rs:32:mod tests {
crates/imp-core/src/system_prompt.rs:598:mod tests {
crates/imp-tui/src/animation.rs:182:mod tests {
crates/imp-core/src/roles.rs:755:mod tests {
crates/imp-tui/src/turn_tracker.rs:168:mod tests {
crates/imp-core/src/repo_intelligence.rs:347:mod tests {
crates/imp-core/src/trace.rs:264:mod tests {
crates/imp-tui/src/keybindings.rs:147:mod tests {
crates/imp-tui/src/interactive.rs:80:mod tests {
crates/imp-tui/src/event_source.rs:64:mod tests {
crates/imp-tui/src/repo_stats.rs:144:mod tests {
crates/imp-llm/src/auth.rs:800:mod tests {
crates/imp-llm/src/usage.rs:77:mod tests {
crates/imp-core/src/runtime.rs:725:mod runtime_events {
crates/imp-tui/src/terminal.rs:138:mod tests {
crates/imp-tui/src/theme.rs:226:mod tests {
crates/imp-tui/src/selection.rs:275:mod tests {
crates/imp-gui/src/lib.rs:549:mod tests {
crates/imp-core/src/session.rs:1915:mod tests {
crates/imp-core/src/session.rs:2673:mod recovery_ledger_tests {
crates/imp-tui/src/app.rs:9014:mod session_lifecycle {
