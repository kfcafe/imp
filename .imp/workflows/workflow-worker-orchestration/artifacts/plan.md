# Workflow worker orchestration plan

## Current state

The workflow tool already exposes worker metadata on `run`: when the next step has a `worker`, the response includes role, writes, worktree, responsibilities, and step checks. The remaining orchestration slice should make that output actionable enough for an agent to dispatch a bounded worker without inventing instructions.

## Implementation plan

1. Extend workflow run output with a concise assignment prompt for worker-backed steps.
2. Include workflow id, step id/kind, dependencies, checks, writable scope, and required result artifact in the assignment.
3. Render the assignment in human-readable tool text and structured details.
4. Add focused tests using dogfood workflows to lock the worker assignment contract.
5. Verify with the narrowest Rust test target for workflow tooling.

## Non-goals

- No background worker spawning yet.
- No git worktree creation yet.
- No delegation outside the current imp runtime/tool contract.
