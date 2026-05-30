# Workflow profiles

Workflow profiles power imp's task slash commands. They are one-turn prompt wrappers: they guide the next response without putting the TUI into a hidden sticky mode.

Built-in profiles:

- `/plan` — plan without editing.
- `/review` — review only.
- `/verify` — run or inspect checks and report evidence.
- `/implement` — make a focused change and verify it.
- `/research` — gather evidence and answer.
- `/debug` — diagnose before fixing.

The normal UI should stay terse. Prefer action labels such as `Save plan`, `Create task`, `Record decision`, `Capture evidence`, `Run verification`, and `Close out`.

## Configure workflows

Built-ins can be overridden and custom workflows can be added in config:

```toml
[workflows.security-review]
description = "Review the current change for security issues."
aliases = ["sec"]
suggest = "ask" # ask | auto | never
readonly = true
tools = ["read", "rg", "bash"]
triggers = ["security review", "audit auth", "check auth"]
confirm_title = "Use /security-review?"
confirm_body = "Review only"
role = "reviewer"
instructions = """
Task workflow: security review

User request:
{{prompt}}

Instructions:
- Review only; do not edit files.
- Focus on auth, secrets, injection, permissions, unsafe IO, and dependency risk.
- Cite evidence for every finding.
- If no issues are found, say what was checked.
"""
```

Then users can invoke:

```text
/security-review audit the auth changes
/sec audit the auth changes
```

Natural-language suggestions use `triggers`. For example, `can you audit auth?` can produce concise UI like:

```text
Use /security-review?
[Use /security-review] [Normal]
```

## Fields

- `description`: short list/detail text.
- `aliases`: extra slash command names.
- `suggest`: `ask`, `auto`, or `never` for natural-language suggestions.
- `readonly`: declares expected edit behavior for UI/policy.
- `tools`: optional tool policy hint.
- `triggers`: natural-language phrases for suggestions.
- `confirm_title`: concise suggestion title.
- `confirm_body`: concise action/body text.
- `role`: optional internal/advanced role hint.
- `instructions`: prompt wrapper. Supports `{{prompt}}`.

## Architecture note

Workflow profiles are backend-neutral. They should use imp-native-ready concepts such as plan, task, evidence, decision, verification, artifact, and closeout. Current workflow compatibility should stay behind adapters; normal users should not need to understand workflow terminology.
