---
id: '17'
title: 'imp efficiency: enable prompt caching'
slug: imp-efficiency-enable-prompt-caching
status: open
priority: 0
created_at: '2026-03-22T23:58:41.214809Z'
updated_at: '2026-03-22T23:58:41.214809Z'
verify: 'cd /Users/asher/tower && grep -q ''cache_system_prompt: true'' imp/crates/imp-core/src/agent.rs && grep -q ''cache_tools: true'' imp/crates/imp-core/src/agent.rs'
---

## Problem
Prompt caching infrastructure exists in imp-llm (CacheOptions, cache breakpoints on system prompt, last tool def, recent turns) but is never enabled. `CacheOptions::default()` sets all fields to `false`.

## Impact
~3,000+ tokens (system prompt + tool definitions) are sent uncached on every single turn. With caching, these get a 90% cost reduction after turn 1. For a 20-turn session, this saves ~54K tokens of billing.

## Implementation
In `imp/crates/imp-core/src/agent.rs` line 251, change:
```rust
cache_options: Default::default(),
```
to:
```rust
cache_options: CacheOptions {
    cache_system_prompt: true,
    cache_tools: true,
    cache_recent_turns: 2,
},
```

Also consider making these configurable via `config.toml`:
```toml
[cache]
system_prompt = true
tools = true
recent_turns = 2
```

## Files
- `imp/crates/imp-core/src/agent.rs` — set cache options
- `imp/crates/imp-core/src/config.rs` — optional: add cache config section
- `imp/crates/imp-llm/src/provider.rs` — CacheOptions struct (already correct)
- `imp/crates/imp-llm/src/providers/anthropic.rs` — cache breakpoint logic (already correct)

## Acceptance
- CacheOptions has system_prompt=true, tools=true, recent_turns=2
- Anthropic API responses show `cache_read_input_tokens` > 0 after turn 1
