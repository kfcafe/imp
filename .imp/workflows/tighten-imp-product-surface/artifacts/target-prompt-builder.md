# Target Prompt Builder

Status: concrete prompt-builder plan for `tighten-imp-product-surface`.

## Goal

Make imp's default system prompt radically small while preserving strong behavior through tools, skills, project instructions, and explicit workflow-run prompts.

Target default identity:

```text
You are imp, a practical and helpful software engineer. Use available tools, skills, and workflows when they help. Read files before editing them.
```

## Current implementation summary

Evidence from:

- `crates/imp-core/src/system_prompt.rs`
- `crates/imp-core/src/builder.rs`
- `crates/imp-core/src/config.rs`
- `crates/imp-core/src/resources.rs`

Current `AssembleParams` includes:

- tools
- AGENTS.md resources
- skills
- mana facts
- project memory status
- personality profile
- soul doc
- task context
- role
- agent mode
- memory
- user profile
- cwd
- learning enabled
- guardrail profile

Current `AgentBuilder::build()` discovers/loads:

- AGENTS.md
- soul.md
- skills
- memory.md and user.md when learning is enabled
- mana prompt context/facts/project memory status
- personality profile from config
- guardrails
- task context
- role/mode data

Current `system_prompt.rs` hard-codes:

- identity/personality/soul logic
- full tool list
- tool routing rules
- operating rules
- learning instructions
- environment
- AGENTS.md
- skills
- facts
- project memory status
- guardrails layer
- task/headless execution contract
- memory/user profile blocks
- role instructions/output schema metadata
- mode instructions

## Target implementation shape

### Config

Prefer a simple config, not presets and not knobs for old internal concepts.

Target TOML:

```toml
[prompt]
system = "You are imp, a practical and helpful software engineer. Use available tools, skills, and workflows when they help. Read files before editing them."
tools = true
skills = true
project_instructions = true
environment = true
append = ["~/.imp/prompt.md", ".imp/prompt.md"]
```

Avoid:

```toml
preset = "minimal"
memory = false
personality = false
project_facts = false
guardrails = false
mode_instructions = "off"
workflow_doctrine = false
```

Reason: those preserve old bloat as configurable product concepts. If a concept is not part of the new default prompt, remove it or make it task-specific/internal.

### Prompt layers to keep

Keep only obvious layers:

1. system identity line
2. available tools
3. available skills
4. project instructions (`AGENTS.md`)
5. prompt appendices
6. environment basics

### Prompt layers to remove from default

Remove from normal default prompt assembly:

- personality profile identity
- soul.md discovery/injection
- personality working-style sliders
- mana facts
- project memory status
- guardrails prompt layer
- learning instructions
- memory.md block
- user.md profile block
- verbose AgentMode instructions
- workflow doctrine/closeout instructions
- role instructions except explicit worker/task contexts

### Task/workflow-specific prompts

Not all removed prompt text disappears forever.

Move strict/directive text to explicit scoped prompts:

- `imp workflow run <id>` step prompt
- workflow subagent prompt
- explicit headless task prompt
- explicit audit/review/eval internal command if kept

Global system prompt stays minimal.

## Prompt appendix behavior

Replace `soul.md` with generic appendices.

Default append paths:

```text
~/.imp/prompt.md
.imp/prompt.md
```

Behavior:

- Missing files are skipped silently.
- Existing files are appended after project instructions and before environment or at the end; choose one consistent order.
- File content is included verbatim with a small heading containing the path.
- Users who want a soul/personality can write it there manually.
- No sliders, no personality schema, no soul-specific parser.

Suggested order:

1. identity
2. tools
3. skills
4. project instructions
5. prompt appendices
6. environment

Reason: appendices can override/tune after project instructions, while environment remains factual metadata.

## Tool rendering

Keep simple name + description from registry:

```text
Tools:
- read: Read a file...
- edit: Edit files...
```

No compact/full modes initially.

