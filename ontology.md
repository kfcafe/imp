# imp ontology

Status: draft
Audience: humans and agents working on `imp`
Scope: shared language for describing imp's current features, planned seams, and boundary with `mana`

---

## Why this document exists

`imp` already has a lot of concepts: session, run, mode, worker, tool, context, memory, compaction, delegation, profile, provider, surface, and more.

Those terms are useful, but some are overloaded or used inconsistently across docs, planning notes, code, and conversation.

This document gives us a stable vocabulary so we can talk about `imp` more precisely when we are:

- planning features
- reviewing architecture
- naming UI surfaces
- describing runtime behavior
- deciding what belongs in `imp` vs `mana`
- writing mana units for imp work

This is a living document. It should prefer clarity and boundary quality over completeness.

---

## Core naming rules

These are the default language rules unless a document explicitly says otherwise.

1. **Use `surface` for a user-facing entrypoint or interaction shape**  
   Examples: TUI, CLI chat shell, headless `imp run`, planned view/inspector.

2. **Use `mode` for authority and behavioral constraints inside the runtime**  
   Examples: `full`, `worker`, `orchestrator`, `reviewer`.

3. **Use `session` for persisted conversation history**  
   A session can have branches and compaction boundaries.

4. **Use `run` for one bounded execution of the agent loop**  
   A session may contain many runs. `imp run <unit>` is a headless surface that starts a run.

5. **Use `turn` for one model-response cycle within a run**  
   A turn may include tool calls.

6. **Use `unit` or `mana unit` for durable work in `mana`**  
   Prefer `job` when the unit is an executable slice of work. Avoid inventing separate words like `subtask model` for the same thing.

7. **Use `context` for the model-visible input assembled for a turn or run**  
   Do not use `context` as a vague synonym for all memory, all history, or all project knowledge.

8. **Use `memory` only when something persists beyond the immediate turn**  
   When possible, say which layer: session memory, personal memory, mana work memory, or synthesized project knowledge.

9. **Use `compaction` for the system operation that reduces old session history**  
   Use `handoff summary` or `compaction summary` for the artifact it produces.

10. **Keep `verification`, `evidence`, and `completion` separate**  
    Verification is the check. Evidence is what was recorded. Completion is the workflow outcome.

---

## Feature families

These families give us a top-level way to classify `imp` functionality.

### 1. Interaction surfaces
How a human or system enters `imp`.

### 2. Runtime execution
How `imp` performs work once active.

### 3. Context and memory
How `imp` assembles relevant information without flooding the model.

### 4. Coordination and delegation
How `imp` works with `mana` and other workers.

### 5. Policy and safety
How `imp` constrains authority, capability, and risky behavior.

### 6. Tools and extensions
How `imp` acts on the environment.

### 7. Models and providers
How `imp` talks to external model services.

### 8. Observability and outputs
How `imp` exposes progress, results, logs, and structured outcomes.

---

## 1. System-level terms

### `imp`
The live worker runtime and orchestrator.

Owns live execution concerns such as:
- agent loop
- tool use
- session behavior
- context assembly
- model/provider interaction
- runtime policy enforcement
- operator-facing interaction surfaces

### `mana`
The durable substrate that stores work, facts, evidence, and coordination state.

Use `mana` when referring to what should survive a worker cold. Use `imp` when referring to what happens during execution.

### `mana ↔ imp contract`
The boundary between durable work state and live execution.

Use this phrase when discussing:
- what data crosses from `mana` into `imp`
- what outcomes `imp` writes back
- how worker assignments, verification, and evidence are represented

---

## 2. Actors and roles

### `operator`
The human using or supervising `imp`.

### `agent`
The model-driven worker behavior inside `imp`.

Use `agent` when talking about the reasoning-and-tool-using entity, not the human UI.

### `worker`
An `imp` agent executing a bounded piece of work.

Most useful when talking about:
- headless execution
- mana-assigned work
- delegated jobs
- constrained task completion

### `orchestrator`
An `imp` behavior or surface focused on planning, decomposition, dispatch, and coordination.

Use this when `imp` is deciding what work should happen or how work should be split, not when it is simply doing one local task.

### `reviewer`
A read-oriented analysis role focused on inspection, critique, or verification support rather than mutation.

