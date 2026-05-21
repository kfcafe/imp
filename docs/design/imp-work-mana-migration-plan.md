# imp-work mana migration plan

`imp-work` is now a native foundation for agent-local work, but imp is not ready to remove mana yet. Mana still owns durable project graphs, existing `.mana` data, verify/close/fail semantics, scheduling, and much of the operator workflow. The migration must be adapter-backed and reversible.

## Goal

Move imp’s default task orchestration from mana-backed workflows to native `imp-work` while preserving existing mana projects until parity is proven.

## Non-goals

- No immediate deletion of the `mana` tool, `mana-core` dependency, or `.mana` compatibility.
- No flag-day conversion of existing `.mana` projects.
- No direct destructive mutation from imp-work into mana during the first migration slices.
- No claim that imp-work replaces mana until parity tests pass on real project units.

## Phase 1: compatibility adapter design

Define a mana-to-imp-work adapter that maps:

- unit id/title/description/design/acceptance;
- status and blockers;
- dependencies;
- labels/priority/paths;
- notes and decisions;
- verify command and timeout;
- artifacts and evidence summaries;
- attempts/failure/retry context.

The adapter output should be an imp-work work item plus context pack, not a mana mutation.

## Phase 2: shadow import

Add a shadow import path for selected mana units:

1. read a mana unit and immediate graph context;
2. materialize an imp-work item in an isolated store/scope;
3. compare readiness, blockers, verify command, paths, acceptance, and dependency summaries;
4. report parity gaps without changing `.mana` state.

First fixture: one closed/simple unit and one open unit with verify/dependency metadata.

## Phase 3: outcome bridge

Add an outcome bridge from imp-work back to mana-compatible proposals:

- completed -> proposed close/verify evidence;
- blocked -> proposed blocker/fail note;
- failed check -> proposed recovery/debt note;
- artifacts -> proposed durable evidence references.

The bridge initially emits proposals only. A human/operator or existing mana tool path applies them.

## Phase 4: route new local work through imp-work first

For new imp-local tasks, prefer imp-work creation and scheduling. Keep mana commands available for existing `.mana` project work.

Routing rule:

- If a user explicitly names a mana unit or `.mana` graph, use compatibility/shadow bridge.
- If a user asks for new local agent work without a mana unit, use imp-work native store.

## Phase 5: parity gates

Before deprecating mana-backed paths in imp, prove parity for:

- dependency readiness;
- blocked/open/closed status;
- verify commands and timeouts;
- notes/decisions preservation;
- acceptance criteria;
- paths/artifacts;
- failure/retry semantics;
- close/fail proposal generation;
- durable evidence summaries.

## Phase 6: deprecation and removal

Only after parity:

1. mark mana-native imp tool usage as compatibility mode;
2. provide migration/export guidance for `.mana` projects;
3. add warnings or routing hints for new work;
4. remove mana runtime dependency only after existing project graphs have a supported bridge.

## First implementation slice

Implement a read-only shadow import fixture:

- add adapter types under `crates/imp-work` or `crates/imp-core` integration layer;
- load a mana unit fixture into an imp-work item;
- assert mapped acceptance, verify, paths, dependencies, notes, and status summary;
- do not write to mana;
- run `cargo test -p imp-work` and focused integration tests.

## Decision

We can start preparing to remove mana from imp by building the adapter and shadow import. We should not remove mana from imp yet.
