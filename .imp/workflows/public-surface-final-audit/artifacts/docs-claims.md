# Docs claims audit

Audited against `command-surface.md` captured from `./target/debug/imp --help` and subcommand help.

## Command surface facts

- Primary interactive surface: `imp tui`; `imp chat` is a legacy alias.
- One-shot/headless prompt mode: root `imp -p/--print`.
- RPC/event surfaces: root `--mode rpc|json`, `--runtime-json`, and `imp acp` for ACP stdio.
- Current subcommands include: `acp`, `tui`, `settings`, `setup`, `login`, `secrets`, `config`, `stats`, `usage`, `workflow`, `evidence`, `import`, `install-local`, `web-login`.
- Planned/stub subcommands visible in help: `mcp`, `view`.

## Stale or risky docs claims found

- `docs/dependency-audit.md` documented old `mana-core`/`serde_yml` advisories that no longer match the dependency scan.
- `docs/autonomy-modes.md` mentioned bash-equivalent mana blocking, which is no longer an active surface.
- `docs/runtime-event-state-api.md` documented `mana_updated` and mana refs in runtime state examples.
- `docs/worktree-auto.md` referenced mana/evidence refs and old mana detection helpers.
- `docs/trust-labels-and-provenance.md` contained mana ledger/provenance examples inconsistent with the current workflow surface.
- `docs/eval-candidates.md` contained mana-unit examples/refs.
- README currently labels MCP as planned and ACP scaffold as internal/out-of-scope; that matches command help and is acceptable.
- README labels TypeScript/Pi extension compatibility as experimental/not shipped; that matches release scope and is acceptable.

## Fix policy

Update active docs to workflow terminology or remove obsolete stale references. Historical/proposal docs already archived are out of scope.
