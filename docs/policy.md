# Runtime policy

Runtime policy constrains what the agent can do. It applies to tools, writes, autonomy, hooks, and verification gates.

Primary implementation areas:

- `crates/imp-core/src/policy.rs`
- `crates/imp-core/src/roles.rs`
- `crates/imp-core/src/hooks.rs`
- `crates/imp-cli/src/lib.rs`

## CLI controls

```bash
imp -p "inspect this diff" --deny-tool bash --deny-write '**'
imp -p "fix the failing test" --allow-write crates/imp-core --verify "cargo test -p imp-core"
```

Common flags:

```text
--allow-tool <name>
--deny-tool <name>
--allow-write <glob>
--deny-write <glob>
--autonomy <mode>
--verify <command>
```

## Autonomy modes

Common autonomy modes:

```text
suggest
safe
local-auto
worktree-auto
allow-all-local
allow-all
ci
```

Autonomy mode controls how much work can proceed without additional confirmation. More permissive modes should be paired with narrower write paths and explicit verify commands.

## Tool policy

Tool policy can allow or deny tools by name. Role policy can also restrict available tools before a run starts.

Deny rules should be used for broad safety boundaries. Allow rules should be used when a run needs a narrow permitted surface.

## Write policy

Write policy constrains file mutation by path pattern. It applies to tools that create, edit, delete, or otherwise mutate files.

Use write allow-lists for targeted automation. Use deny patterns for lockfiles, generated files, secrets, or high-risk project areas.

## Hooks

Hooks can observe or influence runtime behavior. Hook capability and execution should be treated as part of the policy surface because hooks can affect tool execution and runtime flow.

## Verify gates

`--verify` registers commands that prove a task after agent work. Verification gates are part of the completion contract; they should not be weakened to make a run pass.
