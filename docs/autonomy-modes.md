# Autonomy modes

Autonomy mode controls how much imp may do without stopping for explicit human
approval. It is orthogonal to `AgentMode` and `RunPolicy`:

- `AgentMode` describes the agent role and broad tool/action surface.
- `RunPolicy` constrains a specific run with allow/deny lists and path rules.
- `AutonomyMode` decides whether an otherwise-available action is automatic,
  requires approval, is dry-run/sandbox-only, requires verification, or is hard
  denied.

The default remains conservative and familiar: current interactive safe behavior
must not get more permissive unless the user explicitly chooses a more autonomous
mode.

## Modes at a glance

| Mode | Intent | Default behavior |
| --- | --- | --- |
| `suggest` | planning/advice only | Read and inspect; propose changes/commands instead of executing mutable actions. |
| `safe` | current default | Preserve current imp behavior: normal reads/searches, explicit tool policy checks, no new broad grants. |
| `local-auto` | unattended local implementation | May edit workspace and run normal local checks; approval for high-risk shell/network/secrets/outside-scope actions. |
| `worktree-auto` | unattended isolated implementation | Requires an isolated worktree/run cwd. Until 394.9 creates worktrees automatically, this mode returns `SandboxOnly` / `autonomy_worktree_required` unless an existing `WorkspaceScope::Worktree` context is supplied. Use `local-auto` for current-workspace unattended execution. |
| `allow-all-local` | easy auditable local allow-all | Allow local workspace writes and shell commands with audit/evidence; still hard-deny outside-workspace writes, secrets exfiltration, network mutation, and destructive system actions unless separately granted. |
| `allow-all` | easy auditable broad allow-all | Allow most tools/actions with trace/evidence, including network when tools support it; hard rails still apply. |
| `ci` | noninteractive reproducible automation | No user prompts; allow declared commands/gates only; fail closed on missing approvals or ambiguous policy. |

## Hard rails

Hard rails are denied regardless of autonomy mode unless a later explicit
`dangerous-grant` mechanism says otherwise. They are intentionally stronger than
`allow-all`.

Current hard rails:

1. **No secret exfiltration.** Tools must not reveal secret values in chat,
   traces, evidence, command output summaries, extension payloads, or network
   requests. Secret use must stay mediated by host-owned secret plumbing.
2. **No destructive system commands by default.** Examples: `rm -rf /`, disk
   formatting, killing unrelated processes, changing system auth/keychain state,
   changing global shell/profile config, or modifying files clearly outside the
   project/worktree without explicit dangerous grant.
3. **No outside-workspace writes by default.** `allow-all-local` remains scoped to
   the project/worktree. `allow-all` may read broader context when configured, but
   writes outside the declared workspace still require a dangerous grant.
4. **No credential or deployment mutation by accident.** Pushing, publishing,
   deploying, rotating credentials, modifying DNS, billing, package ownership,
   or production state requires a dedicated capability/approval even in
   `allow-all`.
5. **No policy/evidence bypass.** All modes, including allow-all variants, must
   emit `policy.checked` trace events, trace/evidence artifacts, and final
   evidence refs when artifact writing is available.
6. **No extension policy bypass.** Extensions cannot self-authorize. The Rust
   runtime must apply ReferenceMonitor decisions before extension execution.
7. **No silent fallback from isolated to non-isolated execution.** If
   `worktree-auto` requires an isolated worktree and one cannot be created or
   supplied, imp returns `autonomy_worktree_required` instead of running in the
   current workspace.

## Dangerous grants above allow-all

`allow-all` and `allow-all-local` are autonomy modes, not root access. A separate
future dangerous-grant layer is required for exceptional actions that are too
risky to permit silently. Until explicit grant plumbing exists, these rails fail
closed through ReferenceMonitor `DangerousGrant` policy records.

Dangerous rails currently classified in code:

- `dangerous_secret_exfiltration` — exposing secret values to chat, logs,
  network, traces, or extension payloads
- `dangerous_private_key_read` — reading SSH/private keys or equivalent local
  credentials
