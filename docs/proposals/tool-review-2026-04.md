# Tool Review — Revised Proposal

**Date:** 2026-04-04
**Status:** Draft v2
**Author:** imp + asher

## Philosophy

> "I don't agree with telling agents to not do something. I would rather fix the issue than tell them to avoid something whenever possible."

The right approach isn't to put guardrails in the system prompt saying "don't use bash for X." It's to make the native tools unnecessary by making bash itself good enough that the model's natural instinct to `grep`, `ls`, `find` **just works** — and then to only keep dedicated tools where they genuinely add something bash can't do.

## The Core Insight

Models are trained to use shell commands. Rush already has fast, .gitignore-aware, agent-mode-capable builtins for `grep`, `ls`, `find`, `cat`, etc. imp currently wraps these same operations in custom Rust tools (GrepTool, LsTool, FindTool) that:

1. Duplicate what rush already does
2. Have a different UX than what models expect (custom query syntax, tree-sitter modes, etc.)
3. Add tool count to the LLM context (each tool schema costs tokens + decision complexity)

**If bash (via rush) already does ls/find/grep well, the native tools are unnecessary overhead.**

## What to Defunct

### `ls` → bash

Rush's `ls` builtin:
- .gitignore-aware (via `ignore` crate)
- JSON output in agent mode
- Handles `-la`, hidden files, symlink targets, permissions, timestamps
- Already faster than GNU ls

imp's `ls` tool:
- Flat directory listing, no flags, no depth, no metadata
- The model already knows `ls -la` but has to learn a custom tool schema

**Verdict: Remove.** `ls` via bash is strictly better. The model already knows how to use it.

### `find` → bash

Rush's `find` builtin:
- Parallel traversal (ignore crate WalkBuilder)
- .gitignore-aware by default
- Glob patterns, file type filters, size filters, mtime filters, depth limits
- JSON output in agent mode

imp's `find` tool:
- Delegates to `fd` when available, falls back to glob crate
- Glob pattern only, no filters
- Custom result format

**Verdict: Remove.** `find` / `fd` via bash is more capable. The model knows `find . -name "*.rs"` natively.

### `diff` → bash (or remove entirely)

imp's `diff` tool (show + apply):
- `diff show` — generates unified diff between file and proposed content
- `diff apply` — applies a unified diff patch

In practice, the model almost always uses `edit` for modifications. `diff show` is occasionally useful but `diff file1 file2` via bash covers the comparison case. `diff apply` is a niche operation that `patch` via bash handles.

**Verdict: Remove.** If needed, `diff` and `patch` are available via bash.

## What to Keep (with changes)

### `bash` — the backbone

Keep as primary shell tool. Changes:

1. **Add `workdir` parameter** — all three reference implementations support per-command cwd. Avoids `cd dir && cmd` patterns.
2. **Improve description** — don't tell the model what not to do. Instead, make the description clearly state its capability: "Execute any shell command. The shell has fast builtins for grep, find, ls, cat, and other common operations."
3. **Consider `description` parameter** — Hermes doesn't have this but Codex does implicitly via justification. A lightweight way for the model to annotate what a command is doing, useful for UI/logging. (Optional, low priority.)

### `grep` → bash

Rush's `grep` builtin uses **ripgrep internals** (`grep-searcher`, `grep-matcher`, `grep-regex` crates) + `ignore` crate for .gitignore-aware walking. It supports:
- `-r` recursive search
- `-i` case-insensitive
- `-C`/`-A`/`-B` context lines
- `--hidden`, `--no-ignore`
- JSON output in agent mode
- Parallel file walking

It is effectively `rg` as a shell builtin. The model already knows `grep -r "pattern" . --include="*.rs"`.

imp's `grep` tool by contrast has a custom parameter schema (`pattern`, `glob`, `ignoreCase`, `blocks`, `extract`, boolean queries with AND/OR/NOT, stemming) that the model has to learn from scratch. The tree-sitter features (`blocks`, `extract`) belong in `scan`, not grep.

**Verdict: Remove.** `grep -r` via bash/rush is what models are trained on. The tree-sitter extract/blocks features move to `scan`.

### `scan` — absorb extract/blocks, become the code intelligence tool

Scan already does structural extraction (types, functions, impls). It should absorb grep's tree-sitter features:

