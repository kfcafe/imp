---
id: '23'
title: 'Learning loop: agent-curated memory, skill management, session search'
slug: learning-loop-agent-curated-memory-skill-managemen
status: open
priority: 2
created_at: '2026-03-23T19:59:21.752687Z'
updated_at: '2026-03-23T19:59:21.752687Z'
labels:
- feature
- learning-loop
verify: cd /Users/asher/tower && cargo check -p imp-core 2>&1 | tail -1 | grep -q "could not compile" && exit 1; cargo test -p imp-core memory_store session_index skill_manage learning 2>&1 | grep -E "(test result|FAILED)" | grep -v "0 passed" | head -1
---

Implement the closed learning loop for imp, inspired by Hermes Agent. 

See LEARNING_LOOP_SPEC.md for the full technical specification.

Three core capabilities:
1. Agent-curated persistent memory (memory.md + user.md) with a memory tool
2. Skill self-management (create/patch/delete skills via skill_manage tool)
3. Session search (FTS5 index over past conversations)

Plus the glue: learning loop nudges in system prompt and OnAgentEnd hooks.

## Implementation order

1. Memory store + memory tool (no deps)
2. System prompt Layer 6 (depends on 1)
3. Skill manage tool (no deps, parallel with 1-2)
4. Learning nudges — system prompt text + OnAgentEnd hook + config (depends on 1, 2, 3)
5. Session index with rusqlite FTS5 (independent)
6. Session search tool (depends on 5)
