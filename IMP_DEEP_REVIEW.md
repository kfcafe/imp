# imp Deep Review

Date: 2026-03-31

Scope:
- Systematic review of the Rust source tree under `crates/`
- Static scans for enforcement gaps, dead features, panic/unsafe hotspots, and oversized modules
- Targeted close reading of the main runtime, tooling, auth, Lua, provider, CLI, and TUI control paths

Verification performed:
- `cargo check -p imp-cli -p imp-core -p imp-llm -p imp-lua -p imp-tui` passed
- `cargo test -p imp-core --lib` failed 1 test: `session::tests::generic_summary_title_falls_back_to_more_descriptive_phrase`
- `cargo test -p imp-llm --lib` failed 12 OAuth tests in this sandbox because the tests bind loopback listeners and received `PermissionDenied`; I treated that as an environment portability signal, not a conclusive product-runtime bug

## Findings

### F1. Lua extensions bypass core policy boundaries
Severity: High

Why it matters:
- The project presents agent mode, tool filtering, environment scoping, and shell/network boundaries as first-class controls.
- The Lua host API currently lets extensions route around those controls.

Evidence:
- `crates/imp-lua/src/bridge.rs:221` exposes `imp.exec(...)`, which runs `sh -c` directly.
- `crates/imp-lua/src/bridge.rs:324` exposes `imp.tool(...)`, which executes native tools from the runtime-held map.
- `crates/imp-lua/src/bridge.rs:398` exposes `imp.env(...)`; the default policy is allow-all unless an allowlist is explicitly populated.
- `crates/imp-lua/src/bridge.rs:415` exposes unrestricted HTTP GET/POST.
- `crates/imp-lua/src/sandbox.rs:141` stores the native tool map once.
- `crates/imp-lua/src/lib.rs:50` populates that map before the builder later filters the visible registry by mode.
- `crates/imp-core/src/builder.rs:163` filters the registry after Lua loading, but that does not retract the already-captured native tool handles.

Impact:
- A Lua extension loaded in `reviewer`, `planner`, or `orchestrator` mode can still invoke mutating native tools through `imp.tool(...)`.
- Lua can also bypass tool-mode constraints entirely with `imp.exec(...)`.
- `imp.env(...)` defaults to full environment visibility, which exposes secrets unless another layer sets an allowlist.
- `imp.http` provides a second network surface separate from the curated `web` tool.

Recommendation:
- Make Lua host capabilities explicit and deny-by-default.
- Bind the Lua-visible native tool map after mode filtering, not before.
- Gate `imp.exec`, `imp.tool`, `imp.env`, and `imp.http` through the same mode/policy system as native tools.
- Default environment access to empty and require per-extension allowlists.

### F2. Cancellation is not propagated into active tool execution
Severity: High

Why it matters:
- A terminal agent needs cancellation to stop real work, not just stop waiting for it.
- Today, `Cancel` stops the agent loop while the active tool may continue running until timeout or natural exit.

Evidence:
- `crates/imp-core/src/agent.rs:227` handles `AgentCommand::Cancel` only at the agent loop level.
- `crates/imp-core/src/agent.rs:675` creates a fresh `AtomicBool(false)` for each tool invocation instead of sharing a real cancellation token.
- `crates/imp-core/src/tools/bash.rs:132` checks cancellation only before spawning.
- `crates/imp-core/src/tools/bash.rs:223` then waits on the child process / timeout without re-checking the token.

Impact:
- Long-running shell commands are not cancelled when the user cancels the agent.
- Other tools that rely on `ctx.is_cancelled()` have the same problem because the token is never driven by agent cancellation.

Recommendation:
- Move to a shared per-tool cancellation token wired to `AgentCommand::Cancel`.
- Ensure long-running tools poll cancellation while running and terminate subprocess trees immediately.

### F3. Diff tool wiring does not match the mode allowlists
Severity: High

Why it matters:
- The mode model is a core product contract.
- The code currently advertises one permission shape and wires another.

