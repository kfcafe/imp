# HN docs gap audit

Audited README and top-level docs for HN-facing launch readiness against the 0.3.0 public surface.

## Good enough / already covered

- README explains what imp is, basic install, quickstart commands, local data, provider traffic, credential storage, providers, native tools, workflows, sessions/evidence, RPC, policy, config, Lua extensions, crates, status, and technical docs.
- README labels preview/planned/internal surfaces explicitly: MCP, `.imp/agents`, ACP scaffold, hosted sync/team collaboration, workflow API, Rust SDK preview, and TypeScript/Pi compatibility.
- `docs/index.md` links the core reference pages.
- `docs/acp.md` is explicit that ACP is scaffold-level and not daily editor use.
- `docs/extensions-lua.md` says Lua is the shipped extension runtime and TypeScript is not the stable shipped extension system.
- `docs/workflows.md` says API-addressable workflows are planned, not shipped.

## Gaps to fix for HN visitors

- README install section only shows Homebrew. Add a source-install fallback because early readers may not have the tap available or may want to try the current checkout.
- README does not clearly state prerequisites for source install/build.
- README quickstart commands are present but not grouped as a short "try it now" path.
- README status section is accurate but could better distinguish shipped, preview/planned, and compatibility surfaces for drive-by readers.
- Technical docs pages are intentionally concise; no large docs rewrite is required for HN readiness.

## Risky claims reviewed but acceptable

- `imp acp` appears in command help but README/docs label it as scaffold/internal for 0.3.0.
- MCP is visible as a planned command and README labels it planned.
- TypeScript/Pi extension compatibility appears in docs but is labeled compatibility/future rather than shipped stable extension support.
