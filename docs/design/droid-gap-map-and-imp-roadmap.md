# imp gaps behind Droid and imp-native implementation direction

Status: draft
Date: 2026-05-22

Purpose: capture the concrete areas where Factory Droid currently presents a stronger product surface than imp, then translate those gaps into an imp-native roadmap. This is not a mandate to clone Droid. The goal is to preserve imp's strengths: local-first runtime control, native imp-work, durable evidence, policy enforcement, and terminal-first ergonomics.

## Strategic framing

Droid's advantage is mostly packaging: Missions, Custom Droids, MCP, hooks, review workflows, IDE surfaces, and team integrations are presented as coherent product features.

imp's advantage is runtime substance: local durable work, inspectable runs, evidence, verification gates, explicit tool/runtime boundaries, native tools, hooks, and an open-source codebase.

The best direction is to wrap imp's primitives in stronger product surfaces rather than copying Factory's platform shape.

## Non-goals for now

- Do not prioritize installable extension bundles or a public extension marketplace yet.
- Do not reintroduce workflow terminology. New workflow surfaces should use native `imp-work` terms.
- Do not turn every workflow into project workflowgement UI. Keep chat and terminal flow primary.
- Do not build SaaS/team surfaces before local runtime/product quality is strong.

## 1. `.imp/agents`: first-class custom agents

### Droid reference

Droid has Custom Droids: Markdown-defined subagents in `.factory/droids/` or `~/.factory/droids/`, each with a prompt, model preference, and tool policy. They can be selected in the UI and delegated to by the primary agent.

### Current imp position

imp has modes, hooks, skills, native tools, and imp-work workers, but no simple project-visible custom agent definition format. Users cannot easily check in a named reviewer, researcher, QA agent, or migration worker as code.

### Gap

High. This is one of Droid's clearest product primitives and maps cleanly to imp.

### imp-native design

Add project and personal agent directories:

```text
<repo>/.imp/agents/
~/.config/imp/agents/
```

Agent file format:

```md
---
name: security-reviewer
description: Review code changes for security and data-handling risks
mode: reviewer
model: inherit
thinking: medium
tools: ["read", "scan", "git", "web"]
autonomy: safe
---

You are a focused security reviewer. Inspect the provided diff and relevant code.
Report only actionable risks, false-positive caveats, and verification needed.
```

Suggested surfaces:

```sh
imp agent list
imp agent show security-reviewer
imp agent run security-reviewer -- "review the staged diff"
```

TUI:

- `/agents` browser
- create/edit/view/reload actions
- show project vs personal source
- show effective mode, model, tools, and autonomy

imp-work integration:

- allow a task or run to specify `agent = "security-reviewer"`
- allow coordinators to delegate a task to a named agent
- record the selected agent in run metadata and evidence

Important design choice: agents should be policy-constrained objects, not prompt macros. Their mode, tool set, write permissions, and autonomy should flow through the reference monitor.

## 2. Mission Control equivalent over native imp-work

### Droid reference

Droid Missions provide collaborative planning, features/milestones, orchestrator + worker + validator agents, a Mission Control view, and headless `droid exec --mission`.

### Current imp position

imp is moving from workflow to native imp-work. imp-work already has tasks, epics, prototypes, context packs, runs, attempts, leases, path locks, checks, outcomes, memory, and structured scheduler state.

### Gap

Medium-high. imp has stronger primitives than Droid's public docs show, but lacks a clear Mission Control product surface over them.

### imp-native design

Do not call it Missions unless that term proves best. Candidate names:

- Work Control
- Run Control
- Project Run
- Workbench
- Control Board

Core workflow:

```text
conversation -> imp-work epic/task plan -> context packs -> worker runs -> checks/evidence -> structured outcomes -> replan/close
```

Product surfaces:

```sh
imp work plan "migrate auth to oauth"
imp work run <epic-or-task-id>
imp work control
imp run --work <task-id>
```

TUI view should show:

- active task/epic objective
- planned tasks and dependencies
- active leases/workers
- current run phase
- changed paths
- checks pending/running/passed/failed
- evidence and outcome summaries
- blockers and decisions needed

Control actions:

- pause / resume
- stop a worker
- retry failed task
- mark blocked with reason
- replan remaining work
- split a task
- assign a named `.imp/agents` agent
- run verification only

Validation model:

- imp-work checks are first-class, not separate Droid-style validator magic
- validators can be named agents using reviewer/auditor modes
- evidence packets should attach to task outcomes

Key imp distinction: Droid Missions feel like multi-agent project workflowgement. imp Work Control should feel like inspectable local execution with proof.

## 3. MCP support

### Droid reference

Droid has first-class MCP: `/mcp`, `droid mcp add/remove`, HTTP and stdio transports, OAuth, registry, user/project config, disabled tools, and enterprise allowlists.

### Current imp position

imp has strong native tools and extensibility, but MCP is not currently a comparable product surface in the inspected docs.

### Gap

High. MCP has become an expected integration layer.

### imp-native design

CLI:

