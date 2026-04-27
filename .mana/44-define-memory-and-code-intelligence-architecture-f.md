---
id: '44'
title: Define memory and code-intelligence architecture for reliable autonomous workers
slug: define-memory-and-code-intelligence-architecture-f
status: open
priority: 1
created_at: '2026-04-08T08:06:11.948470Z'
updated_at: '2026-04-08T08:06:11.948470Z'
labels:
- architecture
- memory
- code-intelligence
- mana
- imp
kind: epic
feature: true
---

Goal: turn two major open architecture questions into concrete, research-backed design work for Tower: (1) what agentic memory should mean in mana, and (2) how native code intelligence tools should fit into the mana ↔ imp contract.

Current state:
- `VISION.md` now centers reliable autonomy, durable memory, runtime policy, and isolated multi-agent execution.
- Root unit `28.3` already tracks memory/retrieval consolidation across mana and imp, but it does not yet explicitly include comparative research against existing agent memory systems.
- Existing repo evidence points to a likely gap around native code tooling: AST tools exist or are being discussed, linters/verifiers are used in verify gates, and `imp/IMP_REVIEW.md` explicitly calls for future LSP/diagnostic intelligence.
- The product direction suggests hybrid systems: agents should use real deterministic code tools where they help, while mana persists only the durable parts that should survive a run.

Desired outcome:
- Tower has a research-backed direction for agentic memory.
- Tower has a clear position on LSP/diagnostics/linters/AST/code-intelligence tools and their boundary with mana.
- Follow-on implementation planning can be grounded in concrete docs rather than open-ended discussion.

In scope:
- root-level architecture and research work
- cross-project mana ↔ imp boundary implications
- design docs that another worker can build from cold

Out of scope:
- implementing LSP integration now
- changing storage schemas or runtime code in this parent unit
- wizard/familiar planning beyond deferred reference use
