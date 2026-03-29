---
id: '31'
title: Add configurable engineering guardrails to imp
slug: add-configurable-engineering-guardrails-to-imp
status: closed
priority: 1
created_at: '2026-03-26T04:43:27.656392Z'
updated_at: '2026-03-29T22:30:28.704227Z'
notes: |-
  ---
  2026-03-29T22:12:17.252169+00:00
  Backlog review note: core guardrail types and prompt wiring already exist in code. Prioritize verifying what remains and narrowing unfinished scope to profile polish, runtime checks, and docs rather than reopening the design wholesale.

  ---
  2026-03-29T22:12:28.767611+00:00
  Backlog review note: core guardrail types and prompt wiring already exist in code. Prioritize verifying what remains and narrowing unfinished scope to profile polish, runtime checks, and docs rather than reopening the design wholesale.

  ---
  2026-03-29T22:12:28.914928+00:00
  Backlog review note: core guardrail types and prompt wiring already exist in code. Prioritize verifying what remains and narrowing unfinished scope to profile polish, runtime checks, and docs rather than reopening the design wholesale.
labels:
- design
- verification
- guardrails
- language-profiles
closed_at: '2026-03-29T22:30:28.704227Z'
close_reason: Umbrella already decomposed into 31.1 through 31.4. Dry-run showed the parent epic itself was being scheduled; close the umbrella so only the child jobs execute.
verify: grep -q "^# Engineering Guardrails for imp" ENGINEERING_GUARDRAILS.md && grep -q "GuardrailConfig" crates/imp-core/src/guardrails.rs && grep -q "guardrails_layer" crates/imp-core/src/system_prompt.rs && cargo check -p imp-core
fail_first: true
is_archived: true
kind: epic
---

Implement first-class, configurable engineering guardrails in stock imp to help coding agents produce better, safer, and easier-to-verify code in ordinary software projects. Do not present this feature as "Power of Ten" in user-facing config, docs, or code unless citing inspiration in prose. Safety-critical literature can inform the design, but this is not a special safety-critical mode for imp. The product concept is that imp should absorb the underlying advice: bounded execution, explicit error handling, zero-warning culture, small understandable units, and language-aware guidance/checks.

Constraints:
1. This must be built into stock imp, not as a one-off local extension.
2. Behavior must be adjustable via config: at minimum support off, advisory, and enforce-style behavior.
3. Prefer a small set of mechanically enforceable checks and behaviors over a large policy engine or a long subjective rule list.
4. Ship more than one built-in language profile. The initial built-in set should cover Zig, Rust, TypeScript, C, Go, and Elixir. Zig should be the most polished initial profile since we actively use imp on Zig projects, but the others should also have honest starter built-in guidance/check behavior.
5. Reuse the existing `project-detect` workspace dependency for `profile = auto` instead of inventing ad-hoc detection.
6. Keep the implementation inside imp boundaries: config + prompt + hooks + docs/examples. Do not turn this into a cross-project refactor.
7. Keep user-facing terminology simple: prefer "engineering guardrails" or "guardrails".

Expected outcome:
- imp has a documented guardrail feature with clear config.
- imp-core can load guardrail config, derive a profile, inject concise guidance into the system prompt, and run configured checks after writes.
- guardrails focus on practical, mechanically checkable quality and verification improvements rather than safety-critical branding.
- projects can opt into general guardrails with built-in starter profiles for Zig, Rust, TypeScript, C, Go, and Elixir.
- Zig projects get especially useful defaults out of the box or via small config.

Decompose into focused child units for design, config types, runtime integration, and profile/docs work.
