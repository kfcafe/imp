# Command surface capture

Captured from `./target/debug/imp --help` and subcommand help after `cargo build -q -p imp-install`.

## root

```text
Command-line interface for the imp coding agent

Usage: imp [OPTIONS] [ARGS]... [COMMAND]

Commands:
  chat           Open the terminal chat interface (legacy alias for `tui`)
  acp            Run as an Agent Client Protocol stdio server
  tui            Open the fullscreen terminal UI explicitly
  mcp            Manage Model Context Protocol server connections (planned)
  view           Open the viewer/inspector surface (planned; not fully implemented yet)
  settings       Edit a guided subset of imp settings in the terminal
  setup          Run the terminal-native setup wizard
  login          Log in to a provider. OAuth is supported for Anthropic, OpenAI/ChatGPT, and Kimi Code
  secrets        Save, list, or remove API credentials in secure imp auth storage
  config         Edit configuration
  stats          Local statistics from persisted imp sessions
  usage          Usage reporting and export
  workflow       Inspect, validate, run, and update native workflow artifacts
  evidence       Open or inspect run evidence artifacts
  import         Import skills and config from other agents (pi, Claude Code, Codex)
  install-local  Install this build to the user-visible `imp` command path
  web-login      Save a web search provider API key into imp auth storage
  help           Print this message or the help of the given subcommand(s)

Arguments:
  [ARGS]...  File arguments (@file includes file content)

Options:
  -p, --print <PRINT>                  Print response and exit (non-interactive mode)
      --provider <PROVIDER>            LLM provider (anthropic, openai, google)
  -m, --model <MODEL>                  Model to use (alias or full ID)
      --role <ROLE>                    Role to apply (planner, coder, verifier, reviewer, researcher, integrator)
      --thinking <THINKING>            Thinking level: off, minimal, low, medium, high, xhigh
      --api-key <API_KEY>              API key override
  -c, --cont                           Continue most recent session
  -r, --resume                         Browse and select a session to resume
      --session <SESSION>              Use a specific session file
      --no-session                     Ephemeral mode (no session persistence)
      --tools <TOOLS>                  Enable specific tools (comma-separated)
      --allow-tool <ALLOW_TOOLS>       Allow a tool by exact name for this run (repeatable)
      --deny-tool <DENY_TOOLS>         Deny a tool by exact name for this run (repeatable)
      --allow-write <ALLOW_WRITES>     Allow writes matching this path/glob relative to the worker cwd (repeatable)
      --deny-write <DENY_WRITES>       Deny writes matching this path/glob relative to the worker cwd (repeatable)
      --no-tools                       Disable all built-in tools
      --system-prompt <SYSTEM_PROMPT>  Replace default system prompt
      --mode <MODE>                    Output mode: interactive, rpc, json [default: interactive]
      --output <OUTPUT>                Final output format for --print: text or json [default: text]
      --runtime-json                   Emit shared runtime_event/runtime_state payloads alongside legacy JSON events
      --autonomy <MODE>                Autonomy mode: suggest, safe, local-auto, worktree-auto, allow-all-local, allow-all, ci
      --verify <COMMAND>               Verification command gate to require for closeout. Repeat for multiple gates
      --max-turns <MAX_TURNS>          Maximum turns before stopping (default: 50)
      --max-tokens <MAX_TOKENS>        Max output tokens per response
      --verbose                        Verbose startup logging
      --list-models                    List available models
  -h, --help                           Print help
  -V, --version                        Print version
```

## chat

```text
Open the terminal chat interface (legacy alias for `tui`)

Usage: imp chat

Options:
  -h, --help  Print help
```

## acp

```text
Run as an Agent Client Protocol stdio server

Usage: imp acp

Options:
  -h, --help  Print help
```

## tui

```text
Open the fullscreen terminal UI explicitly

Usage: imp tui

Options:
  -h, --help  Print help
```

## settings

```text
Edit a guided subset of imp settings in the terminal

Usage: imp settings

Options:
  -h, --help  Print help
```

