# Verification gates

Verification gates are first-class workflow closeout requirements. They turn
"I ran the tests" from an informal chat claim into structured runtime state,
artifacts, trace events, evidence summaries, and closeout policy.

A gate answers three questions:

1. What verification was required or requested?
2. What happened when imp attempted it?
3. Does the result allow the run to close as `RunFinalStatus::Done`?

## Model

```rust
struct VerificationGate {
    id: String,
    name: String,
    kind: VerificationGateKind,
    requirement: VerificationRequirement,
    status: VerificationGateStatus,
    command: Option<VerificationCommand>,
    artifacts: Vec<VerificationArtifactRef>,
    source: VerificationGateSource,
    reason: Option<String>,
}

enum VerificationGateKind {
    Command,
    Diff,
    Policy,
    Manual,
    Custom(String),
}

enum VerificationRequirement {
    Required,
    Optional,
    Advisory,
}

enum VerificationGateStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
    Blocked,
}
```

Initial implementation should support command gates end-to-end and keep diff,
policy, manual, and custom gate kinds extensible as structured records.

## Gate kinds

### Command

A command gate runs a local command in a declared cwd and captures exit status,
stdout/stderr summaries, timing, and log artifacts. Examples:

- `cargo test -p imp-core reference_monitor`
- `cargo fmt --package imp-core --check`
- `npm test`
- `zig build test`

Command gates are the first executable implementation target.

### Diff

A diff gate checks whether the resulting workspace diff satisfies a condition:
non-empty diff, no generated files, no changes outside scope, or matching an
expected patch. Diff gates can start as manual/structured records and later gain
runners.

### Policy

A policy gate checks runtime policy state: no dangerous grants, no unresolved
approval requests, all required `policy.checked` decisions allowed or justified,
or no high-risk allow-all actions without evidence.

### Manual

Manual gates represent a human-required verification step. They cannot pass
without explicit user/maintainer input. In headless `ci`, unresolved manual gates
fail closed as `Blocked` or `Skipped` depending source.

### Custom

Custom gates preserve extensibility for future extension-provided verifiers. They
must still produce typed status, artifacts, and evidence summaries.

## Requirement semantics

`VerificationRequirement` determines closeout impact:

- `Required`: must pass before the run can close as `RunFinalStatus::Done`.
- `Optional`: should run when available; failure becomes a concern but does not
  necessarily block `Done` unless workflow policy says so.
- `Advisory`: informational only; failure is evidence, not closeout policy.

Required gates are the important closeout boundary.

## Status semantics

| Status | Meaning | Closeout effect when required |
| --- | --- | --- |
| `Pending` | Gate exists but has not run yet. | Prevents `Done`; close as `Blocked` or `DoneWithConcerns` with explicit reason. |
| `Running` | Gate is currently executing. | Cannot close final status except cancellation/failure. |
| `Passed` | Gate completed successfully. | Allows `Done` if all other required gates passed. |
| `Failed` | Gate ran and failed. | Prevents `Done`; usually `DoneWithConcerns` if work is otherwise complete, `Blocked` if failure prevents confidence. |
| `Skipped` | Gate was intentionally skipped. | Prevents `Done` unless explicitly non-required; required skip is at least `DoneWithConcerns`. |
| `Blocked` | Gate could not run because of missing command, dependency, approval, cwd, timeout, or policy. | Prevents `Done`; usually `Blocked`. |

## Mapping to `RunFinalStatus`

`RunFinalStatus` remains the user-facing closeout status. Verification gates refine
whether `Done` is legal.

Rules for required gates:

1. All required gates `Passed` -> `RunFinalStatus::Done` may be used if the agent
   otherwise believes work is complete.
2. Any required gate `Failed` -> never close as `Done`; use
   `RunFinalStatus::DoneWithConcerns` when the implementation is complete but
   verification failed, or `RunFinalStatus::Blocked` when the failure prevents
   progress/confidence.
3. Any required gate `Skipped` -> never close as `Done`; use
   `DoneWithConcerns` with a skipped-gate concern unless user explicitly accepts
   a different closeout.
4. Any required gate `Blocked` -> `RunFinalStatus::Blocked` unless the user
   explicitly asks to stop with concerns.
5. Any required gate `Pending`/`Running` at closeout -> closeout is incomplete;
   run the gate, ask, or report blocked.
6. Optional/advisory gate failures are evidence and may add concerns, but do not
   alone forbid `Done`.

This means verification gates are not merely log lines: they constrain closeout.
A run that changed code and has a required failing test gate should not report
`DONE`.

## Relationship to existing `loop_state.rs`

Existing loop state already defines the final user-visible statuses:

- `RunFinalStatus::Done`
- `RunFinalStatus::DoneWithConcerns`
- `RunFinalStatus::Blocked`
- `RunFinalStatus::NeedsUserInput`
- `RunFinalStatus::Cancelled`
- `RunFinalStatus::Failed`

Verification gates should be evaluated before producing `LoopDecision::Finish`.
When gate results downgrade closeout, the final status should include explicit
concerns/messages such as:

- `required verification failed: cargo test -p imp-core`
- `required verification skipped: formatter unavailable`
- `required verification blocked: command timed out`

`StopReason::ExecutionBlocked` is appropriate for required blocked gates.
`StopReason::WorkCompleted` with `DoneWithConcerns` is appropriate when work is
complete but verification failed/skipped.

## Gate sources

Gates may come from:

- workflow contract (`requires verification: cargo test ...`)
- mana task verify command
- user CLI/TUI input (`--verify`, future `/verify add ...`)
- inferred local project conventions (394.7.9; conservative only)
- policy/autonomy requirements (for example code changes in `local-auto`)
- extension manifests (future custom verifiers)

Source must be recorded so closeout can explain why a gate existed.

## Artifact refs

Each gate can produce artifacts. Initial command gates should write under the run
artifact directory from 394.4:

```text
.imp/runs/<run-id>/verification/<gate-id>/
  stdout.log
  stderr.log
  output.log        # optional combined log
  status.json
```

`VerificationArtifactRef` should include kind, path, optional summary, byte size
when cheap, and redaction/truncation notes. Evidence packets should link artifact
paths rather than inline large logs.

## Trace and evidence

Verification should emit trace/runtime events:

- `verification.started`
- `verification.output` or summarized output metadata
- `verification.finished`
- `verification.skipped`
- `verification.blocked`

Evidence packets should summarize:

- required/optional/advisory counts
- each gate name/kind/status
- command and cwd for command gates
- artifact refs
- closeout effect for failed/skipped/blocked required gates

This maps naturally onto `EvidencePacket.policy` or a future dedicated
`EvidencePacket.verification` section.

## TUI expectations

The TUI should show compact gate progress without taking over the chat:

```text
verify: cargo test -p imp-core … running
verify: cargo test -p imp-core … passed 12.4s
verify: cargo fmt --check … failed (see evidence)
```

Closeout UI should make required failures obvious:

- `DONE` unavailable while required gates are failed/blocked/pending.
- `DONE_WITH_CONCERNS` may be selected or reported with explicit failed/skipped
  gate reasons.
- `BLOCKED` should identify the gate and next action.

For headless/CI runs, unresolved required gates fail closed. The runtime must not
wait forever for TUI input.

## Relationship to autonomy modes

Autonomy can add verification obligations but cannot remove required gates.

- `safe`: existing behavior plus explicitly declared gates.
- `local-auto` / `worktree-auto`: code-changing runs should normally require at
  least one targeted verification gate when known.
- `allow-all-local` / `allow-all`: keep evidence and gate summaries especially
  visible; high autonomy should be more auditable, not less.
- `ci`: required gates must pass or closeout is not `Done`.

## Non-goals for the first implementation

- No broad, clever test inference. 394.7.9 specifies conservative inference.
- No remote verification service.
- No extension custom verifier execution until extension manifests exist.
- No weakening of existing final-status semantics.
- No large log inlining in chat/evidence.

## Implementation order

1. Define Rust gate/status/source/artifact types.
2. Implement command gate runner and artifact capture.
3. Emit verification events and evidence summaries.
4. Apply closeout enforcement before final `RunFinalStatus`.
5. Accept user/mana-provided gate declarations.
6. Add TUI progress/status rendering.
7. Add conservative inference later.

## Conservative gate inference strategy

Verification inference is intentionally limited. Imp should prefer explicit gates
from the user, mana task, workflow contract, or trusted config. Inference is a
fallback that can suggest or add low-risk checks only when confidence is high.

### Why inference is limited

Automatic verification can be expensive, flaky, destructive, or surprising:

- `cargo test` may run integration tests that need services or credentials.
- JS package scripts can execute arbitrary project code.
- `make test` may build containers, mutate fixtures, or call external systems.
- Monorepos often have many packages and expensive full-suite commands.
- Some projects require generated code, databases, or network resources first.

Therefore inference must never be framed as "the agent knows the right test". It
should be recorded as an inferred gate with a clear source/reason, and users must
be able to override or disable it.

### Inference modes

Inference should be controlled by an explicit setting, not hidden magic:

```text
verification.inference = "off" | "suggest" | "safe"
```

Recommended defaults:

- TUI/default `safe` autonomy: `suggest`
- `local-auto` / `worktree-auto`: `suggest` or `safe` only when project rules are
  cheap and obvious