### `persona role`
A personality/config term describing how `imp` presents itself socially or stylistically.

Examples from config: `agent`, `assistant`, `worker`, `collaborator`, `partner`, `reviewer`, `planner`.

Use `persona role` or `personality role` when talking about profile/config language so it does not get confused with runtime `mode`.

---

## 3. Interaction surfaces

A **surface** is a user-facing way to enter or observe the runtime.

### `TUI surface`
The fullscreen cockpit opened by `imp` or `imp tui`.

Characteristics:
- rich interactive terminal UI
- session navigation
- command palette
- sidebar/tool inspection
- model selection and personality editing

### `CLI chat surface`
The line-oriented interactive shell opened by `imp chat`.

Characteristics:
- interactive, but lighter than the TUI
- text-first terminal workflow
- shares the same runtime concepts underneath

### `print surface`
A one-shot invocation such as `imp -p "..."`.

Use this term for single-question or single-response invocations rather than long-lived interactive sessions.

### `headless worker surface`
The non-interactive execution path, most notably `imp run <mana-id>`.

Use this when discussing `imp` as a native worker runtime for mana-shaped work.

### `view surface` *(planned)*
A planned read-oriented inspector surface.

Because this is not yet a mature shipped surface, documents should mark it as planned rather than treating it as established runtime reality.

---

## 4. Runtime execution terms

### `run`
One bounded execution of the agent loop.

A run may be interactive or headless. A single session can contain many runs.

### `turn`
One cycle of:
1. assemble context
2. call the model
3. receive assistant output
4. execute tools if requested
5. continue or stop

### `agent loop`
The repeating think/act/observe cycle that powers `imp`.

Use this term for the runtime control flow, not for the entire product.

### `tool call`
A model-requested invocation of a registered tool.

### `observation`
The result of a tool call as seen by the runtime and then, selectively, by the model.

### `completion`
The point where a run produces its final user-facing answer or worker outcome.

Do not use `completion` as a synonym for provider completion APIs only; within `imp`, it can also mean workflow completion of a run.

### `handoff`
Information prepared so another future run or worker can continue effectively.

A handoff may be:
- session-local, such as a compaction summary
- mana-durable, such as a unit update or evidence record

---

## 5. Modes, authority, and policy

### `mode`
A runtime authority profile that changes what the agent is allowed to do and what tools appear.

Current core modes described in docs:
- `full`
- `worker`
- `orchestrator`
- `reviewer`

### `capability`
A concrete permission or allowed action inside the runtime.

Examples:
- may run shell commands
- may edit files
- may create mana units
- may perform network access

A mode is a bundle of capabilities. Do not use the words interchangeably.

### `tool visibility`
Whether a tool is shown to the model in the active mode.

### `tool gating`
Whether the runtime will actually allow execution of a tool call.

Important distinction: visibility is prompt-level exposure; gating is runtime enforcement.

### `policy`
Deterministic runtime rules that constrain behavior.

Use `policy` for host-enforced behavior, not for soft prompt advice.

### `guardrail`
A higher-level behavioral or safety constraint, often encoded through configuration or prompt shaping.

Use carefully: if the constraint is actually host-enforced, `policy` is usually the better word.

### `sandbox` *(target-state term)*
An isolated execution boundary for risky or concurrent work.

Use this as an architectural target or runtime-boundary term, not as a claim that every current imp run is fully sandboxed.

### `worktree isolation` *(target-state term)*
A concurrency-oriented isolation surface that keeps multiple workers from colliding in one checkout.

---

## 6. Work and coordination terms

### `assignment`
A bounded piece of work handed to a worker.

In the Tower happy path, the canonical durable assignment is a mana unit.

### `mana unit`
A durable work object in `.mana/`.

Use this as the generic term when the exact kind does not matter.

### `job`
An executable mana unit representing concrete work.

Use `job` when discussing work that a worker should actually perform, especially delegated child jobs.

### `epic`
A higher-level mana unit that groups related child jobs or tracks a broader feature thread.

### `child job`
A mana job delegated beneath a parent unit.

This is the preferred term for delegated work. Avoid inventing separate terms like `subtask object` or `mini-plan item` when mana already holds the durable structure.