Evidence:
- `crates/imp-core/src/config.rs:35` allowlists `diff_show` and `diff_apply`.
- `crates/imp-core/src/tools/diff.rs:13` defines the registered tool name as `diff`.
- `crates/imp-core/src/builder.rs:150` registers `DiffTool`.
- `crates/imp-core/src/builder.rs:165` filters tools by exact registered name.

Impact:
- Non-`full` modes do not actually get the intended diff capability.
- Reviewer/planner/orchestrator behavior can diverge from both docs and tests depending on which name the model tries to call.

Recommendation:
- Register `diff_show` and `diff_apply` explicitly, or change the mode allowlists to match the real registered tool names.
- Add an end-to-end test that asserts the visible tool names per mode, not just allowlist logic in isolation.

### F4. Shell backend is a user-visible setting that runtime execution ignores
Severity: Medium

Why it matters:
- A configuration control that does not affect runtime is a trust problem.
- It creates support/debug confusion because the UI implies the shell backend is selectable.

Evidence:
- `crates/imp-tui/src/views/settings.rs:138` loads `config.shell.backend`.
- `crates/imp-tui/src/views/settings.rs:213` and `crates/imp-tui/src/views/settings.rs:337` let the user cycle it.
- `crates/imp-core/src/tools/bash.rs:17` and `crates/imp-core/src/tools/bash.rs:63` decide shell behavior from environment variables and PATH probing, not `Config::shell.backend`.
- `crates/imp-core/src/builder.rs` never passes shell backend config into the bash tool path.

Impact:
- The setting appears to work in the TUI but does not govern actual command execution.

Recommendation:
- Thread shell backend choice from config into `ToolContext` or an explicit shell-execution config object.
- Remove the setting until it is authoritative if you do not want to support it yet.

### F5. Auth store persistence is brittle
Severity: Medium

Why it matters:
- Credential storage should fail loudly and write safely.
- Silent fallback and non-atomic writes are a poor fit for auth state.

Evidence:
- `crates/imp-llm/src/auth.rs:100` loads the auth file.
- `crates/imp-llm/src/auth.rs:103` uses `serde_json::from_str(&data).unwrap_or_default()`, which silently drops malformed credential data and returns an empty store.
- `crates/imp-llm/src/auth.rs:300` writes directly to the target file via `std::fs::write(...)` without a temp-file-and-rename flow.

Impact:
- A corrupted `auth.json` is silently treated as “no credentials,” which is hard to diagnose.
- Interrupted writes can leave the auth file truncated or partially written.

Recommendation:
- Fail with a surfaced error when persisted auth cannot be parsed.
- Write auth files atomically with `write temp -> fsync -> rename`.

### F6. File cache and rollback history are implemented but not integrated into the live tool path
Severity: Medium

Why it matters:
- These mechanisms increase code surface and mental load, but currently do not buy much production value.

Evidence:
- `crates/imp-core/src/tools/mod.rs:141` defines `FileCache`.
- `crates/imp-core/src/tools/mod.rs:201` defines `FileHistory`.
- `crates/imp-core/src/tools/read.rs:77` reads files directly with `tokio::fs::read(...)` instead of using `ctx.file_cache`.
- `crates/imp-core/src/tools/mod.rs:194` defines `invalidate(...)`, but no production path calls it.
- `crates/imp-core/src/tools/mod.rs:225` defines snapshot/rollback behavior, but only tests reference it.

Impact:
- The runtime carries partially implemented safety/performance concepts that are not actually enforcing or accelerating anything.
- Future contributors may assume these protections are live when they are not.

Recommendation:
- Either wire file cache/history into the actual read/write/edit flows or remove them until there is a complete design.

### F7. Buffered tool output can be dropped at the end of execution
Severity: Medium-low

Why it matters:
- Streaming output is part of the product quality of a terminal agent.
- Losing the tail of tool output is subtle and hard to reproduce.

Evidence:
- `crates/imp-core/src/agent.rs:689` spawns a forwarder that drains `update_rx`.
- `crates/imp-core/src/agent.rs:704` awaits the tool execution.
- `crates/imp-core/src/agent.rs:709` aborts the forwarder immediately after the tool returns.

