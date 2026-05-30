# Worktree-auto

`worktree-auto` is the autonomy mode for isolated autonomous development. It lets
imp perform local implementation work without touching the main checkout until the
user chooses what to do with the result.

Current implementation status: the core runtime has worktree planning/creation,
worktree-scoped agent setup, diff/evidence capture, conservative apply, keep, and
discard helpers, CLI closeout commands, and TUI-visible worktree status/events.
Some higher-level orchestration paths are still being integrated, so the safety
model below remains the contract: do not silently edit the main checkout when a
worktree run cannot be established.

The key invariant is simple:

> `worktree-auto` must never silently fall back to editing the main workspace.

`worktree-auto` now has runtime helpers for planning, creating, capturing, and
closing out isolated git worktree runs. If the runtime cannot create or use an
isolated worktree, it still fails closed with `autonomy_worktree_required`; it
never silently falls back to editing the main checkout.

## Scope

In scope for 394.9:

- detect whether the current cwd is inside a git repository
- detect whether cwd is already a secondary worktree
- create a dedicated git worktree and branch for a workflow run
- run the workflow with the isolated worktree as cwd
- capture worktree metadata in trace/evidence
- capture worktree diff artifacts
- present apply / keep / discard choices
- clean up safely when requested

Out of scope:

- multi-agent parallel workers
- tmux/team orchestration
- automatic conflict resolution
- remote sandboxes
- pushing branches
- production deployment
- rebasing/force-pushing user branches

## Existing building blocks

### `WorktreeTool`

`crates/imp-core/src/tools/worktree.rs` already exposes:

- `worktree list`
- `worktree add`
- `worktree remove`

It validates refs/paths, prevents removing the main/current worktree, and uses
`mana_core::worktree::detect_worktree(cwd)` in list output to distinguish a
secondary worktree from the main checkout.

`worktree-auto` should reuse the same git safety assumptions but should not depend
on the agent calling the tool manually. Worktree lifecycle helpers should live in
runtime/workflow code and may share lower-level parsing/detection helpers with the
tool later.

### Runtime isolation

394.6 made `worktree-auto` fail closed unless the monitor sees
`WorkspaceScope::Worktree`. 394.9 replaces that placeholder with runtime-created
worktree execution where the child run context carries:

```rust
WorkspaceScope::Worktree {
    path: <worktree path>,
    branch: Some(<branch>),
}
```

## Flow

### 1. Detect repository and current worktree state

Before creating anything:

1. Run git repo detection from current cwd.
2. Determine repo root/main checkout.
3. Determine whether cwd is already a secondary worktree.
4. If not in a git repo, fail closed with a clear message.
5. If current workspace has dirty changes, do not mix them into the isolated run.

Dirty main workspace behavior:

- default: fail closed and ask user to commit/stash/clean first
- optional future: allow dirty main if worktree starts from `HEAD` and no apply
  is attempted automatically
- never: silently include main dirty changes in worktree-auto

### 2. Choose branch and path

Branch naming should be deterministic enough for review and unique enough to avoid
collisions:

```text
imp/<workflow-id-or-run-id>/<slug>
```

Examples:

```text
imp/394-9/run-abc123
imp/run-f3e4d5/worktree-auto
```

Path should be outside the main checkout but near it, for example:

```text
<repo-parent>/.imp-worktrees/<branch-safe-run-id>
```

or a configured worktree root. Path rules:

- never inside `.git`
- never inside the main worktree directory
- safe path segment only
- if path exists, fail unless it is an abandoned matching worktree that can be
  recovered/kept explicitly

Start point defaults to `HEAD`. Later workflows may allow explicit start refs from
trusted config/user input.

### 3. Create worktree

Use git equivalent:

```sh
git worktree add -b <branch> <worktree-path> <start-point>
```

Creation must produce recoverable metadata:

```json
{
  "repo_root": "...",
  "main_worktree": "...",
  "worktree_path": "...",
  "branch": "imp/...",
  "start_point": "HEAD",
  "created_by_run": "run_..."
}
```

Write metadata into run artifacts before executing the workflow so interrupted
runs can recover.

### 4. Run workflow inside worktree cwd

After creation:

- set agent cwd to worktree path
- set workflow contract `workspace_scope` to `WorkspaceScope::Worktree`
- keep autonomy mode as `worktree-auto`
- keep trace/evidence under the original project `.imp/runs/<run-id>` unless a
  future host setting says otherwise
- all file writes and shell commands happen in the worktree cwd by default

If runtime cannot switch cwd/context, abort and keep/remove worktree according to
cleanup policy. Do not run in the main checkout.

### 5. Capture diff artifacts

At closeout, capture at least:

```sh
git -C <worktree> status --short
git -C <worktree> diff --stat
git -C <worktree> diff --binary
```

