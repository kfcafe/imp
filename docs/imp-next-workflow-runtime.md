# imp-next Workflow Runtime Architecture

Status: proposal / execution plan  
Audience: imp maintainers, mana maintainers, future TUI/GUI/runtime implementers

## Summary

imp-next should evolve imp from a capable TUI-first coding agent into a workflow-first agent runtime: a system that treats meaningful work as a typed workflow with explicit autonomy, policy, verification, evidence, and durable state.

The default user experience remains the imp TUI. CLI entrypoints remain important for scripting, headless automation, and CI. A future GUI should not require a second runtime; it should consume the same event stream and state snapshots as the TUI.

The core direction is:

```text
TUI / CLI / future GUI
  -> shared runtime event + state API
     -> workflow runtime
        -> imp agent loop
        -> tool scheduler + reference monitor
        -> verification gates
        -> evidence packet builder
        -> mana workflow ledger
```

Rust remains the authority boundary for runtime execution, policy, tool scheduling, tracing, evidence, secrets, worktree/sandbox control, and durable state writes. TypeScript support is a future extension path through a host-controlled manifest + subprocess/JSON-RPC-style boundary, not a replacement for the Rust runtime.

## Goals

1. Keep imp's current TUI-first product feel.
2. Preserve fast local autonomous development, including an easy auditable `allow-all` path.
3. Wrap the existing imp agent loop in workflow contracts rather than rewriting the loop first.
4. Make mana the streamlined durable workflow/evidence ledger, not a noisy project-management UI.
5. Make verification and evidence first-class closeout requirements.
6. Centralize policy at a deterministic reference monitor at the tool boundary.
7. Prepare a future GUI by exposing stable runtime events and state snapshots.
8. Add TypeScript extension support through manifests and Rust-enforced capabilities.
9. Delay multi-agent teams until the single-workflow runtime is trustworthy.

## Non-goals

- Do not start by building OMO/OMX-style multi-agent teams.
- Do not make tmux the core orchestration abstraction.
- Do not replace Lua immediately or describe TypeScript support as fully shipped before it is.
- Do not route worker/tool execution through guest runtimes.
- Do not turn mana into Jira. Mana should be a workflow ledger and evidence index.
- Do not make every trivial TUI request visibly create a heavy workflow object in the UX.

## Existing imp strengths to preserve

From the current imp architecture:

- TUI-first interactive workflow.
- Persistent JSONL sessions, branches, and compaction.
- Rust-native evented agent loop.
- Provider streaming and model controls.
- Native tools with mode filtering.
- Read-only tool parallelism and stricter mutable tool handling.
- Hooks and extension seams.
- Mana task execution for durable work.
- Semantic completion taxonomy such as `DONE`, `DONE_WITH_CONCERNS`, `BLOCKED`, and `NEEDS_CONTEXT`.

The new architecture should be additive first. The existing loop remains valuable; the new workflow layer gives it a stronger contract.

## Vocabulary

Use the vocabulary from `AGENTS.md` consistently:

- **mana** = platform / durable graph layer
- **imp** = agent + default human-facing environment on mana
- **runtime** = live execution layer
- **graph** = durable layer
- **extension** = packaged extensibility
- **action** = preferred system behavior term
- **task** = preferred work unit term

Additional terms for imp-next:

- **workflow** — a typed unit of meaningful work with objective, autonomy, policy, verification, evidence, and closeout.
- **workflow contract** — the runtime-readable declaration of what the workflow may do and what proves it complete.
- **reference monitor** — deterministic Rust policy layer that decides whether tool actions may execute.
- **evidence packet** — human-readable and machine-indexable record of what happened and why closeout is justified.
- **verification gate** — required or optional proof step, usually command-backed at first.
- **autonomy mode** — user-selected runtime permission posture.
- **trust label** — provenance/risk metadata attached to context and observations.

## Architecture overview