If token pressure becomes real, solve later with shorter tool descriptions in registry or a boolean `tools = false`, not multiple rendering modes.

## Skill rendering

Keep simple name + description:

```text
Skills:
- rust: Conventions for writing and reviewing Rust code.
- testing: Write, review, and fix tests across any language.
```

The model can read a skill file when needed.

Do not turn skills into top-level slash-command sprawl by default.

## Environment rendering

Keep minimal:

```text
Environment: cwd=/path, os=macos, date=2026-05-27
```

Consider dropping `home` from default environment because prompt appendices can reference `~` and tools already have cwd.

## Current code removal/replacement map

### `system_prompt.rs`

Remove or refactor:

- `Fact` as default prompt concept.
- `project_memory_status_layer` from default path.
- `working_style_lines` and personality imports.
- soul identity handling.
- guardrails prompt insertion.
- learning instruction insertion.
- global operating rules/tool routing doctrine.
- memory/user profile append by default.
- most mode instructions from default path.

Keep/refactor:

- `AssembledPrompt` and token estimate.
- tools rendering.
- skills rendering.
- AGENTS.md rendering.
- environment rendering.
- task/headless layer only for explicit task/worker execution, maybe split into separate builder.

### `builder.rs`

Change prompt assembly inputs:

- stop discovering `soul` for default prompt.
- stop always passing `personality: Some(&self.config.personality.profile)`.
- stop loading memory/user blocks by default.
- stop constructing mana `PromptContext` for normal prompt assembly.
- load prompt appendices from prompt config paths.

Keep:

- `system_prompt_override` for tests/internal override until prompt config supersedes.
- tool registration/filtering.
- skills discovery.
- AGENTS discovery.

### `config.rs`

Add:

- `PromptConfig` with system/tools/skills/project_instructions/environment/append.

Remove later:

- `PersonalityConfig` after personality backend removal.
- learning memory defaults if memory command/backend removed.
- prompt-relevant guardrail defaults if guardrails stop being prompt layer.

### `resources.rs`

Replace:

- `discover_soul`
- `discover_project_soul`
- `suggested_project_soul_path`

With:

- `discover_prompt_appendices(paths, cwd, user_config_dir)` or simpler path resolver.

Keep:

- `discover_agents_md`
- `discover_skills`
- `discover_prompts` if prompt templates remain separate useful feature.

## Migration/test plan

Add/adjust tests:

1. default prompt starts with exact one-line identity.
2. default prompt includes tools when `prompt.tools = true`.
3. default prompt includes skills when `prompt.skills = true`.
4. default prompt includes AGENTS.md when enabled.
5. default prompt includes configured append files in deterministic order.
6. default prompt omits personality/soul text.
7. default prompt omits memory/user profile by default.
8. default prompt omits mana facts/project memory status by default.
9. default prompt omits guardrail/mode/workflow doctrine by default.
10. explicit task/workflow runner prompt can include scoped execution instructions separately.

Expected test churn:

- many existing `system_prompt.rs` tests assert current layers and will need replacement or relocation.
- do not preserve old tests just to keep old bloat.

## Implementation sequence

1. Add `PromptConfig` with default one-line system prompt and append paths.
2. Add prompt appendix resolver/loader.
3. Refactor `system_prompt::assemble` around minimal layers.
4. Adjust `AgentBuilder::build()` to pass only retained prompt inputs by default.
5. Split task/headless/workflow-run prompt construction from global system prompt if needed.
6. Remove personality/soul prompt tests and replace with appendix tests.
7. After prompt builder is stable, remove personality UI/backend.

## Open decisions

1. Should environment include `home`, or only cwd/os/date?
2. Should prompt appendices appear before or after environment? Current recommendation: before environment.
3. Should project `.imp/prompt.md` override global `~/.imp/prompt.md` or append after it? Current recommendation: append global first, project second so project context can refine.
4. Should `system_prompt_override` stay as an internal escape hatch after prompt config lands? Current recommendation: yes, for tests/internal only.
