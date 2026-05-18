# imp-next Workflow UI Wireframes

Status: grounded design draft  
Scope: improve the current TUI first; keep the design GUI-ready through shared runtime state  
Related epic: `394 Evolve imp into workflow-first agent runtime with mana ledger and extension support`

## 1. Grounding: current TUI reality

This document is based on the current imp TUI implementation, not a hypothetical app shell.

Relevant current source:

- `crates/imp-tui/src/app.rs`
  - `App::render` clears the frame and splits the terminal vertically into:
    - a **chat/messages area**
    - a bottom **editor/ask prompt area**
  - The sidebar opens by splitting the chat area horizontally when `self.sidebar.open && chat_area.width >= 60`.
  - The bottom prompt area renders:
    - `AskBar` when `ask_state` is active
    - otherwise `EditorView`
- `crates/imp-tui/src/views/editor.rs`
  - `EditorView` is already a rich prompt box/superbar.
  - It has top-left identity: cwd + session name.
  - It has top-right turn elapsed time.
  - It has bottom-left labels: mana scope, mana run, build loop/loop state.
  - It has bottom-right labels: model, thinking level, context usage, git label, loop/activity.
  - Placeholder currently says: `Ask anything… ⇧↵ newline  @file attach context  / palette  ! or : shell  :cd cwd`.
- `crates/imp-tui/src/views/chat.rs`
  - `ChatView` renders display messages and interleaved tool calls.
  - Tool calls can be focused/expanded.
- `crates/imp-tui/src/views/sidebar.rs`
  - `SidebarView` already lists tool calls and detail.
  - It can show mana run detail or thinking when no tool is selected.
- `crates/imp-tui/src/views/ask_bar.rs`
  - `AskBar` already supports prompt/question UX with options and free-form reply.

So the UI plan should **extend** these existing surfaces:

```text
Current structure:

┌──────────────────────────────────────────────┐
│ chat / startup panel                         │
│ optionally split with tool/sidebar inspector │
├──────────────────────────────────────────────┤
│ EditorView prompt OR AskBar prompt           │
└──────────────────────────────────────────────┘
```

Not this:

```text
┌ new persistent header ───────────────────────┐
│ new dashboard                                │
├ new status-only footer ──────────────────────┤
│ separate command bar                         │
└──────────────────────────────────────────────┘
```

## 2. Product direction

The current imp TUI should remain conversation-first. Workflow-first features should make the existing TUI more legible, not turn it into a project-management dashboard.

A future GUI may make sense for richer evidence browsing, diff review, worktree management, and child workflow supervision. But the GUI should consume the same runtime state as the TUI:

```text
RuntimeEvent + RuntimeStateSnapshot
  -> current TUI presentation
  -> future GUI presentation
  -> CLI/RPC/headless presentation
```

## 3. Design principles

1. **Keep the current layout.** Chat remains above; prompt remains below.
2. **Use the existing editor border.** Put workflow/mode/gate metadata in `EditorView` labels before inventing a new header/footer.
3. **Use the existing sidebar.** Tool/workflow/evidence details belong in the current sidebar pattern.
4. **Use AskBar for interruptions.** Approvals, risky confirmations, and failed verification choices should reuse `AskBar` where possible.
5. **Reduce redundant words.** Prefer compact values over repeated labels like `workflow:`, `phase:`, `status:`.
6. **Make autonomy visible.** `local-auto`, `worktree-auto`, and `ALLOW-ALL` must be obvious in the prompt box.
7. **Make worktree scope obvious.** If editing an isolated worktree, the prompt identity/status must show that.
8. **Closeout should be satisfying.** Final summaries should make evidence, verification, diff, and mana status easy to inspect.

## 4. Existing layout, annotated

