# Full Codebase Audit for Tightening imp

Status: draft full-codebase audit for `tighten-imp-product-surface`.

This audit focuses on product/code bloat, removable experimental surfaces, prompt/runtime rigidity, and places where workflows can replace earlier feature experiments.

## Scope inspected

Commands run/inspected:

- `tokei . --exclude target --exclude .git --exclude .imp/runs --exclude .mana --exclude '*.html' --exclude Cargo.lock`
- `find crates -type f \( -name '*.rs' -o -name 'Cargo.toml' \) -print0 | xargs -0 wc -l | sort -nr | head -60`
- `cargo metadata --no-deps --format-version=1`
- module/file inventories under `crates/imp-core`, `crates/imp-tui`, `crates/imp-cli`, `crates/imp-lua`, `crates/imp-llm`, `crates/imp-gui`
- searches for slash commands, CLI commands, tools, compatibility/legacy/planned/prototype/personality/mana/eval references
- prompt/runtime prompt audit in `prompt-audit.md`

Important limitation: this is a static audit plus narrow targeted tests from the rigidity fix. It is not a full compile/test of the whole workspace.

## Size and shape

`tokei` excluding generated/heavy local state reports:

```text
Rust:       183 files, 136,267 lines, 122,706 code
Markdown:    98 files, 24,596 lines
Total:      330 files, 169,985 lines, 129,025 code
```

Large Rust hotspots:

```text
13,239 crates/imp-tui/src/app.rs
 6,566 crates/imp-cli/src/lib.rs
 6,071 crates/imp-core/src/agent/mod.rs
 5,561 crates/imp-core/src/tools/mana.rs
 2,684 crates/imp-core/src/tools/scan/mod.rs
 2,543 crates/imp-core/src/session.rs
 2,464 crates/imp-llm/src/providers/anthropic.rs
 2,081 crates/imp-tui/src/views/settings.rs
 1,976 crates/imp-tui/src/views/sidebar.rs
 1,897 crates/imp-core/src/reference_monitor.rs
 1,846 crates/imp-core/src/config.rs
 1,803 crates/imp-core/src/system_prompt.rs
 1,312 crates/imp-core/src/tools/workflow.rs
 1,312 crates/imp-core/src/mana_worker.rs
   977 crates/imp-core/src/personality.rs
   958 crates/imp-core/src/tools/prototype.rs
   951 crates/imp-core/src/agent/workflow_integration/mana_compat.rs
   949 crates/imp-core/src/workflow/controller.rs
   874 crates/imp-core/src/typescript_extensions/mod.rs
```

Local state / repo clutter:

```text
.imp:     413M
.mana:    6.1M
docs:     936K
crates:   4.9M
target:   46G
```

Docs count:

```text
73 markdown docs under docs/ at max depth 2
```

## Workspace/crate surface

Workspace packages from `cargo metadata`:

- `imp-cli`
- `imp-core`
- `imp-gui`
- `imp-install` root package
- `imp-llm`
- `imp-lua`
- `imp-tui`

Root `Cargo.toml` includes `crates/imp-gui` in both `members` and `default-members`.

Recommendation:

- remove `imp-gui` from `default-members` first.
- later decide whether to move `imp-gui` to archive/experiment or keep as non-default workspace member.

## Product surfaces

### Keep first-class

- TUI
- one-shot prompt mode
- workflows
- Lua extensions for shipped extensibility
- provider/auth/config/runtime basics
- sessions, especially `/resume`

### Candidate first-class retained TUI commands

From user direction and prior inventory:

- `/new`
- `/resume`
- `/model`
- `/compact`
- `/quit`
- `/loop`
- `/reload`
- `/setup`
- `/secrets`
- `/login`
- `/name`
- `/tree`
- `/settings`
- `/stop`

Open:

- `/fork` — likely useful but still a product call.
- `/copy` — useful affordance but not yet user-confirmed.

Likely remove:

- `/personality` and backend.

### Slash command bloat

Current built-ins include many top-level commands that encode old experiments:

- improve/improve-safe/improve-merge/improve-help
- eval
- status
- autonomy
- clean
- queue
- run
- scope
- mana
- fork
- copy
- export
- personality
- memory
- checkpoints
- restore-checkpoint
- hotkeys

Additional routes also exist for:

- `/plan`
- `/workflow`
- `/workflows`
- workflow-profile slash commands such as plan/review/verify/debug/research/implement
- Lua extension commands
- skill commands