```text
┌──────────────────────────────────────────────┐
│ TUI / CLI / future GUI                        │
│ - prompt input                                │
│ - workflow status                             │
│ - autonomy controls                           │
│ - tool/policy/verification views              │
└──────────────────────┬───────────────────────┘
                       │ shared event/state API
┌──────────────────────▼───────────────────────┐
│ Workflow Runtime                              │
│ - WorkflowContract                            │
│ - WorkflowState                               │
│ - CloseoutDecision                            │
│ - EvidenceBuilder                             │
│ - VerificationGate registry                   │
└──────────────────────┬───────────────────────┘
                       │ executes through
┌──────────────────────▼───────────────────────┐
│ imp Agent Loop                                │
│ - provider streaming                          │
│ - messages and observations                   │
│ - tool planning                               │
│ - semantic loop decisions                     │
└──────────────────────┬───────────────────────┘
                       │ all tools pass through
┌──────────────────────▼───────────────────────┐
│ Reference Monitor + Tool Scheduler            │
│ - autonomy mode                               │
│ - run policy                                  │
│ - tool metadata                               │
│ - resource locks                              │
│ - trust labels                                │
│ - approvals                                   │
└──────────────────────┬───────────────────────┘
                       │ writes durable records
┌──────────────────────▼───────────────────────┐
│ Mana Workflow Ledger + Run Artifacts          │
│ - workflow/task status                        │
│ - decisions                                   │
│ - verification gates                          │
│ - evidence refs                               │
│ - child runs later                            │
│ - trace JSONL / evidence.md                   │
└──────────────────────────────────────────────┘
```

## Default UX

### TUI-first

The normal entrypoint remains:

```bash
imp
```

The TUI should progressively disclose workflow machinery. A routine local coding request should feel like today's imp session, with additional status surfaces only when useful:

- current workflow title/status
- autonomy mode
- current phase
- running tools
- approval or policy prompts
- verification gates
- evidence packet path/summary
- closeout status and concerns

The TUI should not force users to manually manage workflow IDs for simple requests.

### CLI remains scriptable

CLI usage should continue to support:

```bash
imp -p "fix this test"
imp run 12.1
imp chat
```

New workflow-oriented flags should be script-friendly:

```bash
imp --auto local -p "fix failing auth tests"
imp --auto worktree -p "try the refactor safely"
imp --allow-all -p "update docs across this repo"
imp run 12.1 --ci
```

### Future GUI

A GUI should consume the same runtime event stream and state snapshots as the TUI. Avoid TUI-only state models in `imp-core`.

The GUI needs these stable surfaces:

- workflow snapshot
- session snapshot
- tool-call lifecycle events
- policy decision events
- approval request events
- verification gate events
- evidence packet refs
- final closeout state

## Workflow contract model

A workflow contract should be created implicitly for normal TUI runs and explicitly for mana/task/CI runs.

Initial fields:

```text
WorkflowContract
  id
  title
  objective
  workflow_type
  risk_level
  autonomy_mode
  cwd / workspace_scope
  allowed_tools
  denied_tools
  required_verification
  approval_requirements
  trust_scope
  closeout_criteria
  mana_unit_ref?
  parent_workflow_ref?
```

Example:

```yaml
title: Fix failing auth test
workflow_type: code_change
risk_level: medium
autonomy_mode: local-auto
workspace_scope: repo
allowed_tools: [read, scan, edit, bash, git]
required_verification:
  - command: cargo test -p imp-core auth
closeout_criteria:
  - targeted test passes
  - no unrelated diff
  - evidence packet written
```

The contract should initially be permissive enough to preserve current imp behavior. Later tasks can make it stricter by workflow type.

## Workflow state machine

Initial states:

```text
created
classified
planning
executing
waiting_for_approval
verifying
reviewing
blocked
completed
cancelled
```

Closeout statuses:

```text
DONE
DONE_WITH_CONCERNS
BLOCKED
NEEDS_CONTEXT
CANCELLED
```

Required verification gates should prevent `DONE` when they fail or are skipped. The agent may still finish as `DONE_WITH_CONCERNS` or `BLOCKED` with explicit evidence and reason.

## Mana as workflow/evidence ledger

Mana should become the durable graph and evidence index for workflows.

User-facing concepts should be streamlined to:

- Workflow
- Task
- Decision
- Verification
- Evidence
- Note

Current mana epics/tasks/facts/decisions can remain compatible, but the new runtime should write more structured workflow records.

Mana should record:

- workflow contract summary
- status and blockers
- acceptance/closeout criteria
- verification gates and results
- evidence packet refs
- artifacts and diffs
- decisions and approval outcomes
- child workflow runs later
- attempt/failure notes