- `allow-all-local` / `allow-all`: `suggest`; high autonomy must not expand into
  unbounded verification automatically
- `ci`: `off` unless trusted config declares gates

Semantics:

- `off`: no inferred gates.
- `suggest`: report candidate commands in evidence/TUI but do not execute them.
- `safe`: create optional or required gates only when conservative rules pass.

### Conservative rules

A gate may be inferred only if all of these are true:

1. The command is local to the workspace/worktree.
2. The command does not require network by default.
3. The command does not require secrets by default.
4. The command is not destructive and does not deploy/publish/push.
5. The command is bounded by a timeout and output capture limits.
6. The project root is unambiguous.
7. There is no explicit user/mana/workflow gate that already covers the same
   verification intent.
8. The inferred gate is marked with `VerificationGateSource::Inferred` and a
   reason explaining the rule.

If any rule is uncertain, infer a suggestion rather than an executable gate.

### Initial candidate rules

#### No project detected

When no recognizable project file exists (`Cargo.toml`, package manifest,
`flake.nix`, `zig.build`, etc.), infer nothing. Evidence may say:

```text
No verification gate inferred: project type not recognized. Use --verify <cmd>
to declare one explicitly.
```

#### Rust single crate or workspace

If the workspace root has a `Cargo.toml` and no explicit gate exists:

- prefer `cargo test --workspace --all-targets` only when a trusted project config
  explicitly opts in, because full workspace tests can be expensive
- otherwise suggest `cargo test` rather than executing it automatically
- if the changed path is inside one crate and package name can be read cheaply,
  suggest `cargo test -p <package>`
- use `cargo test -p <package> <known-test-filter>` only when the failing test or
  user prompt supplied the filter explicitly

Do not infer commands that require installed services, Docker, network, or
release builds unless trusted config declares them.

#### Rust formatter

`cargo fmt --check` is usually cheap, but it should still be inferred as optional
or suggested, not required, unless config says formatting is a required gate.

#### JavaScript/TypeScript

Do not execute `npm test`, `pnpm test`, `yarn test`, `bun test`, or package
scripts automatically from inference alone. Package scripts are arbitrary shell.
Suggest likely commands when a package manifest exists, and let users promote
with `--verify` or trusted config.

#### Zig/Odin/Go/etc.

Treat language-specific checks as suggestions unless a trusted project convention
file declares them. Prefer explicit gates over heuristics.

### Override and disable

Users can always provide explicit gates:

```sh
imp --verify "cargo test -p imp-core reference_monitor" "finish the policy slice"
```

Future config can disable inference:

```toml
[verification]
inference = "off"
```

Or declare trusted gates:

```toml
[[verification.gates]]
name = "core tests"
command = "cargo test -p imp-core"
required = true
```

Explicit user/mana/workflow gates take precedence over inferred gates. Inference
must not add duplicate gates when an explicit gate already exists.

### Recording inferred gates

Inferred gates must be auditable:

- `source = VerificationGateSource::Inferred`
- `reason = "detected Cargo.toml; suggested cargo test"` or similar
- `requirement = Optional` unless trusted config says Required
- trace/evidence should say whether the gate was executed, suggested, skipped, or
  disabled

Suggested-but-not-executed gates should not block closeout. Executed inferred
required gates should follow the normal closeout rules.

### Test plan if implemented

If inference moves beyond design, add tests for:

1. no-project directory -> no inferred gates
2. Rust `Cargo.toml` -> suggested or optional cargo gate depending mode/config
3. disabled inference -> no gates even when project files exist
4. explicit `--verify` present -> no duplicate inferred cargo gate
5. package scripts -> suggestions only, no automatic execution

Until these tests exist, inference should remain design-only or suggestion-only.

## User-facing UX examples

### Required cargo test gate

Use `--verify` to declare a required command gate:

```sh
imp --verify "cargo test -p imp-core reference_monitor" \
  "finish the reference monitor change"
```

The command is recorded as a required `VerificationGate`. During closeout, imp
runs it under the run artifact directory and writes logs to:

```text
.imp/runs/<run-id>/verification/cli-verify-1/
  stdout.log
  stderr.log
  status.json
```

If the command passes, the required gate allows `DONE` when the rest of the run is
complete. The evidence packet includes the gate status, command, exit code, and an
artifact ref.

### Optional lint gate

Optional gates are useful for checks that should be attempted but should not by
themselves block `DONE`:

```toml
[[verification.gates]]
name = "lint"
command = "cargo clippy --workspace --all-targets"
required = false
```

Optional failure is still evidence. It should appear in trace/evidence and may be
reported as a concern, but optional gates do not by themselves forbid `DONE`.

### Manual gate

