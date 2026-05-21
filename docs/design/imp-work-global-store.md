# imp-work Global Project-Scoped Store

## Decision

imp-work should use a single user-global work store by default, with `project_root` recorded as first-class metadata on stored work records. This avoids mana's cwd-scoped `.mana` fragmentation where work created from `~` is invisible when imp is later launched from `~/imp`, and vice versa.

## Chosen topology

Default global root:

```text
~/.imp/work
```

Project-scoped work records include:

- `project_root`: canonical or normalized project path at creation/import time;
- `task`: native imp-work task payload;
- future equivalents for memory, decisions, runs, events, and prototypes.

The current markdown `WorkStore` can remain useful for local/project stores and compatibility, but the migration direction is a global store API that can filter by project root.

## Query behavior

- Current-project view: detect the active project root and filter records by `project_root`.
- Global view/search: query all records across all projects.
- Explicit project view: caller passes a `project_root` to filter another project.
- Imported `.mana` records should preserve source refs and record the project root of the source graph.

## Alternatives considered

### Independent project-local stores

Rejected as the default because it recreates the `.mana` cwd fragmentation problem. It can be retained as an import/export or project-local backup format.

### Single store without project metadata

Rejected because global work would become ambiguous and hard to filter safely.

### Multiple stores with discovery search

Rejected as the primary model because discovery order and cwd behavior remain surprising. It can be a migration fallback only.

## Migration policy

1. Import old `.mana` graphs into the global store with `project_root` set to the source project.
2. Import existing project-local `.imp/work` stores into the global store if found.
3. Never delete source stores automatically.
4. Report imported/skipped/lossy records.
5. Make the active store/source visible in work-tool output.

## Implementation status

Initial implementation slice adds a `GlobalWorkStore` wrapper in `imp-work` that stores project-scoped task records in a single JSONL file and supports filtering by project root. This proves the core model without forcing an immediate migration of every existing markdown store API.
