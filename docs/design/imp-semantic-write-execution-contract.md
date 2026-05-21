# Semantic Write Execution Contract

This contract specifies runtime sequencing for write-oriented semantic actions in imp. It was reconstructed from mana unit `44.1.5.5` because the originally referenced source docs were not present in this worktree.

## Scope

Applies only to semantic writes such as rename, organize imports, and allowlisted code actions. Normal `edit`/`write` tools and read-oriented code-intelligence queries are separate paths.

## Execution Flow

1. **Request intake**
   - Normalize the requested semantic action, target file/range/symbol, operator intent, and desired verification command.
   - Reject requests outside the workspace boundary.
2. **Capability lookup**
   - Resolve action policy by language, backend, action kind, workspace trust, and configured posture.
   - Deny unknown write actions by default.
3. **Freshness check before preview**
   - Confirm file content, semantic backend index, and workspace root match the request snapshot.
   - If stale, refresh once; if still stale, reject with a stale-preview error.
4. **Preview generation**
   - Ask the hosted semantic adapter for an edit preview only.
   - Convert backend edits into a normalized preview envelope with affected paths, ranges, summary, risk flags, and size estimates.
   - Do not execute arbitrary backend commands.
5. **Preview validation**
   - Ensure all edits remain inside the workspace and inside allowed file classes.
   - Reject previews that are too large, ambiguous, partial, or include unsupported operations.
6. **Approval gate**
   - Apply the configured `ApprovalPosture`.
   - If approval is required, show the operator-visible preview and require explicit acknowledgement tied to the preview fingerprint.
7. **Checkpoint**
   - Create a restore checkpoint after approval and immediately before apply.
   - Label it with action kind, target, preview fingerprint, timestamp, and verify command.
8. **Apply**
   - Apply the normalized hosted edit set through imp’s workspace edit path.
   - Re-check that target files have not drifted since preview.
9. **Semantic refresh**
   - Refresh semantic diagnostics/index state after apply.
   - Treat backend success as secondary evidence only.
10. **Verify**
    - Run the explicit verify command or required narrow project check.
    - Verification is mandatory before reporting success.
11. **Result or restore surface**
    - Emit a `SemanticWriteResult` receipt with preview, approval, checkpoint, apply, refresh, and verify outcomes.
    - If apply or verify fails, keep restore affordance visible and include the checkpoint id.

## Freshness and Preview Recompute Rules

- A preview is bound to a workspace root, file content hashes, backend generation, action kind, and target location.
- Recompute preview when any affected file changes, the backend reports a newer semantic generation, or the operator changes action parameters.
- Reject instead of recomputing when the backend cannot prove freshness, preview generation repeatedly changes affected paths, or the target symbol/range no longer resolves.
- Approval is invalidated by any recomputed preview. The operator must acknowledge the new preview fingerprint.

## Checkpoint Rules

- Checkpoint after approval, before apply.
- Capture all files that may be edited by the normalized preview, plus minimal metadata needed to explain/restore the action.
- Label format should include: `semantic-write:<action-kind>:<target>:<preview-fingerprint>`.
- The restore affordance must remain visible until verification passes or the operator dismisses it after reviewing failure state.
- If checkpoint creation fails, do not apply.

## Approval Handshake

- `ApprovalPosture::AutoAllow` may proceed only for allowlisted low-risk actions with bounded preview size and fresh backend state.
- `ApprovalPosture::PreviewRequired` requires rendering the preview but may continue after explicit model/operator acknowledgement if policy allows.
- `ApprovalPosture::OperatorRequired` requires a human-visible acknowledgement of the exact preview fingerprint.
- Acknowledgement must include action kind, affected paths, edit count, risk flags, and verify command.
- Any preview drift, stale state, policy change, or target change clears approval.

## Hosted Apply Path

- Semantic backends may propose edits; imp owns applying them.
- Backends must not run arbitrary write commands, shell commands, package-manager commands, or formatter commands as the apply mechanism.
- All edits pass through the same workspace boundary checks as other hosted edits.
- File creation/deletion is denied unless the action kind explicitly allows it.

## Failure Handling

- **Preview too large**: reject with summary, affected count, truncation reason, and suggested narrower target.
- **Apply drift**: abort before writing, recompute preview if safe, otherwise require new approval.
- **Partial apply**: stop, surface files changed, keep checkpoint visible, and recommend restore before retry.
- **Semantic refresh failure**: report degraded state; still run deterministic verify if safe, but do not claim semantic success.
- **Verify failure**: report failure with command, exit code, compact diagnostics, and restore affordance.
- **Approval rejection**: no checkpoint or apply; record rejected preview as non-durable runtime output unless explicitly promoted.
- **Operator cancellation**: cancel before next stage; if after apply, present checkpoint restore option.

## Receipt Rules

A semantic write receipt should include:

- action kind and target;
- policy decision and approval posture;
- preview fingerprint and affected paths;
- checkpoint id/label if created;
- apply outcome;
- semantic refresh outcome;
- verify command and outcome;
- restore affordance state;
- durable evidence summary.

Durable evidence includes final outcome, affected paths, verification result, checkpoint reference, and failure/restore status. Live runtime output includes raw backend messages, transient progress, and detailed protocol payloads.

## Worked Examples

### Rename

1. Request rename of symbol at `src/lib.rs:42:8` to `SessionConfig`.
2. Capability lookup allows rename for the language/backend but requires operator preview.
3. Preview lists edits across `src/lib.rs`, `src/session.rs`, and tests.
4. Operator approves preview fingerprint.
5. imp checkpoints affected files, applies normalized text edits, refreshes semantic diagnostics, and runs `cargo test -p target_crate`.
6. Receipt records success or verify failure with restore option.

### Organize Imports

1. Request organize imports for one file.
2. Policy allows auto-apply only if preview affects that file and contains import-order edits only.
3. Preview is fresh and small, so imp checkpoints the file and applies.
4. imp runs formatter/check as configured.
5. Receipt notes single-file change and verify result.

### Allowlisted Code Action

1. Backend proposes an allowlisted action such as adding a missing import.
2. imp validates action id, affected paths, and edit shape.
3. If preview includes unrelated edits, reject as policy drift.
4. Otherwise follow approval/checkpoint/apply/refresh/verify sequencing according to posture.

## Implementation Notes

- Preview, checkpoint, approval, apply, refresh, verify, and receipt are distinct states.
- Backend success never replaces explicit verification.
- Mana may store the final durable evidence summary; imp owns live runtime state, semantic backend lifecycle, and restore UX.
