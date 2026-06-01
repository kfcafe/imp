# Audit and repair native workflows — results

## Status

Completed and verified.

## Verification evidence

- `workflow validate --strict`: 24 workflows ok, 0 diagnostics.
- `cargo test -p imp-core tools::workflow::tests::`: 11 passed, 0 failed.

## Real-use trials completed

- `workflow list` against the repo workflow set.
- `workflow show` against active and completed workflows.
- `workflow validate` for one workflow and all workflows, strict and draft paths.
- `workflow run` against this audit workflow and fixture workflows.
- `workflow update` against this audit workflow, including event-log generation.

## Defects fixed

1. Update/event audit gap
   - Problem: `workflow update` replaced `workflow.yaml` before proving `events.jsonl` could be opened.
   - Risk: workflow state could mutate without an audit event if event logging failed.
   - Fix: open/preflight the event log before replacing YAML.
   - Regression: `workflow_update_rejects_unwritable_event_log_without_replacing_yaml`.

2. Workflow id path escape
   - Problem: explicit workflow `id` was joined directly onto `.imp/workflows`.
   - Risk: absolute paths or traversal could address workflow-like files outside the workflow root.
   - Fix: explicit ids must be a single normal workflow directory name; absolute, `../`, and nested ids are rejected.
   - Regression: `workflow_rejects_absolute_or_parent_directory_ids`.

3. Entry-point action policy coverage
   - Problem: mode/action policy had config-level tests but lacked direct workflow tool entrypoint coverage.
   - Fix: added regression that Auditor mode cannot run an `update` through `WorkflowTool::execute`.
   - Regression: `workflow_tool_execute_enforces_mode_action_policy`.

## Remaining concerns

- Full cross-file atomicity is still not guaranteed if event writing fails after YAML rename, because YAML and JSONL are separate files. Current hardening covers common/open/preflight failures. A journaled transaction format would be needed for stronger guarantees.
- `.imp/` is ignored by normal git status output in this repo, so workflow artifact changes require explicit inspection.

## Next useful tasks

- Consider a transaction/journal design for workflow updates if crash consistency becomes a product requirement.
- Add CLI/e2e coverage around the workflow tool once the public command surface stabilizes.

## Additional schema hardening

After the workflow was initially closed, deeper manual audit found that schema strict validation also joined parent/child workflow references directly. Validation now rejects path-like parent/child workflow references before joining paths.

Additional regression:

- `workflow_schema_rejects_path_like_workflow_references`

Additional verification:

- `cargo test -p imp-core tools::workflow::tests::`: 11 passed.
- `cargo test -p imp-core workflow_schema_`: 6 passed.
- `workflow validate --strict`: 24 workflows ok, 0 diagnostics.

Full `cargo test -p imp-core` was also attempted. It reached 895 passed, 1 ignored, and 6 failures in agent tests; five failures reported `Llm(Provider("Stream error"))`, and one was an agent workflow follow-up assertion. These failures are outside the workflow tool/schema audit path and are being investigated separately from the workflow hardening checks above.

## Full imp-core test note

A full `cargo test -p imp-core` run was attempted after workflow/schema hardening. It failed outside the workflow tool/schema scope:

- `agent::integration::agent_bash_search_finds_pattern` reproduces alone with `Llm(Provider("Stream error"))`.
- Inspection shows the local `MockProvider` returns an error event when its scripted response vector is exhausted (`No more mock responses`), which the run loop surfaces as `Stream error`.
- This is diagnostic evidence for agent integration fixture behavior, not evidence that workflow tool/schema hardening failed.

Workflow-focused verification remains green and is the relevant gate for this audit.

## Full imp-core suite resolved

After investigating the full-suite failures, the failing agent tests were updated to match current workflow closeout/follow-up behavior instead of exhausting scripted mock responses or asserting outdated wording.

Final verification:

- `cargo test -p imp-core`: 901 passed, 0 failed, 1 ignored; doc-tests 2 passed.
- `workflow validate --strict`: 24 workflows ok, 0 diagnostics.

## Final closeout verification

The final combined gate passed after correcting the formatter command scope:

- `cargo test -p imp-core`: 901 passed, 0 failed, 1 ignored; doc-tests 2 passed.
- `cargo clippy -p imp-core --all-targets -- -D warnings`: passed.
- `rustfmt --edition 2021 --check` on audit-touched Rust files: passed.
- `git diff --check` on audit-touched Rust files: passed.
- `workflow validate --strict`: 24 workflows ok, 0 diagnostics.

Workspace-wide `cargo fmt --check` is intentionally not used as the closing gate because unrelated dirty files in the workspace still have formatting drift. The audit-touched Rust files pass the scoped formatter check.

## Security/dependency audit addendum

Dependency scan:

- `audit_scan deps .` reports 7 known advisories in the workspace lockfile.
- Direct workflow/`imp-core` parsing path uses `serde_yaml v0.9.34+deprecated`, not the OSV-reported `serde_yml/libyml` pair.
- `serde_yml/libyml` are pulled via `mana-core -> imp-gui`.
- `yaml-rust` is pulled via `syntect -> imp-tui`.
- `lru` is pulled via `ratatui -> imp-tui`.
- Practical workflow parser mitigation added: `workflow.yaml` loads are capped at 1 MiB before YAML parsing.

Secrets scan:

- Current working tree scan with `gitleaks detect --source . --no-banner --redact --no-git`: no leaks found.
- History scan reports test/header literals in historical commits; inspected current lines and found no current-tree leak from this audit.

Final post-addendum verification:

- `cargo test -p imp-core`: 902 passed, 0 failed, 1 ignored; doc-tests 2 passed.
- `cargo clippy -p imp-core --all-targets -- -D warnings`: passed.
- Scoped `rustfmt --edition 2021 --check` for audit-touched Rust files: passed.
- Scoped `git diff --check` for audit-touched Rust files: passed.
- `workflow validate --strict`: 24 workflows ok, 0 diagnostics.

## Capped update-read consistency follow-up

Maintainability review found one consistency gap after adding the 1 MiB workflow YAML cap: normal workflow loads used the cap, but `workflow update` still read `workflow.yaml` directly. This is now fixed by introducing `load_workflow_raw` and using it from both normal load and update paths.

Final verification after formatting correction:

- `cargo test -p imp-core`: 902 passed, 0 failed, 1 ignored; doc-tests 2 passed.
- `cargo clippy -p imp-core --all-targets -- -D warnings`: passed.
- Scoped `rustfmt --edition 2021 --check` for audit-touched Rust files: passed.
- Scoped `git diff --check` for audit-touched Rust files: passed.
- `workflow validate --strict`: 24 workflows ok, 0 diagnostics.

## TUI workflow rendering follow-up

A subsequent workflow-surface scan found that the TUI workflow helpers also parsed workflow YAML directly and joined explicit workflow ids. The TUI now reuses capped `imp_core::workflow` loaders and validates explicit workflow ids as single workflow directory names before rendering/validation.

Clippy also surfaced nearby TUI issues while verifying the change; fixed locally:

- Boxed `AskReply::WorkflowSuggestion.profile` to avoid a large enum variant.
- Removed a needless borrow in sidebar detail rendering.
- Replaced a `let...else` early return with `?` in cached chat render lookup.

Affected-package verification:

- `cargo test -p imp-core`: 902 passed, 0 failed, 1 ignored; doc-tests 2 passed.
- `cargo check -p imp-tui`: passed.
- `cargo clippy -p imp-core --all-targets -- -D warnings`: passed.
- `cargo clippy -p imp-tui --all-targets -- -D warnings`: passed.
- Scoped `rustfmt --edition 2021 --check` for audit-touched Rust files: passed.
- Scoped `git diff --check` for audit-touched Rust files: passed.
- `workflow validate --strict`: 24 workflows ok, 0 diagnostics.

## Documentation semantics follow-up

A documentation consistency pass found stale design wording that said workflow runtime should append `events.jsonl` before refreshing `workflow.yaml`. Current implementation validates the prospective YAML, opens/preflights `events.jsonl`, replaces `workflow.yaml`, then appends the event. README and workflow docs now describe that behavior and explicitly note the remaining two-file crash-consistency limit.

Verification:

- `git diff --check -- README.md docs/workflows.md docs/design/imp-native-workflow-engine.md`: passed.
- Doc consistency search found no stale `append events first` wording in the updated files and confirmed the new `open/preflight`/non-full-transaction wording.
- `workflow validate --strict`: 24 workflows ok, 0 diagnostics.