```text
┌─────────────────────────────────────��────────────────────────────────────────┐
│ ChatView / StartupPanelView                                                  │
│                                                                              │
│ - User/assistant/system/warning messages                                     │
│ - Interleaved tool calls depending on ChatToolDisplay                        │
│ - Optional streaming activity                                                │
│ - Optional word wrap and timestamps                                          │
│                                                                              │
│                                              ┌──────────────────────────────┐│
│                                              │ Optional SidebarView         ││
│                                              │ - tool list/detail           ││
│                                              │ - thinking detail            ││
│                                              │ - mana run detail            ││
│                                              └──────────────────────────────┘│
├ ~/cwd · session-name ──────────────────────────────────────────────── 00:08 ┤
│ Ask anything… ⇧↵ newline  @file attach context  / palette  ! or : shell      │
│                                                                              │
│ bottom-left: mana/build/loop labels                                           │
│ bottom-right: model · thinking · context · git · activity                     │
└──────────────────────────────────────────────────────────────────────────────┘
```

The workflow UI should be designed as new labels, content, sidebar modes, and AskBar states inside this structure.

---

# Part A — TUI wireframes based on current implementation

## 5. Default workflow run

This is the main wireframe. It keeps the current prompt box visible.

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ You                                                                          │
│   Fix the failing auth tests.                                                 │
│                                                                              │
│ imp                                                                          │
│   I’ll reproduce the failure, inspect the auth/session code, make a narrow    │
│   fix, then rerun the targeted test.                                          │
│                                                                              │
│ Plan                                                                         │
│   1. Reproduce failing test.                                                  │
│   2. Inspect assertion and expiry code.                                       │
│   3. Patch smallest relevant path.                                            │
│   4. Verify.                                                                 │
│                                                                              │
│ Tool calls                                                                   │
│   ✓ bash npm test -- tests/auth/session.test.ts        12.4s failed          │
│   ✓ read tests/auth/session.test.ts                    3.1KB                 │
│   ✓ read src/auth/session.ts                           5.8KB                 │
│   ✓ edit src/auth/session.ts                           +3 -2                 │
│   ◌ bash npm test -- tests/auth/session.test.ts        running 00:08         │
├ ~/project · fix-auth-tests ────────────────────────────────────────── 00:08 ┤
│ Ask anything… ⇧↵ newline  @file attach context  / palette  ! or : shell      │
│                                                                              │
│ wf auth-tests · local-auto · verifying                                       │
│ gpt-5.1-codex · high · 42k/200k · main · gates 0/1 · test running            │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Implementation mapping

- Top-left border remains `build_identity_label(cwd, session_name, width)`.
- Top-right remains elapsed time.
- Bottom-left extends `build_bottom_left_label(...)` with workflow title/mode/phase.
- Bottom-right extends current metadata cluster with gates/activity.
- Tool calls remain in `ChatView` and `SidebarView`.

## 6. Narrow terminal

```text
┌────────────────────────────────────┐
│ You                                │
│   Fix the failing auth tests.       │
│                                    │
│ imp                                │
│   I’ll reproduce and patch.         │
│                                    │
│ ✓ read src/auth/session.ts         │
│ ✓ edit src/auth/session.ts         │
│ ◌ bash npm test ...       00:08    │
├ ~/project ───────────────── 00:08 ┤
│ Ask anything… / palette            │
│                                    │
│ wf auth · local-auto · verifying   │
│ gpt-5.1 · 42k/200k · gates 0/1     │
└────────────────────────────────────┘
```

Narrow terminals should not get a separate status bar or inspector.

## 7. Wide terminal with existing sidebar

Current sidebar behavior already fits the desired inspector pattern.

```text
┌───────────────────────────────────────────────┬──────────────────────────────┐
│ You                                           │ Tool calls                   │
│   Fix the failing auth tests.                  │                              │
│                                               │ ✓ bash npm test ... failed   │
│ imp                                           │ ✓ read session.test.ts       │
│   I’ll reproduce the failure...                │ ✓ read session.ts            │
│                                               │ ✓ edit session.ts            │
│ Plan                                          │ ◌ bash npm test ... running  │
│   1. Reproduce failing test.                   │                              │
│   2. Patch expiry logic.                       │ Detail                       │
│   3. Verify.                                  │ cwd: ~/project               │
│                                               │ log: .imp/runs/.../test.log  │
├ ~/project · fix-auth-tests ────────────────────────────────────────── 00:08 ┤
│ Ask anything… ⇧↵ newline  @file attach context  / palette  ! or : shell      │
│                                                                              │
│ wf auth · local-auto · verifying                                             │
│ gpt-5.1-codex · high · 42k/200k · main · gates 0/1 · test running            │
└──────────────────────────────────────────────────────────────────────────────┘
```

