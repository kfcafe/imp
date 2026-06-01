# Release surface inventory

## Cargo/package surface
Cargo.toml:2:name = "imp-install"
Cargo.toml:6:description = "Source-install shim so `cargo install --path .` and `uu install` work from the imp repo root"
Cargo.toml:11:name = "imp"
Cargo.toml:37:version = "0.3.0"
crates/imp-cli/Cargo.toml:2:name = "imp-cli"
crates/imp-cli/Cargo.toml:6:description = "Command-line interface for the imp coding agent"
crates/imp-cli/Cargo.toml:15:name = "imp"
crates/imp-cli/Cargo.toml:18:[features]
crates/imp-core/Cargo.toml:2:name = "imp-core"
crates/imp-core/Cargo.toml:6:description = "Agent engine for imp: loop, tools, sessions, hooks, context, and SDK"
crates/imp-core/Cargo.toml:74:[features]
crates/imp-core/Cargo.toml:83:name = "core_hot_paths"
crates/imp-gui/Cargo.toml:2:name = "imp-gui"
crates/imp-gui/Cargo.toml:6:description = "Native GUI for the imp coding agent"
crates/imp-gui/Cargo.toml:14:name = "imp-gui"
crates/imp-llm/Cargo.toml:2:name = "imp-llm"
crates/imp-llm/Cargo.toml:6:description = "Standalone multi-provider LLM streaming client"
crates/imp-lua/Cargo.toml:2:name = "imp-lua"
crates/imp-lua/Cargo.toml:6:description = "Lua extension runtime for imp"
crates/imp-tui/Cargo.toml:2:name = "imp-tui"
crates/imp-tui/Cargo.toml:6:description = "Terminal UI for the imp coding agent"
crates/imp-tui/Cargo.toml:13:[features]

## CLI commands
enum Commands {
    /// Run as an Agent Client Protocol stdio server
    Acp,
    /// Open the fullscreen terminal UI explicitly
    Tui,
    /// Open the viewer/inspector surface (planned; not fully implemented yet)
    View {
        /// Viewer area to open (planned: sessions, tree, logs, checkpoints)
        area: Option<String>,
    },
    /// Edit a guided subset of imp settings in the terminal
    Settings,
    /// Run the terminal-native setup wizard
    Setup,
    /// Log in to a provider. OAuth is supported for Anthropic, OpenAI/ChatGPT, and Kimi Code.
    Login {
        /// Provider to configure (`anthropic`, `openai`, `kimi`, or `kimi-code`). Defaults to anthropic.
        provider: Option<String>,
    },
    /// Save, list, or remove API credentials in secure imp auth storage
    Secrets {
        #[command(subcommand)]
        command: Option<SecretsCommand>,
        /// Provider/service to configure (e.g. tavily, exa, resend, my-service)
        provider: Option<String>,
    },
    /// Edit configuration
    Config,
    /// Enter the mana-aware operator namespace. Use `imp mana <unit-id>` to run one unit.
    #[cfg(feature = "mana-ui")]
    Mana(ManaNamespaceArgs),
    /// Local statistics from persisted imp sessions
    Stats {
        #[command(subcommand)]
        command: StatsCommand,
    },
    /// Usage reporting and export
    Usage {
        #[command(subcommand)]
        command: UsageCommand,
    },
    /// Inspect, validate, run, and update native workflow artifacts
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommand,
    },
    /// Open or inspect run evidence artifacts
    Evidence {
        #[command(subcommand)]
        command: Option<EvidenceCommand>,
    },
    /// Import skills and config from other agents (pi, Claude Code, Codex)
    Import {
        /// Only detect — don't copy anything
        #[arg(long)]
        dry_run: bool,
        /// Import from a specific agent: pi, claude, codex
        #[arg(long)]
        from: Option<String>,
        /// Skip the confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Install this build to the user-visible `imp` command path
    InstallLocal {
        /// Print the chosen install destination without writing it
        #[arg(long)]
        dry_run: bool,
        /// Explicit install destination path
        #[arg(long)]
        dest: Option<PathBuf>,
    },
    /// Save a web search provider API key into imp auth storage
    WebLogin {
        /// Search provider to configure (tavily, exa, linkup, perplexity)
        provider: String,
    },
}

