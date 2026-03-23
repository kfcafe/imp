# imp

AI agent engine. Tools, modes, context management, multi-provider LLM streaming.

```
  ╔╗    ╔╗
  ║╚════╝║
  ║ ■  ■ ║
╔═╩══════╩═╗
║    imp    ║
╚══════════╝
```

## What it does

imp runs AI coding agents. You give it a task, it reasons through it, calls tools, and gets work done.

- **Agent loop** — ReAct-style: think → act → observe → repeat
- **Tool system** — file read/write/edit, shell execution, grep, web search, AST-aware code search
- **Agent modes** — full, worker, orchestrator, planner, reviewer, auditor — each with different tool permissions and execution-time enforcement
- **Context management** — observation masking, LLM compaction, sliding window — fits long sessions into finite context windows
- **Multi-provider LLM** — Anthropic, OpenAI, Google, AWS Bedrock, xAI, Groq, and more via a unified streaming interface
- **Session persistence** — conversations save to disk and can be resumed, forked, or replayed
- **Hooks** — before/after tool calls, file writes, LLM calls — configurable via TOML or programmatic registration
- **Shell backends** — traditional `sh -c` or in-process execution via [rush](https://github.com/kfcafe/rush) for zero-fork built-in commands

## Crates

```
imp/
├── crates/
│   ├── imp-llm/    # Multi-provider LLM client (standalone)
│   ├── imp-core/   # Agent engine, tools, sessions, hooks
│   ├── imp-tui/    # Terminal UI (interactive mode)
│   ├── imp-cli/    # CLI binary
│   └── imp-lua/    # Lua scripting integration
```

**`imp-llm`** — Standalone LLM client. Streaming, prompt caching, model registry, OAuth. Works independently of the agent engine.

**`imp-core`** — The agent engine. Agent loop, tool registry, builder pattern, roles, modes, context management, session persistence, system prompt assembly, hook system.

**`imp-tui`** — Terminal UI with streaming output, input editing, slash commands, settings panel.

**`imp-cli`** — Entry point. Interactive TUI, headless task execution (`imp run <unit-id>`), RPC mode for integration.

## Modes

Modes control what an agent is allowed to do, enforced at both tool registration and execution time.

| Mode | Tools | Purpose |
|------|-------|---------|
| `full` | everything | Interactive use, trusted user |
| `worker` | read + write + bash | Execute a task, no coordination |
| `orchestrator` | read + mana + ask | Plan and delegate, no file writes |
| `planner` | read + mana (create) + ask | Decompose work, human approves |
| `reviewer` | read + ask | Read-only analysis |
| `auditor` | read + mana (read-only) | Inspect and report |

```toml
# .imp/config.toml
mode = "orchestrator"
```

Or via environment: `IMP_MODE=worker imp run 5.1`

## Configuration

Layered config, each level overrides the previous:

1. Built-in defaults
2. `~/.config/imp/config.toml` — personal defaults
3. `<project>/.imp/config.toml` — repo-shared settings
4. Environment variables (`IMP_MODEL`, `IMP_MODE`, `IMP_THINKING`)
5. CLI flags

```toml
# ~/.config/imp/config.toml
model = "sonnet"
thinking = "medium"
max_turns = 100

[context]
observation_mask_threshold = 0.6
compaction_threshold = 0.8
mask_window = 10

[shell]
backend = "rush"  # or "sh" (default)
```

## Providers

| Provider | Models |
|----------|--------|
| Anthropic | Claude Sonnet, Haiku, Opus |
| OpenAI | GPT-4o, o3, o3-mini |
| Google | Gemini 2.5 Pro, Flash |
| AWS Bedrock | Claude via Bedrock |
| xAI | Grok 3, Grok 2 |
| Groq | Llama 3.3 70B |

## License

Apache-2.0
