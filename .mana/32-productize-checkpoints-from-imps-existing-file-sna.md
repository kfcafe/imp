---
id: '32'
title: Productize checkpoints from imp's existing file snapshot safety
slug: productize-checkpoints-from-imps-existing-file-sna
status: open
priority: 1
created_at: '2026-03-27T03:47:46.773527Z'
updated_at: '2026-03-27T03:47:46.773527Z'
labels:
- feature
- safety
- ux
- imp-core
verify: cd /Users/asher/tower/imp && rg 'checkpoint' crates/imp-core/src/tools crates/imp-core/src/session.rs && cargo test -p imp-core rollback checkpoint && cargo check -p imp-core
fail_first: true
kind: epic
---

Turn imp’s existing pre-edit file snapshot mechanism into a visible, user-facing checkpoint feature. The core runtime already tracks original file contents before edits so files can be rolled back, but this capability is currently an internal safety primitive instead of an explicit user concept. This unit should expose checkpoints as a first-class feature in the CLI/TUI and session model without changing the underlying architecture more than necessary.

Do the following:

1. Audit and reuse the existing snapshot/rollback mechanism.
   - Start from the current file history / pre-edit snapshot implementation in imp-core.
   - Do not replace it with a new checkpoint system if the current mechanism can be extended cleanly.
   - Preserve the existing safety behavior for writes/edits.

2. Define a user-facing checkpoint model.
   - Introduce an explicit checkpoint concept that groups file-state safety into a named/visible action.
   - Distinguish file-state checkpoints from conversation branching/forking.
   - Keep the design local-first and session-oriented.

3. Create checkpoints automatically at appropriate times.
   - Add automatic checkpoint creation before risky edit waves or multi-file write operations.
   - Avoid excessive checkpoint spam; prefer meaningful checkpoints over one checkpoint per trivial edit.
   - Use clear names or summaries where practical.

4. Expose checkpoints in a minimal usable way.
   - Add a way to list available checkpoints.
   - Add a way to inspect what files a checkpoint covers.
   - Add a way to restore a checkpoint.
   - If full diff UX is too large for this unit, restoring and listing are higher priority than elaborate visualization.

5. Surface checkpoints in the session/runtime UX.
   - Persist enough checkpoint metadata that the session can explain what happened.
   - If appropriate, use session custom entries or another existing session mechanism instead of inventing a parallel log.
   - Make it possible for future TUI work to show checkpoints in the session tree/timeline.

6. Keep scope tight.
   - Do not implement full approval policy UX in this unit.
   - Do not implement LSP in this unit.
   - Do not redesign session branching.
   - Do not build a cloud/background checkpoint system; keep this local and file-state focused.

Desired outcome: imp has a real user-facing checkpoint feature built on the existing snapshot safety primitives. Users should be able to trust that imp can create safe restore points before larger edit operations and recover to an earlier file state when needed.