### `delegation`
The act of turning broad work into one or more bounded assignments for other workers.

### `orchestration`
The higher-level process of sequencing, dispatching, tracking, and reconciling work across units or workers.

### `verify gate`
The explicit command or check that must pass for a work item to count as verified.

### `evidence bundle`
The durable set of outputs that justify a claimed result.

This may include:
- verify command results
- changed files
- review output
- summarized outcomes
- references another worker can inherit

---

## 7. Context and memory terms

### `context`
The model-visible input assembled for a specific turn or run.

Context may include:
- active session history
- file prefill
- system prompt layers
- attached files
- mana task briefing
- relevant memory

### `context assembly`
The runtime process of selecting and packaging relevant inputs for the model.

### `prefill`
Context injected ahead of the next model call.

Common examples:
- file contents
- mana unit context
- compacted handoff summaries

### `attachment`
A user-specified file or artifact explicitly included in the request, such as `@file` in chat.

### `session`
The persisted conversation record, including branches and tool history.

### `branch`
A forked line of conversation within a session.

### `observation masking`
Replacing older tool outputs with lightweight placeholders to reduce token load while preserving action history.

### `compaction`
A stronger context-reduction step that summarizes older history into a structured handoff while preserving raw history on disk.

### `active history`
The subset of session history currently exposed to the model after masking and/or compaction.

### `session memory`
What persists inside session files for one conversation tree.

### `personal memory`
Cross-session global memory stored for the agent/user outside a specific project.

### `mana work memory`
Durable project work state stored in `.mana/`, including units, facts, notes, dependencies, and verify history.

### `synthesized project knowledge` *(planned)*
A future maintained knowledge layer such as `.mana/wiki/` that turns raw durable artifacts into reusable project understanding.

### `durable handoff`
A handoff written somewhere another worker can inherit cold, usually in `mana` rather than only in session history.

---

## 8. Tooling and extension terms

### `tool`
A structured action the agent can invoke through the runtime.

Use `tool` for the runtime abstraction, not for every shell command or external program.

### `native tool`
A tool implemented directly by imp in Rust.

Examples: `read`, `write`, `edit`, `mana`, `memory`.

### `shell tool`
A configured tool that ultimately executes shell behavior but is exposed through imp's tool system.

### `Lua extension`
A user-installed script that can register tools, hooks, or commands.

### `extension`
A capability added to imp through an explicit extension seam rather than the built-in core.

### `guest runtime` *(target-state term)*
A stricter future framing for extension execution where Lua is one possible backend, not the entire conceptual model.

### `readonly tool`
A tool that inspects without mutating state.

### `mutating tool`
A tool that changes files, work state, configuration, or external systems.

### `tool schema`
The structured input contract for a tool.

### `tool result`
The structured output returned from a tool invocation.

### `hook`
An extension callback that runs around lifecycle events such as tool execution.

---

## 9. Models, providers, and auth terms

### `provider`
The external service integration layer.

Examples: Anthropic, OpenAI, Google, Exa, Tavily.

### `model`
A specific model identifier used through a provider.

Examples: Sonnet, GPT-5.4, Gemini 2.5 Pro.

### `model selection`
The runtime decision or operator action choosing which model to use.

### `thinking level`
A runtime hint about how much reasoning budget or effort the model should use.

### `auth store`
Imp's secure credential storage layer.

### `secret provider`
A named credentials entry inside the auth store.

This is separate from an LLM provider. For example, Exa may be both a web provider and a secret provider name in config/auth storage.

---

## 10. Personality and configuration terms

### `config`
The layered runtime configuration resolved from defaults, user config, project config, environment variables, and CLI flags.

### `personality`
The operator-configurable behavioral presentation layer for `imp`.

This includes identity and sliders such as autonomy, verbosity, caution, warmth, and planning depth.

### `personality profile`
A named saved personality configuration.

Use this phrase instead of just `profile` when clarity matters.

### `identity`
The named self-presentation fields inside personality config.

### `slider`
A bounded tuning dimension within personality config.

### `profile`
An overloaded word. Prefer a qualified phrase:
- `personality profile`
- `guardrail profile`
- `sandbox profile`
- `provider profile`

Do not rely on `profile` alone when the subtype matters.

---

## 11. Output and observability terms