```sh
imp mcp list
imp mcp add linear https://mcp.linear.app/mcp --type http
imp mcp add playwright "npx -y @playwright/mcp@latest"
imp mcp remove linear
imp mcp doctor
```

Config:

```text
~/.config/imp/mcp.json
<repo>/.imp/mcp.json
```

TUI:

- `/mcp` workflowger
- list servers and connection state
- inspect tools
- enable/disable server or individual tools
- authenticate when supported
- show policy-denied tools clearly

Runtime integration:

- MCP tools become normal tools behind the reference monitor
- tool names should be namespaced, e.g. `mcp.linear.create_issue`
- secrets/OAuth tokens should use imp's existing secret storage model
- MCP calls should appear in traces and evidence like native tool calls
- mode/tool policy should be able to allow a server but deny specific tools

Initial priority:

1. stdio MCP
2. HTTP MCP
3. project/user config layering
4. policy integration
5. TUI workflowger
6. curated registry later

## 4. Hooks UX improvements

### Droid reference

Droid exposes lifecycle hooks through `/hooks` and shell-command configuration. Hooks can run before/after tools, on prompt submit, notification, stop, compaction, session start/end, and subagent stop.

### Current imp position

imp supports hooks, including Lua hooks and blocking tool hooks, and has a reference-monitor direction. The gap is not capability as much as discoverability and ergonomics.

### Gap

Medium. imp likely has enough hook power, but the UX can be much better.

### imp-native design

Keep Lua hooks for advanced behavior, but add a simple declarative hook layer for common cases.

Example:

```toml
[[hooks.pre_tool_use]]
tool = "bash"
command = "$IMP_PROJECT_DIR/.imp/hooks/check-command.sh"
blocking = true

[[hooks.after_file_write]]
path_glob = "**/*.rs"
command = "cargo fmt -- {path}"
blocking = false
```

TUI:

- `/hooks` browser/editor
- show active hooks by source: built-in, project, personal
- test hook with sample event
- show last hook failures
- enable/disable without deleting

Runtime:

- hook decisions should be normalized as policy records
- blocking hooks should produce clear denial messages and recovery suggestions
- hook outputs should be redacted/truncated consistently

## 5. `/review` workflow

### Droid reference

Droid has a visible `/review` command and Droid Action for PR review/security scans/PR descriptions.

### Current imp position

imp has reviewer/auditor modes, git tools, scan tools, web, evidence, and audit tools available in the runtime. It lacks a polished single-command review workflow.

### Gap

Medium-high because it is high user value and easy to explain.

### imp-native design

CLI:

```sh
imp review
imp review --staged
imp review --base main
imp review --security
imp review --format markdown
imp review --format sarif
```

TUI:

- `/review` command
- choose staged/uncommitted/base branch
- choose focus: correctness, security, tests, docs, migration risk
- emit findings with severity and file refs

Output should include:

- summary
- blocking findings
- non-blocking concerns
- test/verification recommendations
- files inspected
- evidence packet path when applicable

imp-work integration:

- create follow-up tasks from findings
- attach review evidence to current task/run
- use `.imp/agents/security-reviewer.md` or `.imp/agents/code-reviewer.md` when configured

## 6. Policy: what it means beyond system prompts and hooks

### Practical definition

Policy is the runtime answer to:

> May this action happen in this context, under this identity/mode/task, against this target, with these inputs, and what audit record explains the decision?

A system prompt asks the model to behave. Hooks can add programmable checks. Policy is the deterministic enforcement layer that does not depend on the model remembering instructions.

### Concrete policy examples

Tool policy:

- reviewer mode can read and scan, but cannot write files or run mutating shell commands
- a named agent can use `git diff` but not `git commit`
- MCP `linear.read_issue` is allowed, but `linear.update_issue` requires approval

Path policy:

- allow writes only under `crates/imp-core/**`
- deny edits to `.env`, lockfiles, generated files, or migration directories unless explicitly approved
- allow test snapshots only when the run is in update-snapshots mode

Command policy:

- allow `cargo test -p imp-core`
- require approval for `rm`, `sudo`, `curl | sh`, package installs, networked deploy commands, or long-running daemons
- deny repeated identical commands after a loop threshold

Network/secrets policy:

- allow web search in research mode but deny arbitrary HTTP POSTs
- allow reading secret metadata but never secret values into model-visible output
- restrict which MCP servers can access credentials

Autonomy policy:

- in `safe`, suggest changes but do not write
- in `local-auto`, write local files and run tests, but ask before commits or installs
- in `ci`, run non-interactively but only within a declared allowlist

Task/work policy:

- a worker may only touch paths declared in its imp-work context pack
- a run cannot close `DONE` until required checks pass or a force reason is recorded
- a task with stale context must refresh before execution

Provider/model policy:

- disallow certain providers for private repos
- use local/private models for sensitive paths
- require high-reasoning model for validator agents

### Implementation direction

The existing ReferenceMonitor direction is the right foundation. Extend it so all consequential actions produce a `PolicyDecision`:

```text
Allow | Deny | RequireApproval | Warn | InvalidInput
```

Each decision should include:

