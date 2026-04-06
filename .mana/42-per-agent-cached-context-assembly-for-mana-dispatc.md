---
id: '42'
title: Per-agent cached context assembly for mana dispatch
slug: per-agent-cached-context-assembly-for-mana-dispatc
status: open
priority: 0
created_at: '2026-04-01T06:36:07.304781Z'
updated_at: '2026-04-01T06:36:07.304781Z'
labels:
- epic
- context
- cache
- mana
verify: 'cd /Users/asher/tower && cargo test -p imp-core context_prefill 2>&1 | tail -5 | grep -q "test result: ok" && cargo test -p imp-llm cache 2>&1 | tail -5 | grep -q "test result: ok"'
kind: epic
---

## Problem

When `mana run` dispatches agents, each agent starts cold — it has to spend 3-5 turns calling `read` on files that the orchestrator already identified in the unit description. Those read calls cost output tokens and add latency, and the file contents aren't in the cached prefix so they're re-sent on every subsequent turn.

Meanwhile, the unit description already contains file paths, code snippets, and type signatures. The orchestrator did the thinking — the worker just needs the context delivered efficiently.

## Solution

Add a **context prefill** system: at dispatch time, read the files referenced in the unit, assemble them into a structured cached message that precedes the task prompt. The agent starts with all relevant files already in its cached context.

## Architecture

### The dispatch flow becomes:

```
System prompt: [identity + AGENTS.md + RULES.md + tools]     ← cached (unchanged)
User msg 1:    [assembled file context from unit]              ← NEW, cached via cache_control
Assistant 1:   [prefill: "Context loaded. Ready to work."]     ← NEW, prefill
User msg 2:    [unit task prompt]                              ← existing task_prompt()
→ Agent turn 1: model already has all files, starts working
→ Agent turn 2+: cache_read on system + tools + file context
```

### Key design decisions:

1. **File declaration**: Units get an optional `files:` frontmatter field. Also auto-detect file paths from the description body (regex for common patterns like `src/foo.rs`, `crates/x/y.rs`).

2. **Assembly modes per file**:
   - `full` (default) — entire file content, up to a per-file budget
   - `signatures` — type/function signatures only (future: tree-sitter extraction)
   - `tail:N` — last N lines (useful for test files to show patterns)
   - `snippet:L1-L2` — specific line range

3. **Budget management**:
   - Total context prefill budget: configurable, default 50K tokens (~200KB)
   - Per-file cap: 10K tokens (~40KB) — prevents one huge file from eating the budget
   - When over budget: truncate largest files first, with a note about truncation
   - Files are assembled in declaration order (most important first)

4. **Injection point**: New `context_prefill` field on `SessionOptions` — a `Vec<Message>` injected before the first prompt in `ImpSession::prompt()`. The last message in the prefill gets `cache_control: ephemeral` with extended TTL.

5. **Cache strategy**: The prefill messages use the cache options from the agent config. For mana agents specifically, use `extended_ttl: true` so the cache survives across turns (agent sessions are short — 5-30 minutes — well within the 1-hour TTL).

## Components

### 1. Unit file declaration (mana-core)

Add optional `files` field to unit frontmatter:

```yaml
---
title: Add validation to auth.rs
verify: cargo test auth::validation
files:
  - src/auth.rs
  - src/auth/types.rs
  - src/auth/tests.rs:tail:50
---
```

When `files:` is absent, auto-detect from the description body by scanning for paths matching `[a-zA-Z_][a-zA-Z0-9_/.-]*\.(rs|ts|py|go|js|tsx|toml|yaml|yml|json|md)`.

### 2. Context assembler (imp-core)

New module: `crates/imp-core/src/context_prefill.rs`

```rust
pub struct PrefillConfig {
    /// Max total tokens for assembled context. Default: 50_000.
    pub budget_tokens: usize,
    /// Max tokens per individual file. Default: 10_000.
    pub per_file_tokens: usize,
    /// Whether to use extended TTL on cache. Default: true for mana.
    pub extended_cache_ttl: bool,
}

pub struct FileSpec {
    pub path: PathBuf,
    pub mode: FileMode,
}

pub enum FileMode {
    Full,
    Tail(usize),
    Range(usize, usize),
}

pub struct AssembledContext {
    /// Messages to inject before the first prompt.
    pub messages: Vec<Message>,
    /// Files that were included (for logging/debugging).
    pub included_files: Vec<PathBuf>,
    /// Files that were skipped or truncated (for logging).
    pub warnings: Vec<String>,
    /// Estimated token count of the assembled context.
    pub estimated_tokens: usize,
}

/// Assemble context from file specs, reading from disk and respecting budgets.
pub fn assemble_context(
    specs: &[FileSpec],
    cwd: &Path,
    config: &PrefillConfig,
) -> AssembledContext { ... }

/// Auto-detect file paths from a unit description string.
pub fn detect_file_paths(description: &str) -> Vec<FileSpec> { ... }
```

