# Engineering Guardrails for imp

## Purpose

Engineering guardrails help imp produce code that is easier to reason about, easier to verify, and less likely to leave a project in a broken or ambiguous state.

This feature is for ordinary software projects. It is inspired by safety-critical coding ideas, but it is **not** a special safety-critical mode for imp.

The design goal is simple:

- keep the rule surface small
- prefer mechanically checkable behavior over subjective doctrine
- make better defaults cheap
- keep project-specific proof of correctness in verify gates

## Goals

Guardrails should bias agent behavior toward:

- bounded execution
- explicit error handling
- small, focused changes
- warning-free, buildable code
- language-aware starter checks
- clearer handoff from agent output to project verification

## Non-goals

Guardrails are **not** meant to be:

- a giant policy engine
- a branded "Power of Ten" feature
- a replacement for project tests or mana verify gates
- a proof of full correctness through prompt instructions alone
- a complete style guide for every language

## Core model

Guardrails are an **agent-time** feature in imp.

They do three things:

1. inject concise, profile-aware guidance into the system prompt
2. run configured checks after writes when relevant files change
3. surface failures clearly, optionally blocking forward progress

Guardrails should stay small and practical. A profile is not a doctrine pack. It is a compact bundle of:

- short guidance bullets for the agent
- starter check commands
- optional default scope assumptions

## Config model

Project config should support a top-level `[guardrails]` table in `.imp/config.toml`.

Proposed shape:

```toml
[guardrails]
enabled = true
level = "advisory"  # off | advisory | enforce
profile = "auto"    # auto | generic | zig | rust | typescript | c | go | elixir
critical_paths = ["src/**", "lib/**"]
after_write = []      # optional override; empty = use profile defaults
```

### Fields

- `enabled`: master switch
- `level`: how strongly guardrails affect execution
- `profile`: built-in starter profile or `auto`
- `critical_paths`: file globs that decide when write-triggered checks should run
- `after_write`: explicit command override; if omitted or empty, imp uses built-in defaults for the effective profile

## Levels

### `off`

No guardrail prompt layer and no guardrail-triggered checks.

### `advisory`

- inject guardrail guidance into the prompt
- run configured checks after matching writes
- show failures clearly to the agent and user
- do **not** silently ignore failures
- do **not** block the session outright

This is the default recommendation for most projects.

### `enforce`

- inject the same guidance
- run configured checks after matching writes
- treat failed guardrail checks as blocking for that write/turn
- make the failure visible enough that the agent can fix it intentionally

This is for projects that want stronger discipline, especially in critical paths.

## Profiles

Guardrails are a project-agnostic imp feature. Profiles are just starter bundles.

Initial built-in profiles:

- `generic`
- `zig`
- `rust`
- `typescript`
- `c`
- `go`
- `elixir`

### What a built-in profile includes

Each built-in profile should define:

1. prompt guidance bullets
2. default `after_write` checks
3. optional default path/check assumptions when useful

Profiles should stay honest. If imp cannot provide real starter checks and useful guidance for a profile, that profile should not ship yet.

## Initial built-in profiles

### Generic

Use when no stronger profile is available.

Guidance themes:

- keep control flow straightforward
- keep loops, retries, and timeouts bounded
- make error handling explicit
- prefer small, focused changes
- leave code easy to verify and warning-free

Default checks:

- none built in; project config should provide them

### Zig

This should be the most polished initial profile.

Guidance themes:

- handle errors explicitly with `try` or intentional `catch`
- avoid casual `catch unreachable`
- keep allocator ownership and lifetime clear
- keep loops, retries, and buffers bounded
- prefer small, readable functions
- avoid hidden control flow and unnecessary cleverness in critical paths

Starter checks:

```bash
zig fmt --check .
zig build
zig build test
```

### Rust

Guidance themes:

- make errors explicit with `Result` and meaningful propagation
- avoid hidden panics in library code
- keep async and retry behavior bounded
- prefer small functions and simple control flow
- leave code warning-free and clippy-clean