- `dangerous_outside_workspace_destructive_write` — deleting or destructively
  modifying files outside the declared workspace/worktree
- `dangerous_force_push` — `git push --force` / `--force-with-lease`
- `dangerous_global_git_config_mutation` — changing global git identity, hooks,
  credential helpers, or signing configuration
- `dangerous_production_deploy` — changing live deployed/user-facing production
  state
- `dangerous_cloud_resource_deletion` — deleting cloud resources, databases,
  buckets, DNS zones, or similar infrastructure
- `dangerous_audit_log_disable` — disabling or deleting audit/evidence/policy
  logs

A dangerous grant, when implemented, must be explicit, scoped, time-limited or
run-limited, recorded as policy/evidence, and unavailable to extension code as a
self-service bypass. Headless `ci` must fail closed unless the grant is declared
in trusted configuration.

## ReferenceMonitor decision mapping

The monitor should combine `AgentMode`, `RunPolicy`, tool metadata, resource
scope, trust/provenance context, and autonomy mode. If any stricter layer denies,
the action is denied. Autonomy can require approval or verification; it cannot
turn an `AgentMode`/`RunPolicy` deny into allow.

| Mode | Read/search | Workspace write | Shell/process | Network | Secrets | Outside-workspace write | Destructive/deploy/publish | Verification |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `suggest` | Allow | `AskUser`/suggest patch only | `AskUser`/suggest command only | `AskUser` unless read-only web lookup is explicitly safe | Deny secret reveal/use unless mediated | Deny | Deny | Optional, mostly suggestions |
| `safe` | Preserve current behavior | Preserve current behavior | Preserve current behavior | Preserve current behavior | Preserve current behavior + hard rails | Existing policy/path checks | Hard rails | Existing behavior |
| `local-auto` | Allow | Allow inside workspace if `RunPolicy` allows | Allow common local dev commands; `AskUser` for risky/destructive commands | `AskUser` for network mutation; read-only fetch may allow when tool policy allows | Deny reveal; mediated use may require approval | Deny/AskUser depending future grant | AskUser or Deny | `RequireVerification` after code changes |
| `worktree-auto` | Allow | Allow inside isolated worktree | Allow common local dev commands in worktree | Same as `local-auto` | Same as `local-auto` | Deny outside worktree | AskUser or Deny | `RequireVerification` before apply/keep |
| `allow-all-local` | Allow | Allow inside workspace/worktree | Allow local commands with hard rails | AskUser/deny network mutation unless local-only or explicit network capability | Deny reveal; mediated use only | Deny | Deny without dangerous grant | Require evidence; verification recommended/required for code changes |
| `allow-all` | Allow | Allow within configured scope | Allow with hard rails | Allow when tool/policy permits, but block exfiltration/production mutation | Deny reveal; mediated use only | AskUser/Deny until dangerous grant exists | Deny without dangerous grant | Require evidence; verification recommended/required for code changes |
| `ci` | Allow declared reads | Allow declared generated/workspace paths only | Allow declared commands/gates only | Deny unless declared | Deny reveal; mediated CI secrets only | Deny | Deny | Required gates must pass or closeout fails |

Decision vocabulary:

- `Allow`: action proceeds.
- `Deny`: action cannot proceed.
- `AskUser`: action needs approval; in headless/CI this fails closed unless a
  predeclared approval exists.
- `DryRunOnly`: only dry-run form may execute.
- `SandboxOnly`: only sandboxed/worktree-isolated form may execute.
- `RequireVerification`: action may proceed but creates a workflow obligation
  before `DONE`/closeout.

## Current behavior compatibility

`safe` is the compatibility mode. It must preserve current defaults:

- Existing `AgentMode::allows_tool` behavior remains authoritative.
- Existing `RunPolicy::check_tool` and `RunPolicy::check_write_path` behavior
  remains authoritative.
- Existing bash-equivalent mana blocking remains in force.
- Existing repeated-call loop blocking remains in force.
- Existing schema validation and after-write guardrails remain in force.
- Existing TUI interactions should not feel more bureaucratic for routine work.

