# Tighten imp product surface — results

Status: completed with staged-backend concerns

## What changed

- Removed `crates/imp-gui` from workspace `default-members` while keeping it explicitly buildable.
- Tightened the TUI visible slash command surface to launch-focused commands.
- Removed/hid legacy or experimental TUI command dispatch/help for `/fork`, `/copy`, `/status`, `/eval`, `/improve`, `/mana`, `/scope`, `/run`, `/plan`, `/clean`, `/queue`, `/personality`, `/memory`, checkpoints, and export/session aliases.
- Removed CLI chat shell surface and dead chat-shell runtime/parser code.
- Removed CLI personality/soul editor surface and helpers.
- Removed CLI eval candidate surface and TUI eval command path.
- Stopped default prompt assembly from injecting personality, soul, memory, or user-profile blocks.
- Updated README to describe the tighter launch product: TUI, one-shot, JSONL RPC, native tools, workflows, durable sessions, auth/secrets, Lua extensions, and SDK preview.
- Fixed `imp-lua` Unix process-group compilation by adding the missing `CommandExt` import and `libc` dependency.

## Verification performed

Final verification passed:

```sh
cargo check -p imp-core -p imp-tui -p imp-cli -p imp-lua
cargo test -p imp-tui command_palette --lib
cargo test -p imp-tui slash_unknown --lib
cargo test -p imp-cli parse_tool_output_display --lib
```

Focused diff reviewed:

```sh
git diff --stat -- README.md Cargo.toml crates/imp-cli/src/lib.rs crates/imp-core/src/builder.rs crates/imp-lua/Cargo.toml crates/imp-lua/src/bridge.rs crates/imp-tui/src/app.rs crates/imp-tui/src/views/command_palette.rs
```

Diff size captured at `/tmp/tighten-imp-product-surface-final.diff` for local review: 2751 lines.

## Remaining concerns

- Some compatibility internals remain staged rather than fully deleted, especially mana/improve-related TUI/backend code and optional CLI mana feature gates. They are no longer part of the visible launch command/help/palette surface.
- `crates/imp-tui/src/views/personality.rs` still exists and is compiled because fully deleting personality UI internals was unsafe in this slice after a failed broad removal attempt. Product access is removed from the visible command surface; deeper file deletion should happen as a separate focused cleanup.
- Memory backend/config code remains for compatibility, but default prompt assembly no longer injects memory or user profile content and the memory command/tool/product surface is removed from active docs/help/palette.

## Suggested next tasks

1. Focused deletion of orphaned personality TUI view and remaining personality backend/config types once no references remain.
2. Focused deletion or compatibility isolation of improve/mana TUI internals after workflow runner replacement is ready.
3. Archive old root/docs experimental artifacts to `~/imp-archive` in a dedicated docs/archive cleanup slice.
4. Implement executable workflow runner via `.imp/workflows/implement-executable-workflow-runner/workflow.yaml` so future workflows can execute build steps directly.