Findings:

- task-type shortcuts like `/plan`, `/run`, `/debug`, `/review`, `/verify` are product bloat; workflow/natural language should absorb them.
- `/work`, `/workflow`, `/workflows` can eventually become aliases, but adding `/work` is explicitly out of scope for this workflow.
- current prefix matching risks surprising command execution because large command sets and extension/skill/workflow commands all participate.

### CLI surface bloat

`crates/imp-cli/src/lib.rs` is 6.5k lines and includes:

- TUI launch
- one-shot print mode
- JSON/RPC mode
- interactive CLI chat shell with `:` grammar and slash compatibility
- view/settings/personality/setup/login/secrets commands
- stats/usage/evidence/eval/import/install-local
- mana namespace behind feature
- personality editor
- many tests

Findings:

- TUI and one-shot are product-critical.
- CLI chat shell is not product-critical and creates a second command grammar.
- eval/evidence/stats/usage/import/install-local may be useful dev/internal tools, but should not define product identity.
- personality CLI path becomes removable if personality is removed.

Recommendation:

- remove or archive CLI chat shell after confirming one-shot path covers real needs.
- keep `imp` TUI and `imp -p`/print one-shot.
- decide RPC separately: it may matter for product embedding, but it is not part of the minimal user-facing CLI.

## Model-facing tool surface

Canonical native tools registered in `register_native_tools()`:

- ask
- bash
- edit
- git
- read
- write
- scan
- web
- workflow

This core is tight and should stay.

Non-canonical/extra modules still present:

- `tools/mana.rs` behind `mana-tool`
- `tools/memory.rs`
- `tools/prototype.rs`
- `tools/shell.rs` for configured shell tools
- `tools/multi_edit.rs` used internally by edit when an edits array is present
- `tools/lua.rs` / Lua loader
- `tools/code_intel.rs`, `tools/query.rs`

Findings:

- native model-facing default tools are not the main bloat problem.
- old tool modules can still bloat codebase and prompt/runtime concepts even if not registered by default.
- `prototype` is a strong candidate to remove/fold into workflow artifacts.
- `memory` should be reconsidered; memory/user profile can be a prompt config layer, not a slash command/tool product concept.
- `mana` is a compatibility backend and oversized for target product.

## Durable-work overlap

Current code/docs still contain many durable-work eras:

- mana
- imp-work
- workflow
- workflow profiles
- run evidence
- prototype tool/schema entries
- eval candidates
- improve mode
- mana worker/run state
- mana review
- mana prompt context
- workflow controller with mana root/unit fields
- agent workflow integration with mana compatibility

Findings:

- workflows are the right durable primitive, but current workflow runtime is still partly mana-shaped.
- `workflow/controller.rs` references mana roots, units, run_state/logs, and child runs.
- `agent/workflow_integration/mana_compat.rs` injects imp-work/mana follow-up prompts.
- workflow schemas still include `prototypes` as a first-class field.

Recommendation:

- keep `crates/imp-core/src/workflow/` and `tools/workflow.rs`.
- migrate controller/integration language and state to workflow-native concepts.
- remove or isolate mana compatibility after workflow-native parity exists.
- consider removing standalone prototype tool and keeping only workflow artifacts/checks.

## Prompt/runtime prompt surface

Detailed audit is in `prompt-audit.md`.

High-level findings:

- system prompt is 1.8k lines of code and assembles many hard-coded layers.
- prompt assembly includes tools, AGENTS.md, skills, facts, project memory status, guardrails, task context, headless contract, memory/user profile, personality/soul, roles, mode instructions, learning guidance, and environment.
- only prompt override is all-or-nothing.
- hidden runtime follow-up prompts can force rigid continuation.
- workflow controller prompts are strict and currently mana-shaped.
- personality and operating rules duplicate each other.

Recommendation:

- add a first-class `PromptConfig` with presets/layer toggles.
- default product preset should be minimal.
- disable personality, memory/user profile, mana facts/status, and verbose mode instructions by default in minimal preset.
- expose workflow controller strictness config: likely `off|light|strict`.
- strict controller should be reserved for explicit workflow child runs, not normal TUI conversation.

## Feature/backends likely to cut or archive

### Highest-priority cuts