The 394.5 ReferenceMonitor currently routes AgentMode/RunPolicy preflight checks
and records policy events. 394.6 should add autonomy decisions around that, not
replace those checks.

## Resource scope semantics

Autonomy decisions need a declared resource scope:

- `workspace`: current project root or configured run cwd
- `worktree`: isolated git worktree created for a workflow run
- `outside-workspace`: any path outside the declared workspace/worktree
- `network-read`: read-only HTTP/API lookup
- `network-mutate`: POST/PUT/DELETE, deployments, DNS, billing, publishing
- `secret-use`: host-mediated use of a secret value
- `secret-reveal`: exposing the secret value to model/chat/log/network output
- `system`: OS/global/user-home mutation outside project ownership
- `production`: any action changing live deployed/user-facing state

`allow-all-local` is intentionally not equivalent to unrestricted machine access;
it means local project autonomy with hard rails and evidence.

## TUI presentation

The TUI should display autonomy mode compactly, not as a modal-heavy workflow:

- status item: `autonomy: safe` / `local-auto` / `allow-all-local`
- run closeout/evidence should include selected mode
- approval prompts should state mode, tool, resource scope, and reason code
- switching to allow-all variants should be explicit and visually distinct
- if `worktree-auto` is selected before 394.9 worktree creation support lands,
  the TUI/CLI should show `autonomy_worktree_required` and suggest `local-auto`
  for current-workspace execution or an explicit existing worktree context

Default TUI startup remains `safe` unless config/CLI explicitly sets another
mode.

## CLI/config presentation

CLI/config should support explicit mode selection with names matching this doc:

```sh
imp --autonomy safe "fix the test"
imp --autonomy local-auto "make the obvious change and run tests"
imp --autonomy allow-all-local "do the local refactor, keep evidence"
imp --autonomy ci --verify "cargo test -p imp-core"
```

Config should also allow a default autonomy mode, but CLI/run selection should
win over config. Headless `ci` mode must not block forever waiting for TUI
approval.

## Audit and evidence behavior

All modes emit audit data when artifact writing is available:

- selected autonomy mode in workflow contract, trace, and evidence
- `policy.checked` for meaningful monitor decisions
- approval requests/resolutions once approval UX exists
- verification obligations and results once 394.7 lands
- hard-rail denials with stable reason codes

Allow-all modes are allowed to reduce friction, not accountability. They must be
more auditable than interactive safe mode, not less.

## Implementation order

1. Canonicalize `AutonomyMode` parsing/display for the names above.
2. Add compatibility tests proving default `safe` behavior does not change.
3. Add ReferenceMonitor autonomy mapping for decisions that are pure policy data.
4. Add CLI/config/TUI selection plumbing.
5. Record autonomy mode in evidence/trace where not already present.
6. Add hard-rail dangerous grant design separately in 394.6.7.
7. Let 394.9 implement real `worktree-auto` isolation.

## User-facing examples

### Safe default in the TUI

Default TUI sessions start in `safe` mode:

```text
/autonomy help
/autonomy safe
```

Safe mode is the compatibility baseline. It keeps current TUI expectations:
normal reads, edits, shell/tool use, existing `AgentMode` limits, `RunPolicy`
constraints, repeated-call protection, bash-equivalent mana blocking, schema
validation, and guardrails. Use safe mode when you want the agent to behave like
imp historically behaved.

The TUI status area shows the selected mode as an `autonomy` status item. The
`/autonomy <mode>` command changes future agent starts in the session. High-risk
modes are shown loudly (`ALLOW-ALL`, `ALLOW-ALL-LOCAL`) and print a reminder that
hard rails and evidence remain enabled.

### Local autonomous development

Use `local-auto` for unattended local implementation in the current workspace:

```text
/autonomy local-auto
fix the failing parser test and run the narrow test
```

CLI equivalent:

```sh
imp --autonomy local-auto "fix the failing parser test and run the narrow test"
```

`local-auto` may read, edit, and run ordinary local development commands in the
workspace when `AgentMode` and `RunPolicy` allow them. It does not silently allow
network mutation, direct secret access, outside-workspace writes, production
changes, or dangerous rails. Code-changing runs should still produce evidence and
verification context.