Impact:
- Any updates still buffered in the channel can be lost instead of being forwarded to the UI/event stream.

Recommendation:
- Drop the sender, then await the forwarder to drain remaining updates before returning the final tool result.

### F8. Session title heuristics have drifted from their own tests
Severity: Low

Why it matters:
- The title-generation path is not critical, but it is a visible UX detail and a good proxy for whether heuristics are kept disciplined.

Evidence:
- `crates/imp-core/src/session.rs:1322` defines `fallback_phrase_title(...)`.
- The workspace build reports it as dead code.
- `crates/imp-core/src/session.rs:1653` has a failing test asserting title compactness for a generic summary phrase.

Impact:
- The heuristics are drifting, and the code contains unused fallback logic while a nearby expectation is already broken.

Recommendation:
- Reduce heuristic branches, decide on one title-generation strategy, and keep tests aligned with it.

### F9. Provider-auth resolution logic is duplicated across runtime surfaces
Severity: Low

Why it matters:
- CLI, TUI, and session runtime should not each own near-identical provider-resolution policy.
- Duplication creates silent divergence risk.

Evidence:
- `crates/imp-cli/src/main.rs:934` defines `resolve_provider_api_key(...)`.
- `crates/imp-core/src/imp_session.rs:593` defines a separate `resolve_api_key(...)`.
- `crates/imp-tui/src/app.rs:250` defines another `resolve_provider_api_key(...)`.

Impact:
- Provider-selection and auth-refresh policy changes must be kept in sync manually across three places.
- The current special-casing already makes the control flow harder to reason about.

Recommendation:
- Move provider auth resolution into one shared helper in `imp-core` or `imp-llm`.

### F10. Several key modules are too large for safe change velocity
Severity: Low

Why it matters:
- Very large files slow review, obscure ownership, and increase regression risk when features interact.

Largest examples reviewed in this pass:
- `crates/imp-tui/src/app.rs` — 5145 lines
- `crates/imp-core/src/agent.rs` — 3578 lines
- `crates/imp-cli/src/main.rs` — 2915 lines
- `crates/imp-llm/src/providers/anthropic.rs` — 2451 lines
- `crates/imp-core/src/session.rs` — 2026 lines
- `crates/imp-core/src/tools/mana.rs` — 1804 lines
- `crates/imp-tui/src/views/settings.rs` — 1301 lines

Impact:
- These files already combine multiple responsibilities: state machines, persistence, UI behavior, auth/provider routing, and command handling.
- The review surface is much larger than it needs to be for routine changes.

Recommendation:
- Split by responsibility, not arbitrary size:
- agent stream processing vs tool execution vs cancellation
- app state vs event handling vs modal/view orchestration
- session persistence vs title/summary heuristics
- provider request building vs SSE parsing vs model catalogues

## Verification Notes

- `imp-core` library tests are not fully green on the current branch because of the session-title failure described in F8.
- `imp-llm` library tests were not fully green in this sandbox because OAuth tests assume loopback listener binding:
- `crates/imp-llm/src/oauth/anthropic.rs:408`
- `crates/imp-llm/src/oauth/anthropic.rs:489`
- `crates/imp-llm/src/oauth/chatgpt.rs:455`
- `crates/imp-llm/src/oauth/chatgpt.rs:488`
- That is at least a portability/testing concern even if it is not a product-runtime defect on normal developer machines.

## File Coverage Appendix

Legend:
- `F#` = finding above
- `None` = reviewed in this pass, no distinct issue called out beyond general maintenance risk

### `imp-cli`

| File | Notes |
| --- | --- |
| `crates/imp-cli/src/main.rs` | F4, F9, F10 |
| `crates/imp-cli/src/usage_report.rs` | None |

### `imp-core`