Manual gates represent checks that only a human can complete:

```toml
[[verification.gates]]
name = "manual browser smoke"
kind = "manual"
required = true
```

A required manual gate remains pending until a user/maintainer records the result.
Pending required gates cannot close as `DONE`. In TUI, this should be visible as a
verification status/closeout blocker. In headless/CI, unresolved manual gates fail
closed as blocked or done-with-concerns depending workflow policy.

### Failed required gate

If a required gate runs and fails, imp should not report plain `DONE`:

```text
verify: cargo test -p imp-core … failed
```

Closeout mapping:

- implementation complete + required gate failed -> `DONE_WITH_CONCERNS`
- gate failure prevents confidence/progress -> `BLOCKED`

Evidence should include the failed command, exit code, summary, and artifact refs.
The user can inspect `stderr.log` / `stdout.log` / `status.json` under the run
verification directory.

### Skipped required gate

A required gate may be skipped only with a reason:

```text
required verification skipped: cargo fmt --check (formatter unavailable)
```

Skipped required gates cannot silently become `DONE`. They must produce at least
`DONE_WITH_CONCERNS` unless the user explicitly accepts a different closeout.
The skip reason belongs in trace/evidence.

### Blocked required gate

Blocked gates are stronger than failures. Examples:

- command missing (`cargo` not installed)
- timeout
- invalid cwd
- required approval unavailable in CI
- policy prevents the gate command

Blocked required gates force `BLOCKED` by default:

```text
BLOCKED — required verification blocked: unit tests (command timed out)
```

The next useful action should be explicit: install the tool, adjust timeout,
provide approval, or change the declared gate.

## TUI behavior

The TUI should keep verification visible but compact:

```text
verify: unit tests running required
verify: unit tests passed required
verify: fmt failed required blocks closeout
```

Current implementation consumes verification events and aggregates per-gate status
into the `verify` status item. Non-allowing closeout effects also emit a warning
message. Future closeout UI should make plain `DONE` unavailable while required
gates are pending/failed/skipped/blocked.

Users can declare gates before the run from CLI. Future TUI commands may add or
skip gates interactively, but skipped required gates must always include a reason.

## CLI usage

Repeat `--verify` for multiple required command gates:

```sh
imp \
  --verify "cargo fmt --package imp-core --check" \
  --verify "cargo test -p imp-core workflow_closeout" \
  "finish verification closeout"
```

CLI-provided gates are currently required command gates named `cli-verify-1`,
`cli-verify-2`, and so on. They run at agent closeout and participate in final
status enforcement.

Use explicit gates when inference is disabled or too uncertain:

```sh
imp --verify "npm test -- --runInBand" "fix the frontend regression"
```

## Mana usage

Mana tasks already have a verify-command concept. The workflow runtime should
translate mana-provided verification into `VerificationGate` records with source
`ManaTask`. A mana verify gate should behave like any other required gate:

- pass -> closeout may be `DONE`
- fail -> `DONE_WITH_CONCERNS` or `BLOCKED`
- blocked -> `BLOCKED`
- skipped -> requires reason and cannot silently become `DONE`

When closing mana work, store artifact refs or evidence summaries rather than
inlining full logs. The durable mana record should point to the run evidence and
verification artifacts.

## Evidence packet interaction

Evidence packets summarize verification gates in the verification section:

- gate name
- required flag
- status
- command
- exit code
- artifact path

Large logs stay in artifact files. Evidence should link to them. This keeps final
answers concise while preserving reviewability.

Verification also emits trace events:

- `verification.started`
- `verification.completed`

These complement `policy.checked` and `evidence.written` events from earlier
workflow-runtime slices.

## Maintainer semantics

When adding or changing verification behavior:

1. Required gates are closeout policy, not decoration.
2. Optional/advisory gates are evidence, not blockers.
3. Required skipped gates need reasons.
4. Required blocked gates should produce `RunFinalStatus::Blocked` unless user
   explicitly chooses another closeout.
5. Do not inline large logs in chat or evidence.
6. Preserve artifact refs under `.imp/runs/<run-id>/verification/<gate-id>/`.
7. Headless/CI must fail closed instead of waiting for interactive approval.
8. Inference must remain conservative and auditable.

Relevant implementation points:

- gate model: `crates/imp-core/src/workflow/verification.rs`
- command runner: `crates/imp-core/src/workflow/verification_runner.rs`
- closeout enforcement: `crates/imp-core/src/agent/loop_state.rs`
- runtime execution/evidence: `crates/imp-core/src/agent/run_loop.rs`
- TUI status handling: `crates/imp-tui/src/app.rs`

See also `docs/imp-next-workflow-runtime.md` for the workflow-runtime phase plan.
