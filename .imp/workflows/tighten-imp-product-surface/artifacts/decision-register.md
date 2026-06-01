# Decision Register

Status: draft decisions to review before implementation.

This register turns the audit into explicit decisions. Items marked **recommended** are the default direction unless user changes them.

## D1 — Retained TUI slash commands

Recommendation: keep only:

```text
/new
/resume
/model
/compact
/quit
/loop
/stop
/reload
/setup
/secrets
/login
/name
/tree
/settings
```

Open sub-decisions:

- `/fork`: **remove**
- `/copy`: **remove**
- `/status`: **remove**; use visible TUI state instead of a command

Impact:

- Update `crates/imp-tui/src/views/command_palette.rs`.
- Update `crates/imp-tui/src/app.rs` command dispatch/help.
- Update tests that assert old commands exist.

## D2 — Future workflow aliases

Recommendation:

- Do not add `/work` in this workflow.
- Defer `/workflow` and `/workflows` TUI alias decisions until after `imp workflow run <id>` exists.

Rationale:

- Avoid replacing command sprawl with another half-designed surface.
- CLI workflow runner is the explicit orchestration boundary first.

Impact:

- Remove workflow profile slash commands from default discovery.
- Keep workflow file/tool internals.

## D3 — Minimal CLI surface

Recommendation: launch CLI surface should be:

```sh
imp
imp -p "..."
imp workflow run <id>
```

RPC:

- **keep for launch as an advanced/embedding surface**.
- Existing implementation already supports `--mode rpc` with JSONL input/output and runtime event/state conversion; launch work is likely small if limited to verification, docs tightening, and avoiding new command families.

Remove:

- `imp chat`
- `--mode chat`
- CLI chat shell grammar and slash compatibility

Impact:

- Add minimal workflow CLI.
- Remove chat shell parser/handler/tests later.
- Preserve one-shot and optional RPC.

## D4 — Prompt builder direction

Recommendation: implement simple prompt config, no presets.

Target default system line:

```text
You are imp, a practical and helpful software engineer. Use available tools, skills, and workflows when they help. Read files before editing them.
```

Target config:

```toml
[prompt]
system = "You are imp, a practical and helpful software engineer. Use available tools, skills, and workflows when they help. Read files before editing them."
tools = true
skills = true
project_instructions = true
environment = true
append = ["~/.imp/prompt.md", ".imp/prompt.md"]
```

Avoid knobs for:

- memory
- personality
- project facts
- guardrails
- mode instructions
- workflow doctrine

Rationale:

- If those old layers become config knobs, bloat survives as config surface.

Impact:

- Add `PromptConfig`.
- Refactor `system_prompt.rs`.
- Update prompt tests.

## D5 — Personality/soul

Recommendation: **remove** as product/backend concept.

Replacement:

```text
~/.imp/prompt.md
.imp/prompt.md
```

Impact:

- Remove `/personality`.
- Remove CLI personality.
- Remove TUI personality view.
- Remove `soul.md` discovery/product concept.
- Archive soul/personality docs.

Dependency:

- Implement prompt appendices first.

## D6 — Memory

Recommendation: **remove from product/default prompt**.

Specifics:

- Remove `/memory`.
- Stop default memory/user profile prompt injection.
- Do not list memory as default tool/product feature.
- Remove memory backend entirely unless a narrow compile-time dependency requires a temporary compatibility shim during the staged cut.

Impact:

- TUI tests around `/memory` removed/rewritten.
- Prompt builder tests updated.
- Docs/tools README updated.

## D7 — Mana / imp-work

Recommendation: **remove visible surface, fold durable value into workflows, archive docs**.

Specifics:

- Remove `/mana`, `/scope`, `mana-scope` product commands.
- Remove mana settings from default settings UI.
- Remove mana facts/status from default prompt.
- Keep feature-gated backend temporarily only if workflow runner/controller still needs it.

Impact:

- Large, staged cut.
- Do after visible command/prompt cuts and after workflow runner seam exists.

## D8 — Improve mode

Recommendation: **remove command mode, fold useful sandboxing into workflow runner later**.

Specifics:

- Remove `/improve*` commands.
- Remove Improve UI state/status/settings.
- Preserve idea of sandbox worktree/changelog/merge as future workflow execution policy if still desired.

Impact:

- High TUI `app.rs` cleanup.
- Many tests to update/remove.

## D9 — Eval candidates

Recommendation: **remove TUI product surface; keep CLI internal only if useful until workflow evidence replacement exists**.

Specifics:

- Remove `/eval`.
- Fold useful regression/failure capture into workflow artifacts/evidence.

Impact:

- TUI command/test cleanup first.
- Later remove `eval_candidate*` modules if unused.

## D10 — Prototype tool

Recommendation: **fold into workflow artifacts/evidence and remove standalone tool/product framing**.

Specifics:

- Stop advertising prototype as workflow product primitive.
- Migrate schema/docs from `prototypes` to generic artifacts/evidence only if implementation scope permits.
- Remove standalone tool when no longer needed.

Impact:

- Workflow docs/schema/tests may change.

## D11 — Workflow controller

Recommendation: **defunct ambient controller for normal TUI/one-shot; move strictness into explicit workflow runner**.

Specifics:

- Normal chat/TUI should not get hidden workflow continuation prompts.
- `imp workflow run <id>` owns strict step/check execution.
- Subagents become workflow-runner children.

Impact:

- High runtime change.
- Do after workflow runner seam exists.

## D12 — imp-gui

Recommendation: **remove from default members first**.

Specifics:

- Keep crate as workspace member initially.
- Decide archive/delete later.

Impact:

- Low-risk first implementation slice.

## D13 — Docs/archive

Recommendation:

- Use `~/imp-archive` for root clutter and old experiment/design docs.
- Keep active repo docs launch-focused.

Archive categories:

- GUI prototypes/wireframes
- old rebuild plans
- mana-next/imp-work docs
- personality/soul docs
- eval/prototype docs if surfaces removed
- future ACP/MCP/sync docs unless in roadmap

Impact:

- Rewrite README before broad archive move to avoid broken links.

## D14 — RPC and ACP

Recommendation:

- RPC: keep if low-cost, advanced/internal docs only.
- ACP: future/optional; do not implement now.

Impact:

- Runtime event/state stays.
- Remove ACP from launch-facing README claims.

## D15 — Implementation order

Recommended first implementation batch:

1. Remove `imp-gui` from default members.
2. Tighten TUI command palette/discovery.
3. Add minimal prompt config/one-line prompt appendix support.
4. Add `imp workflow run <id>`.
5. Remove CLI chat.
6. Remove personality UI/backend.

Do not start with mana/controller deletion; those need workflow runner seams first.

## Questions for user

Resolved on review:

1. `/fork`: remove.
2. `/copy`: remove.
3. `/status`: remove; rely on visible TUI state.
4. Memory: remove, including backend unless temporarily needed for staged compilation.
5. Archive destination: `~/imp-archive` for old docs/root clutter.
6. RPC: keep for launch as advanced/embedding surface.

Still open:

1. CLI eval: remove now, or keep hidden/internal until workflow evidence replacement exists?
2. RPC docs: README advanced mention or `docs/rpc.md` only?