Artifacts:

```text
.imp/runs/<run-id>/worktree/
  worktree-metadata.json
  status.txt
  diff.stat
  diff.patch
```

`worktree-metadata.json` is the handoff file for later status/apply/keep/discard
commands.

Evidence should link these artifacts and summarize:

- branch
- worktree path
- changed file count when cheap
- whether worktree is clean/dirty
- suggested next action

### 6. Closeout choices

At closeout, user should choose one:

#### Apply

Apply means bring the worktree result into the main workspace.

Current implementation uses conservative patch application:

1. ensure main workspace is clean
2. generate a binary-safe patch from the worktree
3. run `git apply --check --binary -` in the main workspace
4. run `git apply --binary -` only if the check succeeds
5. preserve the worktree if apply fails so the user can inspect or recover

No automatic conflict resolution in 394.9.

#### Keep

Keep leaves the worktree and branch in place for manual review.

Evidence/mana should record:

- worktree path
- branch
- diff artifact path
- cleanup command suggestion

#### Discard

Discard removes the worktree and optionally deletes the branch:

```sh
git worktree remove <path>
git branch -d <branch>   # or -D only with explicit force
```

Discard must refuse if the worktree has uncommitted changes unless the user
explicitly confirms force discard. The default should be safe keep-on-uncertainty.

## TUI UX

Status examples:

```text
worktree: creating imp/394-9/run-abc123
worktree: running at ../.imp-worktrees/run-abc123
worktree: diff ready 7 files
worktree: apply | keep | discard
```

Closeout should make it obvious that changes are isolated until applied. The TUI
should not report that main workspace was modified unless apply succeeds.

If creation is blocked:

```text
worktree-auto blocked: main workspace has uncommitted changes. Commit/stash or use local-auto.
```

If 394.9 runtime support is not active, current 394.6 behavior remains:
`autonomy_worktree_required`.

## CLI UX

Examples:

```sh
imp --autonomy worktree-auto "make the parser refactor"
imp --autonomy worktree-auto --verify "cargo test -p imp-core" "fix the bug"
```

Closeout commands operate on the saved worktree metadata file:

```sh
imp worktree status .imp/runs/<run-id>/worktree/worktree-metadata.json
imp worktree keep .imp/runs/<run-id>/worktree/worktree-metadata.json
imp worktree apply .imp/runs/<run-id>/worktree/worktree-metadata.json
imp worktree discard .imp/runs/<run-id>/worktree/worktree-metadata.json
```

`status` prints the worktree path, branch, original checkout, patch path, and
current `git status --short`. `keep`, `apply`, and `discard` print a JSON closeout
result. `apply` refuses dirty main workspaces and reports conflicts instead of
resolving them. `discard` removes the worktree and deletes the branch through the
runtime closeout helper; use it only when the worktree result is no longer needed.

## Mana/evidence refs

Mana should store durable refs, not huge diffs:

- run id
- branch
- worktree path
- diff artifact path
- final lifecycle choice
- apply result or cleanup status

Evidence should include the same refs and a short summary. Raw diff stays in
artifact files.

## Safety model

Hard requirements:

- no silent main-workspace fallback
- no destructive cleanup without explicit user choice
- no applying patch to dirty main workspace by default
- no force branch deletion by default
- no automatic conflict resolution
- no pushing/deploying from worktree-auto
- all worktree lifecycle actions are traced/evidenced
- interrupted runs leave enough metadata for recovery

## Failure handling

| Failure | Behavior |
| --- | --- |
| not a git repo | fail closed; suggest local-auto/safe |
| main workspace dirty | fail closed by default; suggest commit/stash/local-auto |
| branch/path collision | fail closed; suggest keep/recover/remove stale worktree |
| worktree add fails | fail closed; include git stderr |
| workflow fails | keep worktree; capture diff/status if possible |
| verification fails | keep worktree; close as concerns/blocked per verification rules |
| apply fails | keep worktree; report manual apply/conflict steps |
| discard sees dirty worktree | refuse unless explicit force discard |

## Implementation mapping

394.9 tasks should roughly map as:

- 394.9.2: lifecycle helpers for repo detection, branch/path selection,
  create/remove, metadata write
- 394.9.3: run agent/workflow with worktree cwd and `WorkspaceScope::Worktree`
- 394.9.4: diff/status artifact capture
- 394.9.5: apply/keep/discard lifecycle commands or runtime actions
- 394.9.6: TUI status and closeout choices
- 394.9.7: trace/evidence/mana metadata refs
- 394.9.8: final user docs and limitations

Current implementation exposes lifecycle actions through `imp worktree` metadata
commands and emits worktree events/artifacts for TUI, trace, and evidence. Future
work may integrate the same actions into a broader workflow-run closeout command
surface.