## setup

```text
Run the terminal-native setup wizard

Usage: imp setup

Options:
  -h, --help  Print help
```

## login

```text
Log in to a provider. OAuth is supported for Anthropic, OpenAI/ChatGPT, and Kimi Code

Usage: imp login [PROVIDER]

Arguments:
  [PROVIDER]  Provider to configure (`anthropic`, `openai`, `kimi`, or `kimi-code`). Defaults to anthropic

Options:
  -h, --help  Print help
```

## secrets

```text
Save, list, or remove API credentials in secure imp auth storage

Usage: imp secrets [PROVIDER] [COMMAND]

Commands:
  list     List configured secret providers/services
  ls       Alias for list
  show     Show status for one configured provider/service
  inspect  Alias for show
  doctor   Verify that configured secrets are readable from secure storage
  remove   Remove a configured provider/service from secure storage
  rm       Alias for remove
  set      Configure or update a provider/service's secret fields
  help     Print this message or the help of the given subcommand(s)

Arguments:
  [PROVIDER]  Provider/service to configure (e.g. tavily, exa, resend, my-service)

Options:
  -h, --help  Print help
```

## config

```text
Edit configuration

Usage: imp config

Options:
  -h, --help  Print help
```

## stats

```text
Local statistics from persisted imp sessions

Usage: imp stats <COMMAND>

Commands:
  summary   Show overall local imp stats
  tokens    Show token and cost stats
  tools     Show stats grouped by tool
  files     Show file/code-change stats
  daily     Show stats grouped by day
  weekly    Show stats grouped by week
  projects  Show stats grouped by project/session directory hint
  sessions  Show stats grouped by session
  wrapped   Show a fun local imp wrapped summary
  export    Export local imp stats records in a machine-friendly format
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## usage

```text
Usage reporting and export

Usage: imp usage <COMMAND>

Commands:
  summary   Show overall usage totals
  daily     Show usage grouped by day
  models    Show usage grouped by model
  sessions  Show usage grouped by session
  export    Export usage records in a machine-friendly format
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## workflow

```text
Inspect, validate, run, and update native workflow artifacts

Usage: imp workflow <COMMAND>

Commands:
  list      List workflows under .imp/workflows
  show      Show a workflow summary
  validate  Validate one workflow, or all workflows when no id is provided
  run       Show the next runnable workflow step
  update    Update a workflow status path and append an audit event
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## evidence

```text
Open or inspect run evidence artifacts

Usage: imp evidence [COMMAND]

Commands:
  list    List recent run evidence records
  latest  Print the latest evidence HTML path
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## import

```text
Import skills and config from other agents (pi, Claude Code, Codex)

Usage: imp import [OPTIONS]

Options:
      --dry-run      Only detect — don't copy anything
      --from <FROM>  Import from a specific agent: pi, claude, codex
  -y, --yes          Skip the confirmation prompt
  -h, --help         Print help
```

## install local

```text
Install this build to the user-visible `imp` command path

Usage: imp install-local [OPTIONS]

Options:
      --dry-run      Print the chosen install destination without writing it
      --dest <DEST>  Explicit install destination path
  -h, --help         Print help
```

## web login

```text
Save a web search provider API key into imp auth storage

Usage: imp web-login <PROVIDER>

Arguments:
  <PROVIDER>  Search provider to configure (tavily, exa, linkup, perplexity)

Options:
  -h, --help  Print help
```

## mcp

```text
Manage Model Context Protocol server connections (planned)

Usage: imp mcp [COMMAND]

Commands:
  list    List configured MCP servers
  add     Add an MCP server to configuration
  remove  Remove an MCP server from configuration
  doctor  Diagnose MCP configuration and connectivity
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## view

```text
Open the viewer/inspector surface (planned; not fully implemented yet)

Usage: imp view [AREA]

Arguments:
  [AREA]  Viewer area to open (planned: sessions, tree, logs, checkpoints)

Options:
  -h, --help  Print help
```