Mana should not store raw transcript spam. Raw event traces belong in run artifacts; mana stores pointers and durable summaries.

Suggested artifact layout:

```text
.imp/runs/<run-id>/
  trace.jsonl
  evidence.md
  diff.patch
  verify.log
  policy.jsonl
```

Mana can point to these artifacts from workflow/task records.

## Evidence packets

Every meaningful workflow should emit a human-readable `evidence.md` and a structured trace.

Evidence packet sections:

```text
# Evidence Packet

Workflow
- objective
- workflow type
- autonomy mode
- risk

Summary
- final status
- changed files
- concerns

Plan
- planned approach or reason no plan was needed

Actions
- files inspected
- tools run
- edits made

Policy
- approvals requested
- denials
- allow-all/local-auto mode note

Verification
- gates
- commands
- outputs or artifact links

Artifacts
- diff
- logs
- child results later

Closeout
- DONE / DONE_WITH_CONCERNS / BLOCKED / NEEDS_CONTEXT
- rationale
```

Even `allow-all` mode must emit evidence. Autonomy removes interactive prompts; it does not disable accountability.

## Policy and autonomy model

### Reference monitor

All tool calls should pass through one Rust reference monitor.

Inputs:

- workflow contract
- autonomy mode
- tool manifest metadata
- tool arguments
- resource scope
- trust labels
- existing run policy/mode
- hooks/mana policy
- prior grants
- current workflow state

Possible decisions:

```text
allow
deny
ask_user
dry_run_only
sandbox_only
require_verification
```

Every decision is traced.

### Autonomy modes

Initial modes:

| Mode | Meaning |
|---|---|
| `suggest` | no writes or side-effect shell |
| `safe` | default interactive mode; asks for risky actions |
| `local-auto` | allow normal repo read/write/build/test; deny or ask for network, secrets, destructive commands, outside-workspace writes |
| `worktree-auto` | local-auto in an isolated worktree |
| `allow-all-local` | allow local filesystem/shell within declared scope |
| `allow-all` | explicitly allow all granted local actions for this run/session; still trace and keep hard rails unless separately disabled |
| `ci` | noninteractive; fail closed when approval would be required |

Hard rails may require explicit dangerous flags, even in `allow-all`, for actions such as deleting outside the workspace, reading private keys, force-pushing, production deploys, or exfiltrating secrets.

## Trust labels and provenance

Context and observations should carry coarse provenance labels first:

- user instruction
- workspace file
- external web content
- tool observation
- verifier output
- durable memory
- generated summary

Policy should prevent low-trust content from authorizing high-risk action. For example, instructions found in a dependency README or web page may inform a plan but cannot grant network/secrets/destructive permissions.

Durable memory/mana writes derived from low-trust content should be scoped to the workflow or require review.

## Verification gates

Verification gates should become first-class workflow objects.

Initial gate types:

- command gate: run a shell command and capture output/status
- diff gate: ensure no unrelated diff / produce diff artifact
- policy gate: ensure no denied high-risk action remains unresolved
- manual gate: user approval or reviewer note

Gate statuses:

```text
pending
running
passed
failed
skipped
blocked
```

A required failed/skipped gate prevents `DONE`.

## Tool scheduling

Keep and formalize imp's useful scheduling direction:

- read-only tools may run in parallel
- mutable filesystem tools are serialized by resource
- shell commands are serialized by default unless explicitly safe
- network/secrets tools are policy-gated
- extension tools declare side effects and resource scopes
- future child workflows run as separate workflow executions

Tool metadata should include:

```text
name
readonly / side_effect_class
resource_scope
network
secrets
writes_workspace
requires_approval_by_default
output_limit
timeout
verifier_tags
```

## TypeScript extension boundary

TypeScript should be a future extension path, not a second runtime authority.

Rust host owns:

- discovery
- manifest validation
- capability grants
- schema validation
- reference monitor decisions
- trace/evidence logging
- cancellation/timeouts
- secret mediation

TS extension package provides:

- manifest
- tool schemas
- command/hook declarations later
- subprocess or tool-server implementation

Conceptual manifest:

