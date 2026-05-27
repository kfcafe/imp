# Native tools

imp exposes structured tools to the model. Native tools provide narrower operations than shell-only automation and are easier to validate, display, constrain, and audit.

Primary implementation areas:

- `crates/imp-core/src/builder.rs`
- `crates/imp-core/src/tools/`
- `crates/imp-tui/src/views/tools.rs`
- `crates/imp-tui/src/views/tool_output.rs`

## Tool inventory

| Tool | Purpose |
|---|---|
| `read` | ranged file/image reads |
| `write` | file creation/overwrite |
| `edit` / `multi_edit` | exact and transactional edits |
| `bash` | shell commands with timeout/cancellation |
| `git` | status, diff, log, stage, commit, restore, worktrees |
| `scan` | tree-sitter code search/extraction |
| `web` | web/GitHub search and page reads |
| `ask_user` | structured user prompts |
| `prototype` | disposable experiments with evidence |
| `workflow` | workflow list/show/validate/run/update |
| `memory` | persistent agent memory |

## Mutability

Read-only tools can run in parallel. Mutable or side-effecting tools are serialized and checked by runtime policy.

Mutable operations include file writes, edits, shell commands, git mutation, workflow updates, and secret-affecting actions.

## Policy interaction

Tool execution is affected by:

- tool allow/deny lists
- write allow/deny patterns
- autonomy mode
- role tool policy
- hooks
- verification gates

The model may see a tool in the registry but still be blocked by policy at execution time.

## Display

The TUI renders compact tool summaries in the chat timeline and expanded output in the sidebar. Tool-specific formatters exist for common tools where raw JSON would be noisy.

Workflow tool calls render as workflow actions with the `⚑` icon and action-specific details such as workflow id, path, value, and reason.

## Shell use

`bash` remains available when policy allows it. Prefer native tools for file reads, exact edits, git inspection, structural search, workflow status, and user questions.