Starter checks:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

### TypeScript

This profile should assume package-manager-based projects and use available scripts.

Guidance themes:

- avoid hidden `any`-style escapes when stricter typing is intended
- make error handling explicit
- keep async flows bounded and understandable
- prefer small, focused changes over broad rewrites
- leave typecheck/lint/test status clean

Starter expectations:

- use the detected package manager
- prefer project scripts when present
- try to run a typecheck script, lint script, and test script when available

Starter checks, conceptually:

```bash
<pkg-manager> run typecheck
<pkg-manager> run lint
<pkg-manager> test
```

The exact commands may need to adapt to the detected project and available scripts.

### C

This profile should stay practical and acknowledge that build system matters as much as language.

Guidance themes:

- keep control flow easy to follow
- keep loops and retries bounded
- make error handling explicit
- avoid macro-heavy or pointer-obscuring code when simpler code works
- leave build/test status clean

Starter expectations:

- use project kind to choose checks
- CMake, Meson, and Make projects may need different defaults

The design should not prevent future project-kind-specific behavior for C-family projects.

### Go

Guidance themes:

- check and propagate errors explicitly
- keep goroutine lifecycle and cancellation understandable
- keep retries and timeouts bounded
- prefer small functions and direct control flow
- leave formatting, vet, and tests clean

Starter checks:

```bash
gofmt -l .
go vet ./...
go test ./...
```

### Elixir

Guidance themes:

- keep process and supervision boundaries clear
- handle `{:ok, value}` / `{:error, reason}` style flows explicitly
- avoid hiding important behavior in opaque control flow
- keep message flows and retries understandable
- leave formatting, warnings, and tests clean

Starter checks:

```bash
mix format --check-formatted
mix compile --warnings-as-errors
mix test
```

## `profile = "auto"`

`profile = "auto"` should use the existing `project-detect` crate.

It should not rely on hand-rolled file sniffing in imp.

The first version should map supported detected project kinds onto built-in profiles where the mapping is obvious, for example:

- Zig project -> `zig`
- Cargo project -> `rust`
- Go project -> `go`
- Elixir project -> `elixir`
- Node/TypeScript project -> `typescript` when appropriate

If detection cannot confidently resolve a specific built-in profile, imp should fall back to `generic` rather than pretending certainty.

## Boundaries

### imp owns agent-time guardrails

imp should own:

- config parsing
- effective profile resolution
- prompt guidance injection
- write-triggered check execution
- advisory vs enforce behavior

### mana owns completion-time proof

Mana verify gates remain the source of truth for task completion.

Guardrails can improve code quality during execution, but they do not prove a unit is done.

### uu informs defaults, but is not a runtime dependency

uu-style project commands are useful design inspiration for starter checks.

But imp should not depend on uu runtime behavior for guardrails.

The dependency for auto-detection should stay at the `project-detect` level.

## Rollout

### Phase 1

- add config types and profile resolution
- support `off`, `advisory`, and `enforce`
- support `auto` via `project-detect`

### Phase 2

- add prompt-layer integration
- run profile/default checks after writes
- surface failures clearly in advisory and enforce levels

### Phase 3

- ship the initial built-in profile set
- document the feature in `README.md`
- polish Zig most deeply first
- keep Rust, TypeScript, C, Go, and Elixir honest and useful as starter profiles

## Example config

### Zig repo

```toml
[guardrails]
enabled = true
level = "advisory"
profile = "auto"
critical_paths = ["src/**", "lib/**"]
```

This should resolve to the Zig profile automatically and use the built-in Zig checks unless the project overrides them.

### Explicit override example

```toml
[guardrails]
enabled = true
level = "enforce"
profile = "zig"
critical_paths = ["src/**"]
after_write = [
  "zig fmt --check .",
  "zig build test",
]
```

The same model should work for Rust, TypeScript, C, Go, and Elixir by changing `profile` and optionally overriding the check commands.
