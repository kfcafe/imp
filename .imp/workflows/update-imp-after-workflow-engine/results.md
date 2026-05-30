# Update imp after workflow engine results

## Summary

Rebuilt the current checkout, installed it to the active user-visible `imp` path, and smoke-tested the installed runtime against the workflow tool/process.

## Preflight

- Worktree was cleaned with targeted commits before install.
- Current branch: `nightly`, ahead of `origin/nightly` by local commits.
- `origin` is a GitHub network remote, so no push was performed.

## Build and install

Commands run:

```sh
cargo check
cargo build --release -p imp-cli
./target/release/imp install-local --dry-run
./target/release/imp install-local
```

Install destination:

```text
/Users/asher/bin/imp
```

## Installed smoke tests

Commands run:

```sh
command -v imp
imp --version 2>/dev/null || true
imp --help >/dev/null
imp --help | sed -n '1,80p'
```

Observed:

```text
/Users/asher/bin/imp
imp 0.2.7
```

## Workflow tool/process smoke tests

Used installed `imp` with a narrow workflow tool allowlist.

```sh
imp --no-session --tools workflow --max-turns 3 --print 'Use the workflow tool to validate all workflows in strict mode. Reply with the validation summary only.'
```

Observed:

```text
Validated 11 workflow(s): 11 ok, 0 with diagnostics.
```

```sh
imp --no-session --tools workflow --max-turns 3 --print 'Use the workflow tool to show the prototype-imp-workflow-engine workflow in strict mode. Reply with the next action and any checks needing attention.'
```

Observed:

```text
Next action: run the `update_imp` workflow step.
Checks needing attention:
- `imp_update_results_ready` — pending artifact
- `required_checks_passed` — pending aggregate
```

```sh
imp --no-session --tools workflow --max-turns 4 --print 'Use the workflow tool to run the update-imp-after-workflow-engine workflow in strict mode. Reply with the next action details, including worker, checks, and child workflow if any.'
```

Observed before marking install complete:

```text
Next action: `run step update_imp [build]`
- Worker: `builder`
- Worker assignment: `builder (builder)`
- Writes: `build_artifacts`
- Worktree: `current`
- Depends on: `verify_before_update`
- Checks: `imp_update_command_passed`
- Child workflow: none reported
```

```sh
imp --no-session --tools workflow --max-turns 4 --print 'Use the workflow tool to run the prototype-imp-workflow-engine workflow in strict mode. Reply with the next action details, including child workflow and checks.'
```

Observed:

```text
Next action: run step `update_imp` (`workflow`)
Child workflow: `update-imp-after-workflow-engine`
Depends on: `consolidate_legacy_systems`
Checks: `imp_update_results_ready`
```

## Result

Installed runtime verification passed. The installed `imp` exposes and successfully uses the workflow tool against the dogfood workflow specs.

## Concerns

- Local branch is ahead of the GitHub remote; changes were not pushed.
- Workflow tool remains advisory for worker execution; it does not spawn background subagents yet.