The assembler:
1. Reads each file from disk (relative to cwd)
2. Applies the file mode (full/tail/range)
3. Tracks cumulative token estimate (1 token ≈ 4 chars)
4. Truncates or skips files that exceed budget
5. Builds a single user message with structured file contents:

```xml
<context>
<file path="src/auth.rs">
fn validate_email(email: &str) -> bool {
    ...full file content...
}
</file>
<file path="src/auth/types.rs">
pub struct AuthToken {
    ...
}
</file>
<file path="src/auth/tests.rs" note="last 50 lines">
    #[test]
    fn test_valid_email() { ... }
</file>
</context>
```

6. Returns `AssembledContext` with the message(s) and metadata

### 3. SessionOptions integration (imp-core)

Add to `SessionOptions`:
```rust
pub struct SessionOptions {
    // ... existing fields ...
    /// Pre-assembled context messages injected before the first prompt.
    /// These messages get cache breakpoints for efficient multi-turn sessions.
    pub context_prefill: Option<Vec<imp_llm::Message>>,
}
```

In `ImpSession::prompt()`, inject prefill messages before the user's prompt:
```rust
if let Some(prefill) = &self.context_prefill {
    for msg in prefill {
        agent.messages.push(msg.clone());
    }
    // The prefill is only injected once (first prompt)
    self.context_prefill = None;
}
```

### 4. Headless dispatch wiring (imp-cli)

In `run_headless_mode()`, between loading the unit and creating the session:

```rust
let unit = load_mana_unit(&cwd, unit_id)?;

// Assemble context from unit file references
let file_specs = unit.file_specs().unwrap_or_else(|| {
    imp_core::context_prefill::detect_file_paths(&unit.description)
});
let prefill_config = PrefillConfig::default(); // 50K budget
let assembled = imp_core::context_prefill::assemble_context(&file_specs, &cwd, &prefill_config);

for warning in &assembled.warnings {
    eprintln!("[imp] context prefill: {warning}");
}

let options = SessionOptions {
    context_prefill: Some(assembled.messages),
    // ... rest unchanged ...
};
```

### 5. Cache breakpoint on prefill (imp-llm)

The Anthropic provider needs to place a `cache_control` breakpoint on the last content block of the prefill user message. This is already supported by the `cache_recent_turns` mechanism — the prefill counts as a user turn.

The existing `cache_recent_turns: 2` in the default cache options already handles this: the prefill user message is the first user turn, and the task prompt is the second — both get cache breakpoints.

For mana agents, also set `extended_ttl: true` so the cache persists across the full agent session.

## What this does NOT include

- Tree-sitter signature extraction (`signatures` mode) — future enhancement, not needed for v1
- Cross-agent cache sharing — each agent has its own prefilled context
- Context from dependency outputs — future enhancement
- Dynamic context refresh mid-session — not needed, files are read once at dispatch

## Testing

- `test_context_prefill_assembles_files` — reads files, builds message with correct structure
- `test_context_prefill_budget_enforcement` — large files get truncated, total stays within budget
- `test_context_prefill_detect_file_paths` — extracts `.rs`, `.ts`, `.py` paths from description text
- `test_context_prefill_missing_file_warning` — nonexistent files produce warnings, not errors
- `test_context_prefill_tail_mode` — `tail:50` returns last 50 lines
- `test_context_prefill_injection` — prefill messages appear before task prompt in agent messages
- `test_context_prefill_empty_when_no_files` — no files = no prefill messages (zero overhead)

## Files to create/modify

**Create:**
- `crates/imp-core/src/context_prefill.rs` — assembler, file specs, budget management

**Modify:**
- `crates/imp-core/src/lib.rs` — export `context_prefill` module
- `crates/imp-core/src/imp_session.rs` — add `context_prefill` to SessionOptions, inject in prompt()
- `crates/imp-cli/src/main.rs` — wire up in `run_headless_mode()`, parse `files:` from unit frontmatter
