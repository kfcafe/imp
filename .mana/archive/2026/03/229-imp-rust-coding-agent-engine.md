---
id: '229'
title: imp — Rust coding agent engine
slug: imp-rust-coding-agent-engine
status: closed
priority: 0
created_at: '2026-03-19T08:52:04.251440Z'
updated_at: '2026-03-19T09:22:22.321189Z'
closed_at: '2026-03-19T09:22:22.321189Z'
verify: cd /Users/asher/imp && cargo build --workspace 2>&1 | tail -5
is_archived: true
history:
- attempt: 1
  started_at: '2026-03-19T09:22:22.321850Z'
  finished_at: '2026-03-19T09:22:23.841099Z'
  duration_secs: 1.519
  result: pass
  exit_code: 0
outputs:
  text: |-
    |
        = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

    warning: `imp-core` (lib) generated 4 warnings (run `cargo fix --lib -p imp-core` to apply 3 suggestions)
        Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.81s
---

Top-level bean for the imp project. A coding agent engine built in Rust with Lua extensibility, native tree-sitter, and ratatui TUI. See imp_core_plan.md for the full technical specification.

Crate structure:
- imp-llm: Standalone LLM client (Anthropic, OpenAI, Google)
- imp-core: Agent engine (loop, tools, sessions, hooks, context)
- imp-lua: Lua extension runtime (mlua bridge)
- imp-tui: Terminal UI (ratatui)
- imp-cli: Binary entry point

Build order: imp-llm types → Anthropic provider → tools → agent loop → sessions → context → more providers → OAuth → hooks → tree-sitter → config → system prompt → Lua → shell tools → TUI → CLI → mana integration