Possible sidebar modes:

```text
Tool detail       current default when tool selected
Workflow detail   when no tool selected or user selects workflow inspector
Evidence detail   after evidence exists
Mana detail       current mana run detail pattern
```

## 8. Editor border states

### Safe default

```text
├ ~/project · session-name ─────────────────────────────────────────── 00:03 ┤
│ Ask anything… ⇧↵ newline  @file attach context  / palette  ! or : shell      │
│                                                                              │
│ safe                                                                         │
│ gpt-5.1-codex · high · 12k/200k · main                                       │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Local auto

```text
├ ~/project · fix-auth-tests ───────────────────────────────────────── 00:08 ┤
│ >                                                                            │
│ wf auth · local-auto · verifying                                             │
│ gpt-5.1-codex · high · 42k/200k · main · gates 0/1 · test running            │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Worktree auto

```text
├ ../project-imp-loader · refactor-loader ──────────────────────────── 01:12 ┤
│ >                                                                            │
│ wf loader · worktree-auto · isolated                                         │
│ gpt-5.1-codex · high · branch imp/loader/run-123 · gates 1/2                 │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Allow-all

```text
├ ~/project · docs-update ──────────────────────────────────────────── 00:31 ┤
│ >                                                                            │
│ wf docs · ALLOW-ALL · repo scope · audit on                                  │
│ gpt-5.1-codex · high · 18k/200k · main · hard rails on                       │
└──────────────────────────────────────────────────────────────────────────────┘
```

## 9. Existing AskBar for approvals

Approvals should initially use `AskBar`, not a new modal system.

```text
┌ ask ────────────────────────────────────────────────────────────────────────┐
│ Approve package install?                                                     │
│                                                                              │
│ Tool: bash npm install left-pad                                              │
│ Reason: dependency change in safe mode requires approval.                    │
│ Risk: may modify lockfile and execute lifecycle scripts.                     │
│ Scope: ~/project                                                             │
│                                                                              │
│  ❯ Approve once                                                              │
│    Deny                                                                      │
│    Dry-run                                                                   │
│    Allow package installs for this workflow                                  │
│    Switch mode                                                               │
│                                                                              │
│ ❯ type to answer freely…                                                     │
│ ↑↓: navigate  Tab: edit  Enter: pick  Esc: skip                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

## 10. Existing AskBar for allow-all confirmation

```text
┌ ask ────────────────────────────────────────────────────────────────────────┐
│ Enable allow-all for this workflow?                                          │
│                                                                              │
│ allow-all reduces prompts but does not disable trace/evidence.               │
│ Scope: ~/project                                                             │
│ Hard rails remain on unless separately disabled.                             │
│                                                                              │
│  ❯ Enable allow-all                                                          │
│    Use allow-all-local instead                                               │
│    Cancel                                                                    │
│                                                                              │
│ ❯ type to answer freely…                                                     │
│ ↑↓: navigate  Tab: edit  Enter: pick  Esc: skip                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

## 11. Policy denial

Policy denials can be chat warnings plus optional AskBar action if user can respond. No new modal required.

```text
Warning
  Blocked by policy: bash cat ~/.ssh/id_rsa

  Reason: private key access is a hard rail.
  Next: provide the needed credential through a scoped secret grant.
```

If there is a choice:

```text
┌ ask ────────────────────────────────────────────────────────────────────────┐
│ Policy blocked this action. What next?                                       │
│                                                                              │
│ bash cat ~/.ssh/id_rsa                                                       │
│ Private key access is a hard rail.                                           │
│                                                                              │
│  ❯ Cancel action                                                             │
│    Open evidence                                                             │
│    Explain secret grants                                                     │
│                                                                              │
│ ❯ type to answer freely…                                                     │
└──────────────────────────────────────────────────────────────────────────────┘
```

## 12. Verification in existing surfaces

Verification should appear in three places:

1. compact gate count in `EditorView` bottom-right metadata
2. chat events/tool rows while verification runs
3. sidebar detail or closeout summary for full results

### In chat

```text
Verification
  ◌ npm test -- tests/auth/session.test.ts        running 00:21
