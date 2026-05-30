# Prompt and Runtime Prompt Audit

Status: draft prompt inventory for `tighten-imp-product-surface`.

## Why this matters

The prompt stack is part of imp's product surface. If the system prompt is long, rigid, or full of old experiment vocabulary, the runtime will feel bloated even if slash commands are cleaned up.

Goal: make imp's default system prompt as small as possible while preserving a strong coding agent. Prefer configurable prompt layers over hard-coded doctrine.

## Current prompt assembly path

Primary assembly lives in:

- `crates/imp-core/src/system_prompt.rs`
- called by `crates/imp-core/src/builder.rs`

`AgentBuilder::build()` assembles the prompt unless `system_prompt_override` is set.

Current inputs:

- tool registry definitions
- `AGENTS.md` resources
- skills index
- mana/project facts
- project memory status
- guardrails prompt
- task context
- headless execution contract
- memory/user profile blocks
- personality profile or soul doc
- role instructions/schema metadata
- agent mode instructions
- learning instructions
- environment block

Existing override:

- CLI/builder can replace the whole system prompt with `system_prompt_override`.

Missing configurability:

- no first-class config for selecting prompt preset/detail level
- no config for turning individual layers on/off except indirect features like learning/guardrails
- no explicit compact/default/full prompt modes
- no easy way to keep core identity/routing while disabling personality, skills index, facts, or role/schema verbosity

## System prompt layers and audit notes

### Identity + tools

Current default identity:

```text
You are imp, a coding agent.
```

Then it lists every available tool as `- name: description`.

Keep, but consider making tool descriptions compact/configurable. The native tool surface is already relatively small, so this is not the biggest bloat source.

### Personality / soul

`system_prompt.rs` imports `crate::personality` and renders:

- custom identity sentence from personality profile
- working style lines derived from sliders
- soul doc overrides personality identity

Concern:

- user is already questioning `/personality` and the personality backend.
- personality-generated working style duplicates hard-coded operating rules.
- this is a non-essential product concept unless it clearly improves agent behavior.

Recommendation:

- mark personality prompt layer as remove candidate.
- keep `soul`/plain config-file prompt override only if a lightweight custom identity hook is still desired.
- if removed, replace with one concise, static identity plus optional user-supplied prepend/append prompt config.

### Tool routing

Current routing rules:

- bash for shell-native search/build/tests/package managers; prefer scan for code structure
- git for git operations
- workflow for durable project plans/status/validation/orchestration
- read before explaining/editing files; edit/write for changes

Keep, but compress. These are useful and not too product-specific.

Potential compact form:

```text
Use read/scan before code claims, edit/write for file changes, bash for builds/tests/search, git for repo state, workflow for durable plans/status.
```

### Operating rules

Currently many hard-coded rules exist. Good ideas, but too much doctrine for the minimal default prompt.

Rules include:

- re-check user intent
- ask clarification for ambiguous continuation
- treat done/status labels as internal closeout semantics
- ground claims in inspected evidence
- stay read-only for analysis-only requests
- handle failed commands/errors as blockers
- ask on scope/risk/user-visible uncertainty
- concise evidence-oriented replies
- use workflows for durable work
- record progress after failures/material planning changes
- treat workflow task as contract
- stop only on verified completion/blocker/decision point

Recommendation:

- keep a minimal default subset.
- move detailed workflow/task doctrine into workflow/tool guidance or a `prompt.preset = "full"` mode.

Proposed minimal default:

```text
Act as a practical coding agent. Inspect before changing or making repo claims. Make small reversible changes. Ask one focused question when scope/risk/intent is ambiguous. Use workflow only for durable plans/status. Verify changed behavior with the narrowest useful check. Reply naturally and briefly; closeout statuses are internal unless asked.
```

### Agent mode instructions

`AgentMode::instructions()` in `crates/imp-core/src/config.rs` is large, especially orchestrator/planner/worker modes.

Concern:

