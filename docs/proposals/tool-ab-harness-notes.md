# A/B Harness Notes

Command:

```bash
cargo run -p imp-core --example tool_ab_harness
```

Modes:
- mock: `cargo run -p imp-core --example tool_ab_harness both mock`
- live: `cargo run -p imp-core --example tool_ab_harness both live`

For live mode, set:

```bash
export IMP_AB_MODEL=<model-id-or-alias>
# optional
export IMP_AB_PROVIDER=<provider>
```

Current scenarios:
- search
- list
- find
- scan_extract
- search_then_read
- search_then_edit
- repeat_read_loop

Current findings:
- Reduced tool set is competitive on simple search/list/find tasks.
- `scan extract` is clearly cleaner than legacy `grep extract`.
- Search/edit workflows show similar turn counts; reduced mode shifts work into `bash` with small streaming overhead.
- Repeat-loop detection works in both variants because it lives at the agent layer, not in a specific tool.
- Reduced variant tends to emit `ToolOutputDelta` events for bash-backed tasks; legacy native tools currently emit fewer deltas.

Interpretation:
- Removing native `ls`, `find`, and likely `diff` is low-risk.
- Replacing native `grep` with `bash` + rush-backed grep is viable for many workflows, but should still be validated with live-model behavior, not just scripted tool-call traces.
- Structural extraction belongs in `scan`, and the harness supports that separation.

Likely next expansions:
- architecture discovery scenario (list/find/read mix)
- large-output scenario to compare truncation behavior
- multi-file rename scenario
- richer live-provider prompts with expected success checks
