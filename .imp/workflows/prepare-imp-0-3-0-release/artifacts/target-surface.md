# imp 0.3.0 target surface

Supported for 0.3.0 RC, based on current README/Cargo/CLI inventory:

- CLI install shim (`cargo install --path .`) and `imp` binary
- TUI (`imp`, `imp tui`)
- one-shot prompt mode (`imp -p ...`)
- JSONL RPC mode (`imp --mode rpc`)
- native tools and runtime policy
- durable sessions, evidence, usage/stats reporting
- file-backed workflows through CLI and native workflow tool
- provider/auth paths for Anthropic and OpenAI-compatible API-key users
- Lua extension runtime
- preview Rust SDK, explicitly documented as preview

Out of 0.3.0 launch scope unless audit proves polished:

- ACP/editor adapter: current repo contains new untracked ACP files and README/docs promises. Treat as out of scope for the stable HN release until separately verified; docs should not present it as stable.
- GUI crate: experimental and not in default workspace members; keep buildable but not advertised as stable.
- legacy mana compatibility: optional compatibility only, not public launch surface.
- TypeScript/Pi extension compatibility: explicitly not shipped.
- hosted sync/team collaboration, workflow API, MCP, `.imp/agents`: planned only.