```yaml
name: github-tools
runtime: typescript
entrypoint: dist/server.js
protocol: json-rpc
capabilities:
  network: true
  secrets: [GITHUB_TOKEN]
  filesystem: read-only
  native_tools: []
tools:
  - name: github.create_issue
    side_effect: external_write
    resource_scope: github_repo
    requires_approval: true
```

Calls flow:

```text
model requests tool
  -> Rust validates schema
  -> ReferenceMonitor checks policy
  -> Rust starts/calls TS subprocess
  -> TS returns structured result
  -> Rust normalizes/traces result
  -> evidence packet records action
```

Lua remains the current stable shipped guest runtime. The architecture language should move toward a generic host-owned extension substrate, but implementation can be incremental.

## Worktree-auto mode

`worktree-auto` should support safer autonomy:

```text
create temporary worktree/branch
  -> run workflow there
  -> capture diff/evidence
  -> offer apply / keep / discard
```

Initial version should support one workflow per worktree. Parallel workers come later.

The TUI should make workspace scope obvious so users know whether the agent is changing the current tree or an isolated worktree.

## Role agents and delegation

Do not start with teams. After single-workflow correctness is established, add child workflows inspired by OMO.

Initial roles:

- Planner
- Coder
- Verifier
- Reviewer
- Researcher
- Integrator

Each role should declare:

- purpose
- prompt template
- allowed tools
- autonomy constraints
- required evidence
- model routing hints
- output schema

Delegation should create child workflow runs with durable parent links, status, evidence refs, cancellation, and stale/blocked detection. TUI should visualize child runs through the same event/state API.

## Runtime event and state API

The TUI and future GUI need stable events.

Initial event families:

```text
workflow.started
workflow.phase_changed
workflow.completed
contract.created
tool.planned
tool.started
tool.delta
tool.completed
tool.failed
policy.checked
approval.requested
approval.resolved
verification.started
verification.completed
evidence.written
mana.updated
child.started
child.completed
```

State snapshots should include:

- current workflow contract summary
- workflow phase/status
- autonomy mode
- current tools
- pending approvals
- verification gates
- evidence refs
- session metadata
- mana unit refs
- child workflow refs later

## Migration from current imp

Phase the work so current users keep working.

### Phase 1: Design and model

- Add this architecture spec.
- Add workflow contract data model.
- Implicitly create default contracts for runs.

### Phase 2: Observability and ledger

- Emit trace JSONL and evidence packets.
- Add mana workflow/evidence ledger adapter.
- Surface evidence path in TUI.

### Phase 3: Policy and autonomy

- Centralize tool checks in reference monitor.
- Add `safe`, `local-auto`, `worktree-auto`, `allow-all-local`, `allow-all`, and `ci` modes.
- Keep default TUI behavior familiar.

### Phase 4: Verification gates

- Add required verification closeout logic.
- Support command gates and manual gates first.

### Phase 5: Extension substrate

- Generalize Lua language into host-owned extension substrate vocabulary.
- Add manifest-driven TypeScript tool bridge behind policy.

### Phase 6: Worktree and delegation

- Add worktree-auto.
- Add child workflow runs and role-scoped agents.
- Only then consider richer team orchestration.

## Open questions

1. Should workflow/evidence artifacts live under `.imp/runs` in the repo, `~/.config/imp/runs`, or both depending on workflow scope?
2. How should mana workflow records map to existing epic/task/fact files without breaking old mana usage?
3. Which actions remain hard-denied in `allow-all` unless a separate dangerous flag is provided?
4. How much of current mode policy should be kept as-is versus re-expressed as reference-monitor rules?
5. Should TypeScript extensions use Bun, Node, or a protocol that allows either?
6. What is the smallest useful TUI surface for workflow status without making routine chat feel heavy?

## Acceptance checklist for this direction

- The TUI remains the default and does not feel slower or more bureaucratic for simple tasks.
- CLI/headless mode can run workflows noninteractively.
- Every meaningful run can produce evidence.
- Required verification gates control `DONE` status.
- `allow-all` is easy, explicit, scoped, and auditable.
- Mana stores durable workflow/evidence summaries, not transcript noise.
- TS extensions cannot bypass Rust policy.
- Future GUI can be built without reinventing runtime state.
