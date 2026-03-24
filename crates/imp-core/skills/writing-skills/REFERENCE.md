# Writing Skills for Imp

Skills are markdown instruction files that get listed in the system prompt. The agent reads them on demand when a task matches the skill's description.

## File Layout

**User-global** (available in all projects):
```
~/.config/imp/skills/<name>/SKILL.md
```

**Project-local** (available only in this project):
```
.imp/skills/<name>/SKILL.md
```

## Format

Every SKILL.md has YAML frontmatter and a markdown body:

```markdown
---
name: skill-name
description: >
  One to three sentences explaining what the skill does and when to use it.
  This text is shown in the system prompt — it's how the agent decides
  whether to load the skill. Be specific about trigger conditions.
---

# Skill Title

## When to Use

- Bullet list of situations where this skill applies

## Instructions

The body is free-form markdown. Write it as instructions to yourself:
clear, direct, actionable. The agent reads this verbatim when it loads
the skill.

## Examples

Include concrete examples — code snippets, command patterns, templates.
Examples are the most useful part of a skill.
```

## Naming Rules

- Lowercase letters, numbers, and hyphens only
- No leading/trailing hyphens, no consecutive hyphens
- Max 64 characters
- Examples: `rust`, `deploy-k8s`, `pr-review`, `docker-workflows`

## The Description is Critical

The description in frontmatter is the **only thing** the agent sees before deciding to load the skill. Write it to answer: "When should I read this file?"

**Good descriptions:**
- "Conventions for writing and reviewing Rust code. Use when creating, modifying, or reviewing .rs files."
- "Systematic debugging through interactive diagnosis. Use when the user reports a bug, error, unexpected behavior, or says something is broken."
- "HTTP requests and API testing with curl. Use when testing APIs, downloading files, or debugging HTTP issues."

**Bad descriptions:**
- "Useful utilities" (too vague — when would the agent load this?)
- "Stuff about testing" (doesn't say what kind, what language, when)
- A 500-word essay (wastes system prompt space)

**Pattern:** `<What it does>. Use when <trigger conditions>.`

## Writing the Body

### Be directive, not descriptive
```markdown
# Bad — describes what exists
The project uses ESLint with the Airbnb config.

# Good — tells the agent what to do
Run `npx eslint --fix` after modifying any .ts file.
Lint rules: Airbnb config. Don't disable rules without explaining why.
```

### Include verify gates
```markdown
## Verify Gate
\`\`\`bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
\`\`\`
All three must pass before committing.
```

### Include anti-patterns
```markdown
## Don't
- Don't use `unwrap()` outside tests
- Don't add dependencies without asking
- Don't modify files not mentioned in the task
```

### Keep it focused
One skill = one domain. Don't combine "Rust conventions" and "deployment" in one skill. If you need both, make two skills.

## Skill Lifecycle

1. **Create**: Use the `skill_manage` tool with action `create`
2. **Update**: Use `skill_manage` with action `patch` (find-and-replace)
3. **Delete**: Use `skill_manage` with action `delete`
4. **Test**: After creating, ask the agent to do a task that should trigger the skill. Check if it loads and follows the instructions.

## Porting Skills from Other Agents

Skills from pi, Claude Code, and other agents use the same format (YAML frontmatter + markdown). To port:

1. Run `imp import --dry-run` to see what's available
2. Run `imp import` to copy compatible skills
3. Review imported skills — some may reference tools or paths specific to the source agent
4. Patch any agent-specific references using `skill_manage`

Common things to fix when porting:
- Tool names that differ between agents (pi's `probe_search` → imp's `bash` + probe CLI)
- Config paths (`~/.pi/` → `~/.config/imp/`)
- Agent-specific instructions ("use the read tool to load..." works in any agent)