1. `imp-gui` from default members.
2. CLI chat shell or at least stop documenting/catering to it.
3. Personality command/backend.
4. Mana compatibility command/backend from default product path.
5. Improve mode commands/backend.
6. Eval candidate CLI/TUI surfaces unless retained as internal dev tooling.
7. Prototype tool/backend as standalone concept; fold useful evidence into workflows.
8. Memory slash command/tool if prompt config replaces it.
9. Checkpoints/restore command until restore is real.
10. Workflow profile slash commands as top-level commands.

### Potentially keep but de-emphasize

- stats/usage reporting: useful personal/internal diagnostics, but not core product surface.
- RPC/runtime JSON: useful for embedding; decide separately.
- reference monitor/guardrails: can help runtime quality but should be configurable and not prompt-heavy.
- roles/modes: likely useful for workflow workers, but too visible/verbose currently.
- TypeScript extension compatibility: future-facing but currently not shipped product; Lua is current shipped extension runtime.

## Documentation bloat

README currently presents:

- TUI/one-shot/RPC
- workflows
- prototypes
- policy
- Lua extensions
- planned MCP/agents/ACP/sync/workflow API
- compatibility/legacy mana

Docs contain many rebuild/proposal/mana-next/type-script-extension docs.

Findings:

- valuable design history is mixed with current product docs.
- planned/legacy content makes imp feel unfinished and bloated.

Recommendation:

- current docs should describe TUI + one-shot + workflows + Lua/config/auth.
- move old rebuild/proposal/mana-next/GUI docs to archive or clearly label as design history.
- README should not present planned features as product surface.

## Architecture hotspots

### `crates/imp-tui/src/app.rs`

13k lines and centralizes:

- app state
- event loop
- slash commands
- sessions
- auth/secrets/setup
- workflow/mana/improve/eval flows
- Lua command dispatch
- agent event handling
- tests

Recommendation:

- do not start with a huge decomposition.
- first cut command surface.
- then extract command registry/dispatch once target surface is clear.

### `crates/imp-cli/src/lib.rs`

6.5k lines and mixes multiple product modes.

Recommendation:

- after deciding CLI chat fate, split CLI into mode modules.
- deleting chat/personality/eval paths first may make extraction smaller.

### `crates/imp-core/src/agent/mod.rs`

6k lines plus split submodules, still contains prompt-follow-up and workflow integration orchestration.

Recommendation:

- isolate runtime follow-up prompt policy.
- move mana compatibility out of default workflow integration.

### `crates/imp-core/src/tools/mana.rs`

5.5k lines behind feature but large, transitional, and conceptually obsolete for target product.

Recommendation:

- archive or isolate further once workflow-native equivalent is sufficient.

### `crates/imp-core/src/system_prompt.rs`

1.8k lines and behaviorally central.

Recommendation:

- introduce prompt presets/layers before deleting too much; this will make future product choices safer and testable.

## Recommended first implementation sequence

1. Finish planning artifacts:
   - `codebase-audit.md`
   - `prompt-audit.md`
   - `backend-inventory.md`
   - `target-surface.md`
   - `archive-plan.md`
   - `backend-cut-sequence.md`
2. Make prompt/runtime less rigid:
   - keep current fix that avoids literal DONE headings.
   - add prompt configurability plan before big prompt cuts.
3. Remove `imp-gui` from default members.
4. Tighten command palette/dispatch to retained commands plus workflow aliases decision, without adding `/work` yet.
5. Remove personality command/backend if approved.
6. Remove or archive CLI chat shell.
7. Remove/improve/eval/prototype/memory/mana surfaces in that order, folding durable value into workflows.
8. Clean docs to current product identity.
9. Add `.imp/runs` retention/global storage policy.

## Risks

- Removing backend code before workflow-native replacement is clear could break hidden runtime paths.
- `mana-api` feature-gated tests may hide dependencies not exercised by default checks.
- prompt changes can alter agent behavior substantially; use targeted prompt snapshot tests.
- command removal in `app.rs` will likely require updating many TUI integration tests.
- deleting CLI chat may expose assumptions in docs/tests/import/setup code.

## Initial conclusion

The codebase bloat is not primarily the native tool set. It comes from accumulated product/control surfaces and old durable-work experiments living beside the new workflow direction.

The tight product should be:

```text
imp = TUI coding agent + one-shot prompt + workflows + Lua extensions
```

Everything else should become:

- workflow-internal behavior,
- explicit optional config,
- internal dev tooling,
- archived design history,
- or deleted.