```

### In sidebar detail

```text
Verification

Required
  ✓ npm test -- tests/auth/session.test.ts     12.8s
  ◌ npm run typecheck                         running 00:21

Optional
  ○ npm run lint                              skipped

Closeout
  DONE requires required gates to pass.
```

### Failed gate as AskBar

```text
┌ ask ────────────────────────────────────────────────────────────────────────┐
│ Required verification failed.                                                │
│                                                                              │
│ npm test -- tests/auth/session.test.ts exited 1.                             │
│ Log: .imp/runs/run_123/verify/auth-test.log                                  │
│                                                                              │
│  ❯ Continue debugging                                                        │
│    Finish BLOCKED                                                            │
│    Open evidence                                                             │
│                                                                              │
│ ❯ type to answer freely…                                                     │
└──────────────────────────────────────────────────────────────────────────────┘
```

## 13. Evidence closeout as chat block plus editor hint

The final answer should be a normal assistant/chat block, followed by compact editor status. Avoid a separate full-screen modal unless the current TUI already uses one for similar flows.

```text
imp
  DONE

  Changed
    src/auth/session.ts                         +3 -2
    tests/auth/session.test.ts                  +8 -0

  Verified
    ✓ npm test -- tests/auth/session.test.ts     12.8s
    ✓ npm run typecheck                         18.3s

  Evidence
    .imp/runs/run_123/evidence.md
    .imp/runs/run_123/trace.jsonl
    .imp/runs/run_123/diff.patch

├ ~/project · fix-auth-tests ───────────────────────────────────────── 00:46 ┤
│ Ask anything… ⇧↵ newline  @file attach context  / palette  ! or : shell      │
│                                                                              │
│ DONE · evidence ready                                                        │
│ e open evidence · d diff · m mana · gpt-5.1-codex · main                    │
└──────────────────────────────────────────────────────────────────────────────┘
```

`DONE_WITH_CONCERNS`:

```text
imp
  DONE_WITH_CONCERNS

  Completed
    Implemented parser fallback for malformed tool JSON.

  Verified
    ✓ cargo test -p imp-core parser
    ○ full workspace test skipped

  Concern
    Full test suite takes ~20m and was not run.
```

`BLOCKED`:

```text
imp
  BLOCKED

  Blocker
    Required environment variable DATABASE_URL is missing.

  Tried
    ✓ inspected test setup
    ✓ checked .env.example
    ✗ attempted integration test

  Next
    Provide a local DATABASE_URL or mark integration test unavailable.