- source: mode, run policy, project policy, hook, reference monitor, user approval, extension policy
- subject: active user/session/agent/run
- action: tool name + normalized action class
- target: path/server/command/resource when applicable
- reason and suggested recovery
- redacted inputs or input hash

Policy should be visible in three places:

1. inline when it blocks or asks for approval
2. run trace/evidence for audit
3. TUI policy/debug panel for advanced users

## 7. ACP support

### Droid reference

Droid supports ACP integrations for JetBrains and Zed.

### Current imp position

imp is terminal-first and exposes a Rust SDK direction, but ACP is not a documented product surface.

### Gap

Medium. Important for editor adoption, but lower priority than custom agents/MCP/review if terminal quality is the current focus.

### imp-native design

Implement ACP as a thin host adapter over imp-core, not as a separate agent brain.

Goals:

- editor can start/resume an imp session
- attach current file/selection/context
- stream assistant/tool events
- request edits as reviewable patches
- surface approvals in editor
- use same policy, hooks, agents, skills, MCP, and imp-work runtime as terminal imp

Suggested sequence:

1. document ACP protocol requirements and event mapping
2. add `imp acp serve` or equivalent stdio/socket server
3. support chat + file context + patch proposals
4. add work/run status events
5. test with Zed first, then JetBrains if feasible

## 8. Slack support

### Droid reference

Factory positions Droid as working across Slack/Teams and enterprise tools.

### Current imp position

imp does not currently present a Slack app/bot surface.

### Gap

Medium-long-term. Useful, but only after core run/control surfaces are solid.

### imp-native design

Slack should be a host adapter around imp, not a separate product brain.

Possible first version:

- mention bot in a channel/thread
- create or continue an imp session tied to the Slack thread
- summarize repo/task context
- create imp-work tasks from messages
- post review summaries or run outcomes back to thread
- require explicit approval for code changes, commits, or external side effects

Architecture:

```text
Slack event -> imp host adapter -> imp-core session/run -> policy/approval -> result posted to thread
```

Important constraints:

- strong workspace/repo authorization
- clear identity mapping between Slack user and local/remote execution identity
- no secrets in Slack transcript
- default read-only until a trusted approval path exists

## 9. Skills: not a major Droid gap

Droid has a polished skills product, but imp already has skills as part of the agent environment. Do not treat skills as a core capability gap.

Remaining imp opportunities are mostly UX and documentation:

- make skill discovery clearer in the TUI
- show which skills were loaded and why
- allow project-local `.imp/skills` if not already standardized
- record skill use in run evidence
- provide authoring docs and examples

This should be positioned as refinement, not parity work.

## 10. Marketing and packaging questions

### Open source now vs paid license

Open source is likely the stronger near-term release path.

Reasons:

- imp needs trust. A coding agent with shell, filesystem, secrets, hooks, and policy benefits from source visibility.
- The product is still evolving. Open source reduces the polish burden because early adopters tolerate rough edges when they can inspect and contribute.
- Developer tools grow through credibility, examples, integrations, and community proof.
- imp's differentiator is runtime quality, not a closed hosted network effect yet.

A paid license can work later, but only when the value is packaged enough that users are buying a product rather than sponsoring potential.

### Is `$10` too cheap?

Probably yes for a serious developer tool if it is a commercial license. `$10` can make the product feel small, under-supported, or hobby-grade, while not generating enough revenue to fund support.

Better options:

1. Open source core now; paid hosted/team features later.
2. Open source with sponsor/support tiers.
3. Free local CLI; paid Pro when polish and hosted/team features exist.
4. Paid app later at a higher price, likely `$15-25/month` for individual Pro or `$49-99/year` for a personal license.

A one-time `$10` license is usually awkward: too low for sustainability, high-friction compared to free/open source, and not enough to signal professional value.

### Suggested positioning for open-source launch

Position imp as:

> The local-first coding agent for durable, inspectable software work.

Supporting claims:

- terminal-native
- open source
- durable sessions and work items
- policy-aware tools
- evidence and verification built in
- extensible with hooks/skills/agents
- bring your own model/provider

Avoid trying to out-Factory Factory on enterprise platform breadth. Instead, own the local-first trust/runtime category.

Possible tagline options:

- "A local-first coding agent with durable work built in."
- "The inspectable coding agent for real software work."
- "Open-source agent runtime for coding, verification, and durable work."
- "Claude Code-style speed, with local evidence, policy, and work tracking."

### Commercial path later

If imp gains traction, monetize around things that naturally cost money or require polish/support:

- hosted sync for imp-work and sessions
- team policy/admin
- Slack/GitHub/Linear hosted adapters
- workflowged remote workers
- private extension/agent registry
- enterprise support
- signed builds and auto-update channel

Keep the local core useful and trustworthy.

## Suggested priority order

1. `.imp/agents` custom agents
2. `/review` workflow
3. MCP stdio/HTTP foundation
4. hooks UX refresh
5. Work Control over imp-work
6. ACP adapter
7. Slack adapter
8. policy UI/audit refinements throughout

Policy is listed last only as a separate UX surface. Enforcement should be woven into every item above from the beginning.
