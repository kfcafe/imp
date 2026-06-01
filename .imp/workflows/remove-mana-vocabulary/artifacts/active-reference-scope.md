# Active reference scope decision

Decision: remove active mana support and optional features from imp.

Scope approved by user:

- Remove `mana-core` / `mana-cli` dependencies and optional feature gates.
- Remove active source files/modules that implement mana support.
- Remove TUI/CLI surfaces and settings for mana.
- Remove active docs/public README claims that mana is supported.
- Archive removed active support files to `~/imp-archive` with a manifest.

Historical changelog/workflow/event artifacts can be handled separately after active code/docs are clean.