### `human-facing output`
Text or UI intended primarily for an operator.

Examples:
- chat reply
- TUI render
- progress text
- explanatory summary

### `machine-facing output`
Structured data meant for reliable downstream consumption.

Examples:
- tool results
- JSON envelopes
- mana record updates
- usage records

### `event`
A structured runtime occurrence such as a model delta, tool start, tool finish, error, or completion marker.

### `artifact`
A durable file or output produced during work.

### `usage accounting`
Structured tracking of token/cost/resource consumption.

### `log`
A diagnostic or historical record of runtime behavior.

### `report`
A human-readable synthesis of what happened, what changed, and what remains.

---

## 12. Preferred distinctions

These distinctions should stay crisp.

### `surface` vs `mode`
- A **surface** is how you interact with imp.
- A **mode** is what authority the runtime has once active.

Example: the TUI surface may run in `full` mode; headless `imp run` commonly implies `worker` mode.

### `session` vs `run`
- A **session** is the persisted conversation tree.
- A **run** is one bounded execution inside or alongside that session.

### `context` vs `memory`
- **Context** is what the model sees now.
- **Memory** is what persists and may be retrieved later.

### `delegation` vs `orchestration`
- **Delegation** is assigning a slice of work.
- **Orchestration** is managing many pieces of work over time.

### `verification` vs `evidence`
- **Verification** is the act/check.
- **Evidence** is the recorded proof or supporting output.

### `tool` vs `capability`
- A **tool** is an interface the model can call.
- A **capability** is the permission to use certain actions.

### `personality role` vs `runtime mode`
- A **personality role** affects presentation/style.
- A **runtime mode** affects authority and tool access.

---

## 13. Terms to avoid or qualify

### Avoid using `context` as a catch-all
Instead say:
- session history
- file prefill
- mana briefing
- personal memory
- active history

### Avoid using `profile` by itself
Prefer the qualified subtype.

### Avoid using `task` when `mana unit` or `job` is the real thing
`Task` is fine informally, but docs should prefer the durable substrate term when it matters.

### Avoid calling planned seams fully real
If something is not yet mature, label it:
- planned
- target-state
- proposal
- migration term

### Avoid collapsing `imp` into just `chat`
`imp` includes chat, but it is better described as a runtime with multiple surfaces.

---

## 14. Working glossary

If we need a short default glossary for planning docs, use this set first:

- `surface`
- `mode`
- `session`
- `run`
- `turn`
- `agent loop`
- `tool`
- `context assembly`
- `compaction`
- `personal memory`
- `mana work memory`
- `assignment`
- `child job`
- `delegation`
- `orchestration`
- `verify gate`
- `evidence bundle`
- `provider`
- `model`
- `personality profile`
- `policy`
- `capability`

---

## 15. Open normalization questions

These are the terms most likely to need another pass.

1. **`run`**  
   We should decide how strongly to separate:
   - provider completion run
   - agent-loop run
   - headless `imp run` surface
   - mana orchestration run

2. **`profile`**  
   We should keep reducing overload across personality, policy, and future sandboxing.

3. **`view`**  
   We should decide whether `view` becomes a first-class surface family or stays a command namespace under other surfaces.

4. **`worker` vs `agent`**  
   We should decide when `worker` implies mana-shaped bounded execution and when it is just a synonym for agent.

5. **`evidence bundle`**  
   We should decide whether this becomes a formal schema term or remains a design-level phrase.

---

## Suggested usage in future docs

When writing new imp docs or mana units about imp, prefer sentences like:

- "The TUI is a surface; `full` is the default interactive mode."
- "`imp run` is a headless worker surface that executes a mana assignment."
- "Compaction reduces active history and produces a handoff summary."
- "The runtime enforces capability policy through tool visibility and gating."
- "This feature belongs to context assembly, not durable memory ownership."
- "Use a mana child job for delegated work rather than inventing a separate subtask model."

---

## Source grounding for this draft

This first pass is grounded in current repo language from:
- `README.md`
- `ARCHITECTURE.md`
- `imp_rebuild_strategy.md`
- `imp_rebuild_plan.md`
- `docs/proposals/imp-memory-architecture-and-mana-boundary.md`

It should be updated as the runtime and docs become more explicit.