#[derive(Subcommand, Debug)]
enum WorkflowCommand {

## README advertised surface
5:## Install
19:## Features
23:- terminal UI
24:- one-shot prompt mode
25:- JSONL RPC mode
26:- provider-flexible model runtime
27:- structured native tools
28:- durable JSONL sessions
29:- context compaction
30:- workflow-backed planning and verification
31:- trace/evidence artifacts
32:- OS-backed secret storage
33:- runtime policy for tools, writes, autonomy, and hooks
39:- workflow artifacts under `.imp/workflows`
40:- YAML workflow schema
41:- schema-checked workflow updates
42:- append-only workflow events
43:- next-action selection
44:- acceptance/check tracking
45:- results and closeout records
49:- Lua tools
50:- Lua slash commands
51:- Lua hooks
52:- extension capability policy
53:- preview Rust SDK
55:## Local data and provider traffic
59:- agent runtime
60:- TUI and RPC surfaces
61:- tool execution
62:- file reads/writes/edits
63:- shell commands
64:- git operations
65:- workflow files and event logs
66:- session JSONL records
67:- Lua hooks/extensions
71:- prompts
72:- selected context
73:- tool observations used for a turn
74:- web-search/read requests when web tools are used
78:| Path | Contents |
80:| `~/.config/imp/config.toml` | user config |
81:| `<project>/.imp/config.toml` | project config |
82:| `~/.imp/auth.json` | auth metadata |
83:| `.imp/workflows/` | workflow YAML, events, results, artifacts |
85:## Providers
89:- Anthropic
90:- OpenAI / ChatGPT
91:- Google
92:- OpenAI-compatible APIs
93:- Moonshot / Kimi
94:- Z.AI / GLM
95:- DeepSeek
96:- Groq
97:- Cerebras
98:- xAI
99:- Mistral
100:- Together
101:- OpenRouter
102:- Fireworks
123:## Tools
125:| Tool | Feature |
127:| `read` | ranged file/image reads |
128:| `write` | file creation/overwrite |
129:| `edit` / `multi_edit` | exact and transactional edits |
130:| `bash` | shell commands with timeout/cancellation |
131:| `git` | status, diff, log, stage, commit, restore, worktrees |
132:| `scan` | tree-sitter code search/extraction |
133:| `web` | web/GitHub search and page reads |
134:| `ask_user` | structured user prompts |
135:| `workflow` | workflow list/show/validate/run/update |
141:- read-only tools can run in parallel
142:- mutable tools are serialized
143:- runtime policy checks tool visibility and execution
144:- write-path policy checks file mutations
145:- autonomy policy controls unattended action level
147:## Workflows
163:- metadata
164:- parent workflow reference
165:- settings
166:- goal/user value
167:- non-goals
168:- acceptance criteria
169:- context requirements
170:- steps
171:- dependencies
172:- checks
173:- verification evidence
174:- workers
175:- results
176:- closeout rules
196:- validates the edited workflow before writing
197:- rejects oversized workflow YAML before parsing
198:- opens/preflights `events.jsonl`
199:- writes allowed status/path changes
200:- appends `events.jsonl`
201:- rejects invalid status values
202:- keeps workflow state file-backed and reviewable
206:- use the native `workflow` tool for list/show/validate/run/update from agent turns
207:- use workflows as the durable replacement for older `/plan`, `/run`, `/debug`, `/review`, and `/verify` task-type commands
208:- current runner selects and records next actions; executable build-step orchestration is tracked as follow-up work
212:## Sessions
216:- messages
217:- tool calls
218:- tool results
219:- usage metadata
220:- cost metadata
221:- branch metadata
222:- compaction entries
223:- checkpoint/recovery records
227:- `--verify` commands
228:- trace events
229:- evidence packets
230:- final outcomes: `DONE`, `DONE_WITH_CONCERNS`, `BLOCKED`, `NEEDS_CONTEXT`
237:## TUI controls
239:| Input | Feature |
241:| `/` | command palette |
242:| `@` | file context attachment |
243:| `Ctrl+L` | model selector |
244:| `Shift+Tab` | thinking level |
245:| `/compact` | context compaction |
246:| `/model` | model selector |
247:| `/settings` | settings editor |
248:| `/secrets` | credential manager |
249:| `/login` | provider OAuth login |
250:| `/new` / `/resume` | session lifecycle |
251:| `/loop` / `/stop` | continue or stop active work |
253:## RPC mode
272:## Policy
303:## Configuration
326:## Extensions
339:- tools
340:- slash commands
341:- hooks
342:- capability policy for shell/filesystem/HTTP/secrets/native tools
351:## Rust SDK
378:## Crates
388:## Status
392:- TUI
393:- one-shot prompts
394:- JSONL RPC mode
395:- native tools
396:- durable sessions
397:- file-backed workflows
398:- verification/evidence
399:- provider auth
400:- OS-backed secrets
401:- policy controls
402:- Lua extensions
403:- Rust SDK preview
407:- executable workflow runner for build-step orchestration
408:- MCP planned
409:- `.imp/agents` planned
410:- ACP editor adapter scaffold
411:- hosted sync/team collaboration planned
412:- workflow API planned
416:- legacy `mana` integration is optional and compatibility-oriented
417:- TypeScript/Pi extension compatibility is experimental and not a shipped extension surface
419:## Technical docs
421:- [Docs index](docs/index.md)
422:- [Workflows](docs/workflows.md)
423:- [ACP editor adapter](docs/acp.md)
424:- [RPC protocol](docs/rpc.md)
425:- [Native tools](docs/tools.md)
426:- [Runtime policy](docs/policy.md)
427:- [Sessions and evidence](docs/sessions.md)
428:- [Lua extensions](docs/extensions-lua.md)
429:- [Architecture](docs/architecture.md)
431:## Development
446:## License

## Crates
crates/imp-cli/Cargo.toml
crates/imp-core/Cargo.toml
crates/imp-gui/Cargo.toml
crates/imp-llm/Cargo.toml
crates/imp-lua/Cargo.toml
crates/imp-tui/Cargo.toml

## Docs
docs/acp.md
docs/architecture.md
docs/autonomy-modes.md
docs/child-workflow-delegation.md
docs/dependency-audit.md
docs/design/dirac-inspired-code-tools.md
docs/design/droid-gap-map-and-imp-roadmap.md
docs/design/droid-mission-mode-vs-imp-work-plan.md
docs/design/imp-host-sync-mirror-daemon.md
docs/design/imp-native-workflow-engine.md
docs/design/imp-semantic-write-execution-contract.md
docs/design/imp-work-global-store.md
docs/design/imp-work-implementation-plan.md
docs/design/imp-work-mana-feature-parity.md
docs/design/imp-work-mana-migration-plan.md
docs/design/imp-work-mana-removal-ledger.md
docs/design/imp-workflow-responsibility-boundaries.md
docs/design/lua-programmatic-interactions.md
docs/design/oss-launch-checklist.md
docs/eval-candidates.md
docs/extensions-lua.md
docs/imp-next-workflow-runtime.md
docs/index.md
docs/mana-next-compatibility-adapter.md
docs/mana-next-examples.md
docs/mana-next-migration-test-plan.md
docs/mana-next-runtime-event-mapping.md
docs/mana-next-storage-strategy.md
docs/mana-next-ux.md
docs/mana-next-workflow-ledger.md
docs/plans/pi-provider-oauth-parity.md
docs/policy.md
docs/proposals/guest-runtime-extension-substrate.md
docs/proposals/guest-runtime-implementation-plan.md
docs/proposals/imp-memory-architecture-and-mana-boundary.md
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md
docs/proposals/inline-mana-state-and-knowledge-surfaces.md
docs/proposals/mana-aware-runtime-context-read-path.md
docs/proposals/mana-wiki-schema-and-workflow.md
docs/proposals/script-tool-boundaries-and-policy.md
docs/proposals/tool-ab-harness-notes.md
docs/proposals/tool-review-2026-04.md
docs/rebuild/imp-attach-path-cutover.md
docs/rebuild/imp-bounded-subagent-orchestration.md
docs/rebuild/imp-durable-storage-surface-audit.md
docs/rebuild/imp-machine-streamed-error-envelope.md
docs/rebuild/imp-normalized-storage-contract.md
docs/rebuild/imp-output-mode-contract.md
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md
docs/rebuild/imp-rebuild-migration-sequence.md
docs/rebuild/imp-session-index-lifecycle-audit.md
docs/rebuild/imp-session-storage-search-recovery-audit.md
docs/rebuild/imp-shared-ui-event-seam.md
docs/rebuild/imp-workflow-feature-inventory.md
docs/rebuild/mana-embedding-surface-audit.md
docs/rebuild/mana-imp-contract-boundary-map.md
docs/reference-monitor-policy.md
docs/release-promotions/commit-board.html
docs/release-promotions/current.md
docs/role-registry.md
docs/rpc.md
docs/run-evidence.md
docs/runtime-event-state-api.md
docs/sessions.md
docs/tools.md
docs/trace-and-evidence-format.md
docs/trust-labels-and-provenance.md
docs/tui-workflow-wireframes.md
docs/typescript-extension-bridge.md
docs/verification-gates.md
docs/workflow-first-ux.md
docs/workflow-profiles.md
docs/workflows.md
docs/worktree-auto.md