| File | Notes |
| --- | --- |
| `crates/imp-core/benches/core_hot_paths.rs` | None |
| `crates/imp-core/benches/grep_vs_probe.rs` | None |
| `crates/imp-core/examples/reuse-bench.rs` | None |
| `crates/imp-core/src/agent.rs` | F2, F7, F10 |
| `crates/imp-core/src/builder.rs` | F1, F3, F4 |
| `crates/imp-core/src/config.rs` | F3, F4 |
| `crates/imp-core/src/context.rs` | None |
| `crates/imp-core/src/error.rs` | None |
| `crates/imp-core/src/guardrails.rs` | None |
| `crates/imp-core/src/hooks.rs` | None |
| `crates/imp-core/src/imp_session.rs` | F9 |
| `crates/imp-core/src/import.rs` | None |
| `crates/imp-core/src/learning.rs` | None |
| `crates/imp-core/src/lib.rs` | None |
| `crates/imp-core/src/memory.rs` | None |
| `crates/imp-core/src/personality.rs` | None |
| `crates/imp-core/src/resources.rs` | None |
| `crates/imp-core/src/retry.rs` | None |
| `crates/imp-core/src/roles.rs` | None |
| `crates/imp-core/src/session.rs` | F8, F10 |
| `crates/imp-core/src/session_index.rs` | None |
| `crates/imp-core/src/system_prompt.rs` | None |
| `crates/imp-core/src/tools/ask.rs` | None |
| `crates/imp-core/src/tools/bash.rs` | F2, F4 |
| `crates/imp-core/src/tools/diff.rs` | F3 |
| `crates/imp-core/src/tools/edit.rs` | None |
| `crates/imp-core/src/tools/extend.rs` | None |
| `crates/imp-core/src/tools/find.rs` | None |
| `crates/imp-core/src/tools/grep.rs` | None |
| `crates/imp-core/src/tools/ls.rs` | None |
| `crates/imp-core/src/tools/lua.rs` | F1 |
| `crates/imp-core/src/tools/mana.rs` | F10 |
| `crates/imp-core/src/tools/memory.rs` | None |
| `crates/imp-core/src/tools/mod.rs` | F6 |
| `crates/imp-core/src/tools/multi_edit.rs` | None |
| `crates/imp-core/src/tools/query.rs` | None |
| `crates/imp-core/src/tools/read.rs` | F6 |
| `crates/imp-core/src/tools/scan/go.rs` | None |
| `crates/imp-core/src/tools/scan/mod.rs` | None |
| `crates/imp-core/src/tools/scan/python.rs` | None |
| `crates/imp-core/src/tools/scan/rust.rs` | None |
| `crates/imp-core/src/tools/scan/types.rs` | None |
| `crates/imp-core/src/tools/scan/typescript.rs` | None |
| `crates/imp-core/src/tools/session_search.rs` | None |
| `crates/imp-core/src/tools/shell.rs` | None |
| `crates/imp-core/src/tools/web/mod.rs` | None |
| `crates/imp-core/src/tools/web/read.rs` | None |
| `crates/imp-core/src/tools/web/search.rs` | None |
| `crates/imp-core/src/tools/web/types.rs` | None |
| `crates/imp-core/src/tools/write.rs` | None |
| `crates/imp-core/src/ui.rs` | None |
| `crates/imp-core/src/usage.rs` | None |

### `imp-llm`

| File | Notes |
| --- | --- |
| `crates/imp-llm/src/auth.rs` | F5, F9 |
| `crates/imp-llm/src/error.rs` | None |
| `crates/imp-llm/src/lib.rs` | None |
| `crates/imp-llm/src/message.rs` | None |
| `crates/imp-llm/src/model.rs` | None |
| `crates/imp-llm/src/oauth/anthropic.rs` | Verification portability note only |
| `crates/imp-llm/src/oauth/chatgpt.rs` | Verification portability note only |
| `crates/imp-llm/src/oauth/mod.rs` | None |
| `crates/imp-llm/src/oauth/pkce.rs` | None |
| `crates/imp-llm/src/provider.rs` | None |
| `crates/imp-llm/src/providers/anthropic.rs` | F10 |
| `crates/imp-llm/src/providers/google.rs` | None |
| `crates/imp-llm/src/providers/mod.rs` | None |
| `crates/imp-llm/src/providers/openai.rs` | None |
| `crates/imp-llm/src/providers/openai_codex.rs` | None |
| `crates/imp-llm/src/providers/openai_compat.rs` | None |
| `crates/imp-llm/src/stream.rs` | None |
| `crates/imp-llm/src/text.rs` | None |
| `crates/imp-llm/src/usage.rs` | None |