- mode instructions encode a lot of workflow/mana-era product behavior.
- current product direction is TUI + one-shot + workflows, not many modes as visible product identity.

Recommendation:

- keep mode policy enforcement in runtime/tool filtering.
- make mode prompt text much shorter.
- consider removing or hiding planner/orchestrator/reviewer/auditor modes unless needed by workflow child runs.

### Learning instructions

Learning prompt text is added when `config.learning.enabled` is true.

Concern:

- learning defaults to enabled.
- memory/user profile can add prompt bulk and behavior drift.
- `/memory` command and skill nudges may be nonessential.

Recommendation:

- consider defaulting learning prompt injection off for product release.
- keep memory as optional configurable context, not core behavior.

### Skills index

The system prompt lists all discovered skills with compact descriptions and tells the agent to load skill files by reading them.

This supports extensibility, but can be noisy when many skills exist.

Recommendation:

- keep skills as Pi-like minimal extensibility, but make skill index optional or thresholded.
- consider `prompt.skills = "off|names|compact|full"`, default `names` or `compact`.

### AGENTS.md / project context

Keep. This is valuable, expected coding-agent behavior.

Recommendation:

- make it configurable only for advanced users; default on.
- maybe cap/project summarize if too large later.

### Facts / mana project memory status

Concern:

- this is legacy mana-flavored context.
- workflow should replace durable orchestration.

Recommendation:

- remove or disable by default unless workflow-native project status replaces it.
- do not inject mana facts/status into default prompt during product cleanup.

### Guardrails layer

Engineering guardrails can be useful but should be explicitly configured.

Recommendation:

- keep, but default should be off or profile-specific, not silent large prompt expansion.

### Task / headless execution layer

Useful for one-shot/headless workflow tasks, but verbose.

Recommendation:

- keep for explicit task execution only.
- shorten headless execution contract.
- ensure it does not force status-label headings in normal replies.

### Environment layer

Current environment block includes:

```text
Environment: cwd=..., os=..., home=..., date=...
```

Keep, but consider whether `home` belongs in prompt by default. It is useful for local path operations but also prompt bulk/exposure.

## Other runtime-injected prompts

### Post-turn follow-up prompts

In `crates/imp-core/src/agent/mod.rs`:

- `confidence_continue_follow_up_text()`
- `failed_bash_recovery_follow_up_text()`
- `execution_debt_follow_up_text()`

These are hidden user messages injected by runtime loop policy.

Concern:

- they can make imp feel rigid/autonomous even when the user expected conversation.
- they are hard-coded and not obviously configurable beyond `continue_policy` for the confidence path.

Recommendation:

- expose runtime follow-up prompt policy under config.
- keep failed bash recovery and execution debt only when auto-loop/workflow execution is active, not normal conversation by default.
- shorten these prompts.

### Workflow controller prompts

In `crates/imp-core/src/workflow/controller.rs`:

- `workflow_bootstrap_prompt()`
- `workflow_supervision_prompt()`
- `workflow_decomposition_prompt()`
- `workflow_graph_closeout_prompt()`
- `workflow_direct_closeout_prompt()`
- `workflow_closeout_prompt()`

These decide whether workflow runtime forces another model turn.

Current controller state tracks:

- workflow id
- mana root id
- active unit id
- child runs
- graph/direct closeout required
- budget counters
- bootstrap state
- graph shape
- planning state
- closeout readiness/blocker

Concern:

- it is still mana-backed/compatibility-shaped despite being named workflow.
- `closeout.ready` defaults true, but many signals set it false and force closeout loops.
- bootstrap requires a bound mana root, which conflicts with workflows as the new durable primitive.
- controller prompts are long and prescriptive.

Recommendation:

- redesign this as a workflow-native controller that references workflow id/steps/checks/artifacts instead of mana root/unit ids.
- make controller continuation policy configurable and visible:
  - `workflow.controller = "off|light|strict"`
  - default likely `light` for TUI, `strict` for child workflow runs.