New `scan` actions:
- `scan` (existing) — scan directory for code structure
- `build` (existing) — build index
- `extract` (new, from grep) — extract code blocks at `file:line`, `file:start-end`, or `file#symbol`

The `blocks` mode from grep (search + find enclosing block) could become an option on extract, or it could just be dropped — the model can `grep` to find the line, then `scan extract` to get the enclosing block.

### `read` — keep as-is

Reading files with offset/limit, line numbers, image support — this is genuinely better as a dedicated tool than `cat`. The structured output with line numbers is valuable for edit workflows.

### `write` — keep as-is

Creating files with auto-mkdir, line ending preservation, overwrite warnings. Better as a tool.

### `edit` — keep as-is

Find-and-replace with fuzzy matching, diff output, CRLF handling, unread-file warnings. This is imp's strongest tool — nothing in bash does this.

### `ask`, `web`, `mana`, `extend`, `memory`, `scan` — keep

These have no bash equivalent.

## New: Repeated-call Detection

Add tracking at the agent loop level:

```
Track (tool_name, hash(args)) for the last N calls.
If the same (tool, args) pair appears 3+ times consecutively:
  - 3x: append warning to tool output
  - 4x: return error instead of executing
Reset counter when a different tool+args is called.
```

This prevents post-compression loops and is cheap to implement.

## Resulting Tool Set

### Before (16 tools)

```
ask, bash, diff, diff_show, diff_apply, edit, extend, find, grep, ls,
mana, memory, read, scan, session_search, web, write
```

### After (11 tools)

```
ask, bash, edit, extend, mana, memory, read, scan,
session_search, web, write
```

**Removed:** diff, diff_show, diff_apply, find, grep, ls (6 tools)

**Modified:**
- `bash` — add workdir, update description
- `scan` — absorb extract action from grep

## Impact

**Fewer tools = less decision complexity for the model.** Each tool schema in the LLM context is ~100-200 tokens. Removing 6 tools saves ~600-1200 tokens per request and reduces the chance of the model picking the wrong tool.

**Models use their trained behavior.** When a model wants to list files, it reaches for `ls -la`. Currently that goes through bash → rush → ls builtin and works great. There's no reason to also have a native `ls` tool that the model has to learn separately.

**Rush becomes a real differentiator.** Instead of rush being a hidden backend that the model doesn't know about, it becomes the reason imp's bash tool is better than everyone else's. Rush's builtins are faster, .gitignore-aware, and produce structured output — and the model gets all of that for free just by using normal shell commands.

## Execution Plan

| Step | Change | Risk |
|---|---|---|
| 1 | Remove `ls`, `find`, `diff`, `grep` from tool registry | Low — bash/rush covers these |
| 2 | Add `extract` action to `scan` (move tree-sitter code from grep) | Medium — move code |
| 3 | Add `workdir` to `bash` | Low — simple parameter |
| 4 | Add repeated-call detection | Low — agent loop change |
| 5 | Update bash description | Low — text change |
| 6 | Delete dead code (old tools) | Low — cleanup |

Step 1 is the big bang — unregister 6 tools in one commit. Steps 2-5 can follow independently.

## Open Questions

1. **Should `scan extract` support the `blocks` semantic (search + find enclosing block) or just positional extraction?** Recommendation: just positional extraction. The two-step workflow (grep → scan extract) is clearer and more composable.

2. **Rush agent mode** — rush builtins already support JSON output via `RUSH_JSON=1`. Should imp set this automatically when running bash commands? This would give structured output from rush's ls/find/grep builtins. Could make bash output more parseable. Worth exploring.

3. **Truncation gap** — the native tools had built-in truncation (2000 lines / 50KB). Bash already has this, but when the model runs `grep -r pattern .` and gets 10,000 lines, the bash tool's tail-truncation kicks in. Is tail-truncation the right default for search output? (Probably yes for builds, maybe not for grep.) Rush's `--max-output` could help here.

4. **Output quality** — rush builtins produce clean, readable output. But models sometimes parse output poorly when it's not structured. The native grep tool produced `file:line:content` format that models reliably parse. Worth monitoring whether grep-via-bash produces output the model handles well. If not, we could add a thin wrapper or rely on rush's JSON agent mode.