### `imp-lua`

| File | Notes |
| --- | --- |
| `crates/imp-lua/src/bridge.rs` | F1 |
| `crates/imp-lua/src/lib.rs` | F1 |
| `crates/imp-lua/src/loader.rs` | None |
| `crates/imp-lua/src/sandbox.rs` | F1 |

### `imp-tui`

| File | Notes |
| --- | --- |
| `crates/imp-tui/src/animation.rs` | None |
| `crates/imp-tui/src/app.rs` | F4, F9, F10 |
| `crates/imp-tui/src/highlight.rs` | None |
| `crates/imp-tui/src/interactive.rs` | None |
| `crates/imp-tui/src/keybindings.rs` | None |
| `crates/imp-tui/src/lib.rs` | None |
| `crates/imp-tui/src/markdown.rs` | None |
| `crates/imp-tui/src/selection.rs` | None |
| `crates/imp-tui/src/terminal.rs` | None |
| `crates/imp-tui/src/theme.rs` | None |
| `crates/imp-tui/src/tui_interface.rs` | None |
| `crates/imp-tui/src/turn_tracker.rs` | None |
| `crates/imp-tui/src/views/ask_bar.rs` | None |
| `crates/imp-tui/src/views/chat.rs` | None |
| `crates/imp-tui/src/views/command_palette.rs` | None |
| `crates/imp-tui/src/views/editor.rs` | None |
| `crates/imp-tui/src/views/file_finder.rs` | None |
| `crates/imp-tui/src/views/login_picker.rs` | None |
| `crates/imp-tui/src/views/mod.rs` | None |
| `crates/imp-tui/src/views/model_selector.rs` | None |
| `crates/imp-tui/src/views/personality.rs` | None |
| `crates/imp-tui/src/views/session_picker.rs` | None |
| `crates/imp-tui/src/views/settings.rs` | F4, F10 |
| `crates/imp-tui/src/views/sidebar.rs` | F10 |
| `crates/imp-tui/src/views/status.rs` | None |
| `crates/imp-tui/src/views/tool_output.rs` | None |
| `crates/imp-tui/src/views/tools.rs` | None |
| `crates/imp-tui/src/views/top_bar.rs` | None |
| `crates/imp-tui/src/views/tree.rs` | None |
| `crates/imp-tui/src/views/welcome.rs` | None |

## Suggestions To Further Improve The Project

- Unify policy enforcement around one capability model. Right now agent mode, visible tools, Lua host API, shell execution, and network access are related but not truly centralized.
- Treat cancellation as a cross-cutting runtime primitive. Shell, web, mana runs, Lua tool invocations, and provider calls should all share the same cancellation semantics.
- Remove or finish half-built features. `FileCache`, `FileHistory`, and shell backend selection all increase complexity faster than they currently increase value.
- Reduce duplication in provider/auth resolution and model-selection policy. One canonical path should serve CLI, TUI, and headless session code.
- Split the largest modules before adding more feature surface. `app.rs`, `agent.rs`, `main.rs`, `session.rs`, and `mana.rs` are already carrying too many responsibilities.
- Add end-to-end tests for user-facing contracts rather than only unit tests for helper logic: mode-visible tools, cancellation behavior, shell backend selection, Lua restriction enforcement, and session-title generation.
- Make persisted auth/session writes atomic and fail-loud. This project is increasingly stateful; silent fallback on corrupted state will get more expensive over time.
- Add a documented trust model for extensions. If Lua is intentionally trusted and allowed to escape policy, state that plainly. If it is meant to respect agent constraints, the runtime needs real enforcement.