```

## 14. Mana ledger through existing sidebar/detail

Current `SidebarView` can show mana run detail when no tool is selected. Build on that.

```text
┌──────────────────────────────┐
│ Mana                         │
│                              │
│ 394.2.3                      │
│ Thread workflow contract...  │
│                              │
│ Status                       │
│ executing                    │
│                              │
│ Acceptance                   │
│ - workflow contract in loop  │
│ - existing tests pass        │
│                              │
│ Verification                 │
│ pending cargo test ...       │
│                              │
│ Evidence                     │
│ .imp/runs/run_123/...        │
└──────────────────────────────┘
```

Do not put mana in the main chat unless the user asked to inspect/update it.

## 15. Worktree-auto in current layout

### Before run via AskBar

```text
┌ ask ────────────────────────────────────────────────────────────────────────┐
│ Create isolated worktree for this workflow?                                  │
│                                                                              │
│ Repo: ~/project                                                              │
│ Worktree: ../project-imp-refactor-loader                                     │
│ Branch: imp/refactor-loader/run-123                                          │
│                                                                              │
│  ❯ Create worktree                                                           │
│    Use current workspace instead                                             │
│    Cancel                                                                    │
│                                                                              │
│ ❯ type to answer freely…                                                     │
└──────────────────────────────────────────────────────────────────────────────┘
```

### During run in editor border

```text
├ ../project-imp-loader · refactor-loader ──────────────────────────── 01:12 ┤
│ >                                                                            │
│ wf loader · worktree-auto · isolated                                         │
│ branch imp/loader/run-123 · original ~/project · gates 1/2                   │
└──────────────────────────────────────────────────────────────────────────────┘
```

### After run as AskBar choice

```text
┌ ask ────────────────────────────────────────────────────────────────────────┐
│ Worktree result ready.                                                       │
│                                                                              │
│ 7 files changed, +184 -96                                                    │
│ Diff: .imp/runs/run_123/diff.patch                                           │
│ Worktree: ../project-imp-refactor-loader                                     │
│                                                                              │
│  ❯ Apply to current workspace                                                │
│    Keep worktree and branch                                                  │
│    Discard worktree                                                          │
│    View diff                                                                 │
│                                                                              │
│ ❯ type to answer freely…                                                     │
└──────────────────────────────────────────────────────────────────────────────┘
```

## 16. Role selection via existing overlays/palette pattern

Role selection probably belongs in the command palette or a picker overlay, similar to model/session picker patterns.

```text
┌ Select role ────────────────────────────────────────────────────────────────┐
│ Coder       edit files and verify within policy                              │
│ Planner     read-only planning                                               │
│ Verifier    run checks and report evidence                                   │
│ Reviewer    review diff and risk                                             │
│ Researcher  search/read/summarize                                            │
│ Integrator  apply/merge approved work                                        │
└──────────────────────────────────────────────────────────────────────────────┘
```

Once selected, it appears in the editor border:

```text
│ wf auth · verifier · read-only · verifying                                   │
```

## 17. Child workflows in current sidebar pattern

Do not invent a team dashboard in the first TUI pass. Use sidebar/detail.

```text
┌──────────────────────────────┐
│ Child workflows              │
│                              │
│ ◌ verifier   running         │
│ ✓ reviewer   completed       │
│ ○ researcher skipped         │
│                              │
│ Selected: verifier           │
│ last activity: 12s ago       │
│ evidence: pending            │
│                              │
│ actions: open cancel nudge   │
└──────────────────────────────┘
```

## 18. Trust warnings as warning messages

Current chat already has warning/error/system roles. Use those first.

```text
Warning
  Low-trust content attempted to authorize a high-risk action.

  Source: external webpage https://example.com/install.md
  Decision: denied
  Reason: external content cannot grant shell/network/secret permissions.
```

If the user can respond, follow with `AskBar`.

---

# Part B — future GUI, grounded in current state model

A future GUI can be richer, but it should still reflect the current TUI concepts:

- chat/activity stream
- tool/sidebar detail
- editor/composer
- ask/approval prompts
- session/workflow list

## 19. GUI shell

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ imp                                                                         │
│ ~/project · fix-auth-tests                         local-auto · executing    │
├──────────────┬───────────────────────────────────────────────┬───────────────┤
│ Sessions     │ Chat / activity                               │ Detail        │
│              │                                               │               │
│ ◌ auth tests │ You                                           │ Tool calls    │
│ ✓ docs       │   Fix the failing auth tests.                  │ ✓ test failed │
│ ! refactor   │                                               │ ✓ read file   │
│              │ imp                                           │ ◌ test run    │
│              │   I’ll reproduce the failure...                │               │
│              │                                               │ Verification  │
│              │ Tool calls                                    │ gates 0/1     │
│              │ ✓ bash npm test ... failed                    │               │
│              │ ◌ bash npm test ... running                   │ Evidence      │
│              │                                               │ pending       │
├──────────────┴───────────────────────────────────────────────┴───────────────┤
│ Ask anything…                                                               │
└──────────────────────────────────────────────────────────────────────────────┘
```

## 20. GUI detail tabs

```text
┌ Detail ────────────────────────────┐
│ [Tools] [Verify] [Evidence] [Mana] │
│ [Policy] [Worktree] [Children]     │
│                                    │
│ selected detail content            │
└────────────────────────────────────┘
```

## 21. GUI approval modal maps to AskBar semantics

```text
┌ Approval required ─────────────────────────────────────────────────────────┐
│ Approve package install?                                                   │
│                                                                            │
│ Tool: bash npm install left-pad                                             │
│ Reason: dependency change in safe mode.                                     │
│ Risk: may modify lockfile and execute lifecycle scripts.                    │
│                                                                            │
│ [Approve once] [Deny] [Dry-run] [Switch mode]                               │
└────────────────────────────────────────────────────────────────────────────┘
```