- in light mode, prefer surfacing next-step suggestions to the user over injecting hidden forced continuation prompts.
- reserve strict controller loops for explicit workflow run/child-worker execution.

### Mana compatibility prompts

In `crates/imp-core/src/agent/workflow_integration/mana_compat.rs`:

- `mana_workflow_follow_up_text()`
- `mana_externalization_follow_up_text()`
- `mana_skill_follow_up_hint()`
- `orchestration_follow_up_text()` uses imp-work/mana wording

Concern:

- direct source of bloat vocabulary: imp-work, mana, work(action=...), native work guide.
- should be removed, archived, or isolated behind compatibility if workflow-native replacement exists.

Recommendation:

- high-priority backend cleanup target.
- archive or rewrite to workflow-native prompts.
- do not expose these in default product runtime.

### TUI workflow context prompt

`crates/imp-tui/src/app.rs` has `workflow_context_prompt_for_request()` around lines 1737+.

It injects context for:

- improve safe mode
- improve mode sandbox/worktree
- active mana scope

Concern:

- old improve/mana concepts leak into every agent start request that carries those modes.

Recommendation:

- remove with improve/mana command cleanup.
- replace with workflow-native context only if needed.

## Prompt bloat / rigidity root causes

1. Hard-coded operating doctrine is too long.
2. Personality and operating rules duplicate each other.
3. Agent modes encode large workflow behavior blocks.
4. Mana/imp-work compatibility prompts still drive runtime continuation behavior.
5. Workflow controller is strict, hidden, and mana-root-shaped.
6. Learning/memory/skills are injected by default rather than being clearly configurable prompt layers.
7. The only prompt override is all-or-nothing, so users cannot easily keep core imp while disabling layers.

## Proposed configurability model

Add config around prompt assembly instead of only `system_prompt_override`.

Possible TOML:

```toml
[prompt]
preset = "minimal" # minimal | standard | full | custom
identity = "imp"
include_tools = true
tool_descriptions = "compact" # names | compact | full
include_tool_routing = true
include_operating_rules = true
include_environment = true
include_agents_md = true
include_skills = "compact" # off | names | compact
include_project_facts = false
include_memory = false
include_user_profile = false
include_personality = false
include_guardrails = "configured" # off | configured | always
include_mode_instructions = "compact" # off | compact | full
append = []
```

Runtime policy:

```toml
[runtime.follow_up]
failed_command_recovery = "off" # off | suggest | auto
execution_debt = "off"          # off | suggest | auto
confidence_continue = "off"     # existing continue_policy maps here

[workflow.controller]
mode = "light" # off | light | strict
strict_for_child_runs = true
```

Recommended product defaults:

```toml
[prompt]
preset = "minimal"
include_agents_md = true
include_skills = "names"
include_memory = false
include_user_profile = false
include_personality = false
include_project_facts = false
include_mode_instructions = "compact"

[workflow.controller]
mode = "light"
strict_for_child_runs = true
```

## Recommended implementation sequence

1. Finish inventory of prompt sources and current token footprint.
2. Add prompt audit artifact to tightening workflow.
3. Introduce `PromptConfig` with minimal/standard/full presets.
4. Wire `PromptConfig` into `system_prompt::assemble` and `AgentBuilder`.
5. Add tests that minimal prompt excludes personality, mana facts/status, learning memory, verbose mode instructions, and status-label response mandates.
6. Rewrite/shorten workflow controller prompts.
7. Remove or isolate `mana_compat` prompts after command/backend cleanup.
8. Consider removing personality backend once prompt config can cleanly disable it.

## Open design decision

Should minimal prompt still include skill discovery by default?

Argument for yes:

- skills are the main Pi-like extensibility path.

Argument for no:

- many installed skills can bloat prompt and distract the model.

Compromise:

- default to skill names only, with descriptions loaded only on search or explicit use.
