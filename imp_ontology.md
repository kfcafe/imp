# imp ontology

Status: draft
Purpose: short shared language for aligning imp going forward

## Core framing

- **mana** — the underlying platform/substrate. Apps and interfaces are built on mana.
- **imp** — the flagship Rust-native agent product and primary human-facing environment on mana.
- **app** — a product built on mana.
- **SDK** — the embedding/programmatic surface for using mana and imp in other apps.

## Platform terms

Use simple code/function names for mana internals unless we later choose stronger names.

- **runtime** — the live execution layer.
- **graph** — the durable layer.
- **extension system** — the packaged extensibility layer.
- **adapter layer** — the connector layer for outside systems.

## Product terms

- **wizard** — the visual/operator app built on mana.
- **familiar** — the team workflow app built on mana.
- **imp** — the flagship agent-facing product and default interface on mana.

## Work terms

- **task** — the preferred term for a bounded piece of work.
- **work item** — a broader durable work object; use when the concept is bigger than one executable task.
- **run** — one bounded execution.
- **attempt** — a task-bound run with history and outcome.
- **session** — persisted interaction history.
- **thread** — the human-facing conversation or interaction thread.
- **handoff** — information another run, agent, or app should inherit.

## Runtime terms

- **context** — the information assembled for a run or turn.
- **memory** — information that persists beyond the immediate turn.
- **agent loop** — the observe / reason / act loop.
- **tool** — a runtime-callable interface the agent can invoke.
- **action** — the preferred term for something the system can do or a concrete request performed by the system.
- **approval** — a required permission step before a consequential action or state change.
- **review** — human inspection or evaluation; narrower than approval.
- **workflow loop** — imp's explicit product loop of `/ask → /plan → /work → /improve`.

## Graph terms

- **object** — the stable addressable thing in the graph.
- **assertion** — a typed statement about an object.
- **fact** — a narrower verified assertion; not the general term for all durable statements.
- **event** — an immutable record that something happened.
- **rule** — a constraint, authorization rule, derivation rule, or workflow rule.
- **workflow** — a structured multi-step process over graph, actions, and approvals.
- **projection** — a derived view such as a queue, board, dashboard, or report.
- **artifact** — a stored file, document, bundle, or output that can be cited or inspected.
- **evidence** — support for a claim, action, or result.

## Extensibility terms

- **extension** — the main packaged unit of extensibility.
- **adapter** — a connector to an external service, runtime, or protocol.
- **TypeScript extension** — the preferred future extension path.
- **Lua extension** — legacy path; likely deprecated if TypeScript extensions land well.

## Preferred usage

- Prefer **mana** for the platform, not just the durable graph layer.
- Prefer **imp** for the flagship agent product and primary human-facing environment.
- Prefer **task** over **unit**.
- Prefer **assertion** over **fact** when speaking generally.
- Prefer **action** over **capability**.
- Prefer **extension** over **module**.
- Prefer **runtime**, **graph**, and **extension system** as the default internal layer names.
- Avoid **shell** as a core ontology term unless it becomes necessary later.
- When needed, distinguish **mana's runtime substrate** from **imp's agent runtime experience** rather than overloading `runtime` to mean both at once.

## Short naming direction

- `mana` = platform
- `imp` = flagship Rust-native agent product + default human-facing environment
- `runtime` = live execution layer
- `graph` = durable layer
- `task` = executable work
- `memory` = durable recall
- `artifact` = stored durable output
- `evidence` = support for claims and actions
- `approval` = permission object / gate
- `action` = what the system can do / one concrete request
- `extension` = packaged extensibility
- `adapter` = outside-world connector