The GUI can render this as a modal, but it should come from the same ask/approval state as the TUI.

## 22. GUI worktree/diff view

This is where GUI becomes more valuable than TUI.

```text
┌ Worktree result ────────────────────────────────────────────────────────────┐
│ Original: ~/project              Worktree: ../project-imp-loader            │
│ Branch: imp/refactor-loader/run-123                                         │
├ Files changed ────────────────┬ Diff ───────────────────────────────────────┤
│ src/loader/index.ts           │ - old line                                  │
│ src/loader/config.ts          │ + new line                                  │
│ tests/loader.test.ts          │                                             │
├───────────────────────────────┴─────────────────────────────────────────────┤
│ [Apply] [Keep worktree] [Discard] [Open in editor]                          │
└──────────────────────────────────────────────────────────────────────────────┘
```

## 23. GUI evidence view

```text
┌ Evidence ──────────────────────────────────────────────────────────────────┐
│ DONE                                                                       │
│                                                                            │
│ Changed                                                                    │
│   src/auth/session.ts                         +3 -2                         │
│   tests/auth/session.test.ts                  +8 -0                         │
│                                                                            │
│ Verified                                                                   │
│   ✓ npm test -- tests/auth/session.test.ts                                  │
│   ✓ npm run typecheck                                                       │
│                                                                            │
│ Artifacts                                                                  │
│   evidence.md                                                              │
│   trace.jsonl                                                              │
│   diff.patch                                                               │
│                                                                            │
│ [Open evidence.md] [Open diff] [Export] [Save eval candidate]               │
└────────────────────────────────────────────────────────────────────────────┘
```

## 24. GUI should remain optional

Everything important must remain possible through:

- TUI
- CLI/headless mode
- trace/evidence artifacts
- mana ledger

The GUI is richer inspection/control, not the source of truth.

---

# Part C — recommended UI implementation path

## 25. Phase 1: extend existing editor labels

- Add workflow title/mode/phase to `EditorView` bottom-left.
- Add gates/activity/autonomy warning to bottom-right where appropriate.
- Avoid adding a new header/footer.

## 26. Phase 2: sidebar detail modes

- Existing tool detail remains default.
- Add workflow detail when no tool is selected.
- Add evidence detail after closeout.
- Add mana detail using current mana run detail pattern.

## 27. Phase 3: AskBar workflows

Use `AskBar` for:

- approval required
- allow-all confirmation
- failed required verification
- worktree create/apply/keep/discard
- policy blocked with choices

## 28. Phase 4: closeout chat blocks

Render `DONE`, `DONE_WITH_CONCERNS`, and `BLOCKED` as normal assistant/chat blocks plus editor border hints.

## 29. Phase 5: GUI prototype over shared state

Only after `RuntimeStateSnapshot` exists:

- sessions/workflows list
- chat/activity center
- detail tabs
- evidence view
- worktree diff view

No GUI-only workflow state.

## 30. Open questions

1. Should workflow title/mode live in bottom-left label or top-left identity after cwd/session?
2. How much can bottom-left hold before it wraps badly?
3. Should `ALLOW-ALL` change editor border style in addition to text?
4. Should verification failures always trigger AskBar, or only when the agent wants a user decision?
5. Should closeout summary be inserted into chat automatically, or shown in sidebar until user asks?
6. Should worktree apply/keep/discard use AskBar immediately or a dedicated overlay later?
7. Should GUI be a local desktop app, browser UI served by imp, or embedded app shell?
8. How much of SidebarView should be generalized from tool detail to workflow/evidence detail?

## 31. Summary

The main correction is: **do not design a new TUI shell yet**.

Use what exists:

- `ChatView` for conversation and tool history
- `SidebarView` for details
- `EditorView` for prompt and compact workflow status
- `AskBar` for approval/decision interactions
- existing overlays/pickers for role/model/session-like selection

This is enough to make workflow-first imp feel much better without losing the current TUI's shape.
