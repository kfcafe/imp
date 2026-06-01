# imp Architecture

This document captures architecture-specific guidance for `imp` beyond the quick product overview in `README.md`.

## Mana is imp's delegation substrate

When `imp` needs to hand work to another agent, it should create a **mana child job** under the current parent job. `imp` should not invent a separate subtask or todo model for delegated work. The mana job is the delegated-worker contract, the execution container, and the completion record.

## Delegated child-job contract

Author delegated child jobs so another agent can complete them in one pass when practical.

### Standard description shape

Use a short operational description with these parts, in this order when possible:

1. **Goal / current state**
   - State the outcome needed.
   - State the relevant current condition or gap.
   - Keep this specific to the parent job's immediate need.

2. **Scope boundaries**
   - Say what is in scope.
   - Say what is explicitly out of scope.
   - Keep the child job narrow enough that success or failure is easy to judge.

3. **Expected deliverable**
   - Name what the parent should get back: code change, doc update, investigation note, repro, verify result, or recommendation.
   - Prefer one primary deliverable.

4. **Patch or no-patch guidance**
   - Explicitly say whether the child should modify files.
   - If no patch is wanted, say that the job is analysis-only or reporting-only.
   - If a patch is wanted, say whether small targeted edits or a contained implementation are expected.

5. **Important files or subsystem focus**
   - Name the key files, directories, APIs, or subsystem when known.
   - Omit this section if the parent truly does not know yet.

6. **Done condition and verify expectations**
   - State what must be true for the child to be considered complete.
   - Include the verify command if there is one, or say `no additional verify` when the parent only needs analysis.
   - Keep verify aligned with mana's normal completion model: the job is done when its stated condition is satisfied and its verify expectation is met.

### Authoring rules

- Prefer **one sharply scoped outcome** per child job.
- Write for a fresh worker that only knows the parent context in the job text.
- Ask for work that is **useful to the parent immediately after completion**.
- Prefer child jobs that are **executable in one pass** over open-ended exploration.
- If a task naturally splits into multiple independent outcomes, create multiple child jobs instead of one blended brief.
- Do not restate a large plan; describe the delegated slice of work.
- Do not use the description to invent a second orchestration layer. Parent/child structure, status, notes, and verify stay in mana.

### Recommended description template

```md
## Goal
<what needs to happen, and what is true now>

## Scope
- In scope: <bounded work>
- Out of scope: <explicit non-goals>

## Deliverable
- <expected artifact/result for the parent>

## Patch Guidance
- <patch required | no patch | small targeted patch only>

## Focus
- <important files, directories, APIs, or subsystem>

## Done / Verify
- Done when: <completion condition>
- Verify: <command or "no additional verify">
```

### Example

```md
## Goal
Document the delegated child-job contract for imp-authored mana work. The current docs say imp should use mana for delegation, but they do not define the standard child-job description shape.

## Scope
- In scope: concise documentation updates for the contract and template.
- Out of scope: runtime orchestration changes, new planner state, or behavior changes outside prompt guidance.

## Deliverable
- A short docs update that defines the delegated child-job contract and a reusable template.

## Patch Guidance
- Patch required; keep edits small and documentation-focused.

## Focus
- `ARCHITECTURE.md`
- `README.md`
- `crates/imp-core/src/system_prompt.rs` only if a brief prompt hint is warranted

## Done / Verify
- Done when the docs clearly define how imp should author mana child jobs for delegated work.
- Verify: `test -n "design-only"`
```