### Isolated development with worktree-auto

`worktree-auto` is for autonomous work in an isolated git worktree:

```sh
imp --autonomy worktree-auto "implement the refactor in an isolated worktree"
```

Until 394.9 creates/manages worktrees automatically, `worktree-auto` fails closed
with `autonomy_worktree_required` unless an existing `WorkspaceScope::Worktree`
context is supplied by the runtime. Use `local-auto` if you intentionally want the
current workspace to be modified now.

Maintainer note: never silently downgrade `worktree-auto` to current-workspace
execution. That would violate user intent and make reviews unsafe.

### Explicit local allow-all

`allow-all-local` is the easy auditable “please stop asking for routine local
permission” mode:

```text
/autonomy allow-all-local
make the mechanical rename, update call sites, run tests
```

CLI equivalent:

```sh
imp --autonomy allow-all-local "do the local cleanup and keep evidence"
```

This mode allows local workspace/worktree actions with fewer prompts, but it is
not root access. It still blocks or requires approval for network actions,
outside-workspace writes, secret-sensitive actions, production/deploy/publish
changes, and dangerous grants. It should be more auditable than safe mode, not
less: traces, `policy.checked`, run artifacts, and evidence stay enabled.

### Explicit broad allow-all

`allow-all` is broader than `allow-all-local` and should be visually distinct in
UI and logs:

```sh
imp --autonomy allow-all "complete the integration task; keep an audit trail"
```

Use it only when the user explicitly wants high autonomy. Even here, hard rails
remain above autonomy: secret exfiltration, private-key reads, force-push,
production deploys, cloud deletion, audit-log disabling, and destructive
outside-workspace operations require a future explicit dangerous grant and fail
closed today.

### CI fail-closed mode

Use `ci` for noninteractive automation:

```sh
imp --autonomy ci --max-turns 8 "run the declared verification and report status"
```

CI must not wait on a TUI approval prompt. Any action that would require approval
fails closed unless future trusted configuration predeclares it. Network, direct
secret access, outside-workspace writes, and ambiguous approval requirements are
denied by default.

### Suggest/review-only mode

Use `suggest` when the user wants a plan or review without side effects:

```sh
imp --autonomy suggest "review this design and suggest the smallest patch"
```

Suggest mode permits read/search/ask-user style interactions and denies
side-effecting tools. It should propose commands or patches instead of executing
mutable actions.

## Maintainer semantics

Autonomy is not a substitute for policy. The decision stack is:

1. `AgentMode` role restrictions.
2. `RunPolicy` tool/path restrictions.
3. ReferenceMonitor autonomy decisions.
4. Hard rails / dangerous grants.
5. Tool execution.
6. Trace/evidence/verification closeout.

A more autonomous mode cannot override a stricter `AgentMode` or `RunPolicy`
deny. `allow-all` means “reduce prompts where policy allows,” not “ignore the
monitor.”

When adding or changing a tool, maintainers must keep `Tool::policy_metadata()`
accurate. If a tool can write, run processes, touch network, access secrets, or
operate as an extension, metadata must say so. The autonomy map relies on this
classification.

When changing autonomy behavior, add tests in three places where applicable:

- `reference_monitor` tests for pure policy decisions
- tool-execution tests for preserved user-facing behavior
- TUI/CLI tests for selection and display plumbing

Do not weaken default `safe` compatibility. If a new mode needs stronger behavior,
make it opt-in and auditable.

## Audit and evidence semantics

Autonomy removes prompts; it does not remove accountability.

All modes should preserve:

- workflow contract autonomy mode
- `policy.checked` trace events for meaningful policy decisions
- run artifacts under `.imp/runs/<run-id>/`
- `evidence.md` summaries, including autonomy mode and hard-rail notes
- stable reason codes for denials and non-allow decisions

Allow-all modes should especially leave a useful audit trail. If future code skips
trace/evidence in allow-all for convenience, treat that as a bug.

See also:

- `docs/reference-monitor-policy.md`
- `docs/trace-and-evidence-format.md`
- `docs/imp-next-workflow-runtime.md`
