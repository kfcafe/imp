# imp_core — Technical Specification

The agent engine. Takes a task or interactive prompt, connects to an LLM, executes tools, manages context, produces results. This document is the canonical reference for building imp_core in Rust.

## Crate Structure

```
imp/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── imp-llm/            # Standalone LLM client library
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── provider.rs       # Provider trait + types
│   │   │   ├── stream.rs         # Streaming event types
│   │   │   ├── message.rs        # Unified message types
│   │   │   ├── model.rs          # Model registry + metadata
│   │   │   ├── auth.rs           # API key + OAuth resolution
│   │   │   ├── usage.rs          # Token counting + cost tracking
│   │   │   ├── providers/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── anthropic.rs  # Anthropic Messages API
│   │   │   │   ├── openai.rs     # OpenAI Responses API
│   │   │   │   └── google.rs     # Google Gemini API
│   │   │   └── oauth/
│   │   │       ├── mod.rs
│   │   │       └── anthropic.rs  # Anthropic Max/Pro OAuth
│   │   └── Cargo.toml
│   │
│   ├── imp-core/           # Agent engine
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── agent.rs          # Agent loop (ReAct cycle)
│   │   │   ├── context.rs        # Context assembly + management
│   │   │   ├── session.rs        # Session persistence (JSONL tree)
│   │   │   ├── config.rs         # TOML configuration
│   │   │   ├── system_prompt.rs  # System prompt assembly
│   │   │   ├── hooks.rs          # In-process hook system
│   │   │   ├── roles.rs          # Agent role definitions
│   │   │   ├── resources.rs      # AGENTS.md, skills, prompts discovery
│   │   │   ├── tools/
│   │   │   │   ├── mod.rs        # Tool trait + registry
│   │   │   │   ├── read.rs
│   │   │   │   ├── write.rs
│   │   │   │   ├── edit.rs
│   │   │   │   ├── multi_edit.rs
│   │   │   │   ├── bash.rs
│   │   │   │   ├── grep.rs
│   │   │   │   ├── find.rs
│   │   │   │   ├── ls.rs
│   │   │   │   ├── ask.rs
│   │   │   │   ├── diff.rs
│   │   │   │   ├── tree_sitter.rs  # probe_search, probe_extract, scan, ast_grep
│   │   │   │   ├── shell.rs       # TOML-defined shell tool loader
│   │   │   │   └── lua.rs         # Lua-defined tool bridge
│   │   │   └── compaction.rs      # Observation masking + LLM compaction
│   │   └── Cargo.toml
│   │
│   ├── imp-lua/            # Lua extension runtime
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── bridge.rs        # Host API exposed to Lua
│   │   │   ├── loader.rs        # Extension discovery + hot reload
│   │   │   └── sandbox.rs       # Lua state management
│   │   └── Cargo.toml
│   │
│   ├── imp-tui/            # Terminal UI
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── app.rs           # App state + event loop
│   │   │   ├── views/
│   │   │   │   ├── chat.rs      # Message display + streaming
│   │   │   │   ├── editor.rs    # Input editor
│   │   │   │   ├── tree.rs      # Session tree navigator
│   │   │   │   ├── tools.rs     # Tool call rendering
│   │   │   │   └── status.rs    # Footer status bar
│   │   │   ├── theme.rs         # Theming
│   │   │   └── keybindings.rs   # Keyboard shortcuts
│   │   └── Cargo.toml
│   │
│   └── imp-cli/            # Binary entry point
│       ├── src/
│       │   └── main.rs          # CLI arg parsing, mode dispatch
│       └── Cargo.toml
│
├── lua/                    # Built-in Lua extensions (shipped with imp)
├── tools/                  # Built-in shell tool definitions (TOML)
└── skills/                 # Built-in skills (SKILL.md files)
```

**Dependency direction:**
```
imp-llm  ←  imp-core  ←  imp-lua  ←  imp-tui  ←  imp-cli
(nothing)   (imp-llm)    (imp-core)   (imp-core    (everything)
                                       imp-lua)
```

**Key external dependencies:**
- `reqwest` — HTTP client (with streaming)
- `tokio` — async runtime
- `serde` / `serde_json` — serialization
- `toml` — config parsing
- `mlua` — Lua 5.4 embedding (with `async` and `send` features)
- `tree-sitter` + language grammars — native code intelligence
- `ratatui` + `crossterm` — terminal UI
- `clap` — CLI argument parsing
- `jsonschema` — tool parameter validation

---

## 1. imp-llm — LLM Client Library

Standalone multi-provider streaming client. No dependency on the rest of imp. Could be published as an independent crate.

### Provider Trait

```rust
/// A provider handles communication with a specific LLM API.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Stream a completion response.
    fn stream(
        &self,
        model: &Model,
        context: Context,
        options: RequestOptions,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>;

    /// Resolve an API key for this provider.
    async fn resolve_auth(&self, auth: &AuthStore) -> Result<ApiKey>;

    /// Provider identifier (e.g., "anthropic", "openai", "google").
    fn id(&self) -> &str;

    /// List available models for this provider.
    fn models(&self) -> &[ModelMeta];
}
```

### Unified Message Types

All providers serialize to/from these types. Provider modules handle the translation.

```rust
/// A message in the conversation.
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    ToolResult(ToolResultMessage),
}

pub struct UserMessage {
    pub content: Vec<ContentBlock>,
    pub timestamp: u64,
}

pub struct AssistantMessage {
    pub content: Vec<ContentBlock>,
    pub usage: Option<Usage>,
    pub stop_reason: StopReason,
    pub timestamp: u64,
}

pub struct ToolResultMessage {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: Vec<ContentBlock>,
    pub is_error: bool,
    pub details: serde_json::Value,
    pub timestamp: u64,
}

pub enum ContentBlock {
    Text { text: String },
    Thinking { text: String },
    ToolCall { id: String, name: String, arguments: serde_json::Value },
    Image { media_type: String, data: String },  // base64
}

pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    Error(String),
}
```

### Streaming Events

Normalized stream that all providers produce:

```rust
pub enum StreamEvent {
    /// New message starting.
    MessageStart { model: String },

    /// Incremental text from the assistant.
    TextDelta { text: String },

    /// Incremental thinking/reasoning output.
    ThinkingDelta { text: String },

    /// A complete tool call (accumulated from deltas).
    ToolCall { id: String, name: String, arguments: serde_json::Value },

    /// Message complete with usage stats.
    MessageEnd { message: AssistantMessage },

    /// Unrecoverable stream error.
    Error { error: ProviderError },
}
```

### Request Options

```rust
pub struct RequestOptions {
    pub thinking_level: ThinkingLevel,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub system_prompt: String,
    pub tools: Vec<ToolDefinition>,
    pub cache_options: CacheOptions,
}

pub enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
}

pub struct CacheOptions {
    /// Anthropic: which content blocks get cache_control breakpoints.
    pub cache_system_prompt: bool,
    pub cache_tools: bool,
    /// Number of recent conversation turns to mark cacheable.
    pub cache_recent_turns: usize,
}
```

### Token Usage & Cost

```rust
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
}

pub struct Cost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}

impl Usage {
    pub fn cost(&self, pricing: &ModelPricing) -> Cost { ... }
}
```

### Model Registry

```rust
pub struct ModelMeta {
    pub id: String,
    pub provider: String,
    pub name: String,              // human-readable display name
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub pricing: ModelPricing,
    pub capabilities: Capabilities,
}

pub struct ModelPricing {
    pub input_per_mtok: f64,       // cost per million input tokens
    pub output_per_mtok: f64,
    pub cache_read_per_mtok: f64,
    pub cache_write_per_mtok: f64,
}

pub struct Capabilities {
    pub reasoning: bool,           // supports thinking/reasoning
    pub images: bool,              // supports image input
    pub tool_use: bool,
}

/// Resolved model ready for use (metadata + provider reference).
pub struct Model {
    pub meta: ModelMeta,
    pub provider: Arc<dyn Provider>,
}
```

Built-in models are hardcoded per provider (updated with releases). Users can add models via `~/.config/imp/models.toml`.

### Auth

```rust
pub struct AuthStore {
    /// Runtime overrides (not persisted).
    runtime_keys: HashMap<String, String>,
    /// Persisted credentials (API keys + OAuth tokens).
    stored: HashMap<String, StoredCredential>,
    /// Path to auth storage file.
    path: PathBuf,
}

pub enum StoredCredential {
    ApiKey(String),
    OAuth(OAuthCredential),
}

pub struct OAuthCredential {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

impl AuthStore {
    /// Resolution order: runtime override → stored → env var → error.
    pub fn resolve(&self, provider: &str) -> Result<String> { ... }
}
```

**API key resolution order:**
1. Runtime override (set programmatically, not persisted)
2. Stored credential in `~/.config/imp/auth.json`
3. Environment variable (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GOOGLE_API_KEY`)
4. Error

### Anthropic OAuth

Anthropic Max/Pro subscription OAuth. This is a v1 requirement. Implementation mirrors pi's approach.

```rust
pub struct AnthropicOAuth {
    pub client_id: String,
    pub redirect_uri: String,
}

impl AnthropicOAuth {
    /// Start the OAuth flow. Opens browser, starts local HTTP server for callback.
    pub async fn login(&self) -> Result<OAuthCredential> { ... }

    /// Refresh an expired token.
    pub async fn refresh(&self, credential: &OAuthCredential) -> Result<OAuthCredential> { ... }
}
```

Flow:
1. Generate PKCE code verifier + challenge
2. Open browser to Anthropic's authorization URL
3. Start local HTTP server on a random port to receive the callback
4. Exchange authorization code for access + refresh tokens
5. Store in AuthStore

When a request gets a 401, automatically attempt token refresh once before failing.

### Provider Implementations

Each provider module translates between imp's unified types and the provider's wire format.

**Anthropic** (`providers/anthropic.rs`):
- Messages API with streaming (SSE)
- `cache_control` breakpoints on system prompt and recent turns
- Extended thinking via `thinking` parameter with budget
- Image content in user messages and tool results (native support)

**OpenAI** (`providers/openai.rs`):
- Responses API with streaming (SSE)
- Session-based caching (automatic, no explicit control)
- Reasoning effort mapping from ThinkingLevel
- Images in tool results require workaround: extract to user message

**Google** (`providers/google.rs`):
- Gemini API with streaming (SSE)
- Thinking config with thinking budget
- Chunked tool call accumulation (Google streams tool call arguments incrementally)
- Image support in user messages

### Retry & Error Handling

```rust
pub struct RetryPolicy {
    pub max_retries: u32,          // default: 3
    pub base_delay: Duration,      // default: 1s
    pub max_delay: Duration,       // default: 30s
    pub retry_on: Vec<RetryCondition>,
}

pub enum RetryCondition {
    RateLimit,        // 429
    ServerError,      // 5xx
    Timeout,
    ConnectionError,
}
```

Exponential backoff with jitter. If the server sends a `Retry-After` header, respect it (but cap at `max_delay` — if the server asks for 5 minutes, fail immediately).

---

## 2. Agent Loop

The core reasoning cycle. Defined in `imp-core/src/agent.rs`.

### State Machine

```
                    ┌──────────────┐
                    │   Idle       │
                    └──────┬───────┘
                           │ prompt()
                           ▼
                    ┌──────────────┐
          ┌────────│  Processing  │◄────────┐
          │        └──────┬───────┘         │
          │               │                 │
          │    ┌──────────▼──────────┐      │
          │    │  LLM Streaming      │      │
          │    └──────────┬──────────┘      │
          │               │                 │
          │    ┌──────────▼──────────┐      │
          │    │  Stop Reason?       │      │
          │    └──┬─────────────┬────┘      │
          │       │ EndTurn     │ ToolUse   │
          │       │ MaxTokens   │           │
          │       ▼             ▼            │
          │  ┌─────────┐  ┌──────────┐      │
          │  │  Done    │  │ Execute  │      │
          │  └─────────┘  │ Tools    ├──────┘
          │               └──────────┘
          │
          │  cancel / steer (via channel)
          ▼
    ┌──────────────┐
    │  Aborted     │
    └──────────────┘
```

### Agent Struct

```rust
pub struct Agent {
    pub model: Model,
    pub thinking_level: ThinkingLevel,
    pub tools: ToolRegistry,
    pub messages: Vec<Message>,
    pub system_prompt: String,
    pub cwd: PathBuf,
    pub max_turns: u32,            // default: 50
    pub role: Option<Role>,

    // Communication channels
    event_tx: mpsc::Sender<AgentEvent>,
    command_rx: mpsc::Receiver<AgentCommand>,
}

pub enum AgentCommand {
    Cancel,
    Steer(String),
    FollowUp(String),
}

pub enum AgentEvent {
    AgentStart { model: String, timestamp: u64 },
    AgentEnd { messages: Vec<Message>, usage: Usage, cost: Cost },
    TurnStart { index: u32 },
    TurnEnd { index: u32, message: AssistantMessage, tool_results: Vec<ToolResultMessage> },
    MessageStart { message: Message },
    MessageDelta { delta: StreamEvent },
    MessageEnd { message: Message },
    ToolExecutionStart { tool_call_id: String, tool_name: String, args: serde_json::Value },
    ToolExecutionUpdate { tool_call_id: String, partial: String },
    ToolExecutionEnd { tool_call_id: String, result: ToolResultMessage },
    CompactionStart,
    CompactionEnd { summary: String },
    Error { error: String },
}
```

### Run Loop

```rust
impl Agent {
    pub async fn run(&mut self, prompt: String) -> Result<()> {
        self.emit(AgentEvent::AgentStart { ... });

        // Add user message
        self.messages.push(Message::User(UserMessage {
            content: vec![ContentBlock::Text { text: prompt }],
            timestamp: now(),
        }));

        let mut turn = 0;

        loop {
            if turn >= self.max_turns {
                self.emit(AgentEvent::Error { error: "Max turns exceeded".into() });
                break;
            }

            // Check for cancel/steer between turns
            if let Some(cmd) = self.check_commands() {
                match cmd {
                    AgentCommand::Cancel => break,
                    AgentCommand::Steer(msg) => {
                        self.messages.push(Message::User(UserMessage {
                            content: vec![ContentBlock::Text { text: msg }],
                            timestamp: now(),
                        }));
                    }
                    AgentCommand::FollowUp(_) => { /* queue for after loop */ }
                }
            }

            self.emit(AgentEvent::TurnStart { index: turn });

            // Check context budget, compact if needed
            self.check_context_budget().await?;

            // Fire pre-turn hooks (context injection, etc.)
            self.fire_hooks(HookEvent::BeforeLlmCall).await;

            // Assemble context and stream LLM response
            let context = self.build_context();
            let assistant_msg = self.stream_completion(context).await?;

            // Extract tool calls
            let tool_calls: Vec<_> = assistant_msg.content.iter()
                .filter_map(|b| match b {
                    ContentBlock::ToolCall { id, name, arguments } =>
                        Some((id.clone(), name.clone(), arguments.clone())),
                    _ => None,
                })
                .collect();

            self.messages.push(Message::Assistant(assistant_msg.clone()));

            if tool_calls.is_empty() {
                // No tool calls — model is done
                self.emit(AgentEvent::TurnEnd { index: turn, message: assistant_msg, tool_results: vec![] });
                break;
            }

            // Execute tool calls
            let tool_results = self.execute_tools(tool_calls).await;

            // Fire post-tool hooks (lint after write, etc.)
            for result in &tool_results {
                self.fire_hooks(HookEvent::AfterToolCall {
                    tool_name: &result.tool_name,
                    result,
                }).await;
            }

            // Add tool results to messages
            for result in &tool_results {
                self.messages.push(Message::ToolResult(result.clone()));
            }

            self.emit(AgentEvent::TurnEnd {
                index: turn,
                message: assistant_msg,
                tool_results: tool_results.clone(),
            });

            turn += 1;
        }

        self.emit(AgentEvent::AgentEnd { ... });
        Ok(())
    }
}
```

### Tool Execution

Tool calls from a single assistant message execute with controlled concurrency:
- **Read-only tools** (`read`, `grep`, `find`, `ls`, `probe_search`, `probe_extract`, `scan`) — execute concurrently
- **Write tools** (`write`, `edit`, `multi_edit`, `bash`) — execute sequentially, in order
- Mixed batches: read-only tools run concurrently first, then writes sequentially

Before execution, each tool call is:
1. Validated against the tool's JSON schema
2. Passed through `before_tool_call` hooks (can block)
3. Executed with timeout and cancellation support

```rust
async fn execute_tools(&self, calls: Vec<(String, String, Value)>) -> Vec<ToolResultMessage> {
    let (readonly, mutable): (Vec<_>, Vec<_>) = calls.into_iter()
        .partition(|(_, name, _)| self.tools.get(name).map_or(false, |t| t.is_readonly()));

    let mut results = Vec::new();

    // Read-only tools concurrently
    let mut futures = FuturesOrdered::new();
    for (id, name, args) in readonly {
        futures.push_back(self.execute_one_tool(id, name, args));
    }
    while let Some(result) = futures.next().await {
        results.push(result);
    }

    // Mutable tools sequentially
    for (id, name, args) in mutable {
        // Check for cancel between writes
        if self.is_cancelled() { break; }
        results.push(self.execute_one_tool(id, name, args).await);
    }

    results
}
```

---

## 3. Tool System

### Tool Trait

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (used in LLM tool calls).
    fn name(&self) -> &str;

    /// Human-readable label.
    fn label(&self) -> &str;

    /// Description shown to the LLM.
    fn description(&self) -> &str;

    /// JSON Schema for parameters.
    fn parameters(&self) -> &serde_json::Value;

    /// Whether this tool only reads (no side effects).
    fn is_readonly(&self) -> bool;

    /// Execute the tool.
    async fn execute(
        &self,
        call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput>;
}

pub struct ToolContext {
    pub cwd: PathBuf,
    pub signal: CancellationToken,
    pub update_tx: mpsc::Sender<ToolUpdate>,  // for streaming partial results
}

pub struct ToolOutput {
    pub content: Vec<ContentBlock>,
    pub details: serde_json::Value,
    pub is_error: bool,
}

pub struct ToolUpdate {
    pub content: Vec<ContentBlock>,
    pub details: serde_json::Value,
}
```

### Tool Registry

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Register a native Rust tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) { ... }

    /// Load shell tools from TOML definitions.
    pub fn load_shell_tools(&mut self, dir: &Path) -> Result<()> { ... }

    /// Load Lua-defined tools from the Lua runtime.
    pub fn load_lua_tools(&mut self, lua: &LuaRuntime) -> Result<()> { ... }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> { ... }

    /// Get all tool definitions (for LLM context).
    pub fn definitions(&self) -> Vec<ToolDefinition> { ... }
}
```

### Native Rust Tools

All built-in tools. Always loaded. Each is a struct implementing `Tool`.

| Tool | File | Readonly | Description |
|------|------|----------|-------------|
| `read` | `tools/read.rs` | yes | Read file contents (text + images). Offset/limit pagination. |
| `write` | `tools/write.rs` | no | Write file, create parent directories. |
| `edit` | `tools/edit.rs` | no | Find-and-replace with fuzzy matching, diff output. |
| `multi_edit` | `tools/multi_edit.rs` | no | Multiple find-and-replace ops on one file, atomic. |
| `bash` | `tools/bash.rs` | no | Shell command execution with streaming, timeout, process management. |
| `grep` | `tools/grep.rs` | yes | Search file contents via ripgrep. |
| `find` | `tools/find.rs` | yes | Find files by glob via fd (or native walkdir fallback). |
| `ls` | `tools/ls.rs` | yes | List directory contents. |
| `ask` | `tools/ask.rs` | yes | Ask user a question (multiple choice, text input, confirm). Native TUI widget. |
| `diff_show` | `tools/diff.rs` | yes | Generate unified diff between file and proposed content. |
| `diff_apply` | `tools/diff.rs` | no | Apply a unified diff patch to a file. |
| `probe_search` | `tools/tree_sitter.rs` | yes | Semantic code search (ripgrep + tree-sitter AST). |
| `probe_extract` | `tools/tree_sitter.rs` | yes | Extract complete code blocks by line or symbol. |
| `scan` | `tools/tree_sitter.rs` | yes | Extract code structure (types, functions, imports). |
| `ast_grep` | `tools/tree_sitter.rs` | mixed | Structural code search. Readonly for search, write for replace. |

#### Tree-sitter Integration

Native embedding via the `tree-sitter` crate. Language grammars compiled into the binary:

**v1 languages:** Rust, TypeScript/TSX, JavaScript/JSX, Python, Go, C, C++, Java, Ruby, Elixir, Lua, Bash, TOML, YAML, JSON, Markdown, HTML, CSS

```rust
pub struct TreeSitterEngine {
    parsers: HashMap<&'static str, tree_sitter::Parser>,
}

impl TreeSitterEngine {
    /// Parse a file and return its AST.
    pub fn parse(&self, source: &str, language: &str) -> Result<tree_sitter::Tree> { ... }

    /// Extract the enclosing function/class/struct at a line number.
    pub fn extract_block(&self, tree: &Tree, source: &str, line: usize) -> Option<CodeBlock> { ... }

    /// Extract all top-level symbols (functions, types, imports).
    pub fn extract_structure(&self, tree: &Tree, source: &str) -> FileStructure { ... }

    /// Search for AST pattern matches (ast-grep style).
    pub fn pattern_search(&self, tree: &Tree, source: &str, pattern: &str) -> Vec<Match> { ... }

    /// Detect language from file extension.
    pub fn detect_language(path: &Path) -> Option<&'static str> { ... }
}
```

`probe_search` combines ripgrep (for fast text matching) with tree-sitter (to expand matches to complete code blocks). The ripgrep step uses the `grep` crate for in-process execution — no subprocess needed.

#### Output Truncation

Every tool truncates output to protect the context window:
- **50KB** or **2000 lines**, whichever is hit first
- Long individual lines truncated to **500 chars**
- When truncated, full output written to a temp file; path included in result

```rust
pub struct TruncationResult {
    pub content: String,
    pub truncated: bool,
    pub output_lines: usize,
    pub total_lines: usize,
    pub output_bytes: usize,
    pub total_bytes: usize,
    pub temp_file: Option<PathBuf>,
}

pub fn truncate_head(input: &str, max_lines: usize, max_bytes: usize) -> TruncationResult { ... }
pub fn truncate_tail(input: &str, max_lines: usize, max_bytes: usize) -> TruncationResult { ... }
```

### Shell Tools (TOML-defined)

Zero-code CLI wrappers. Discovered from:
- `~/.config/imp/tools/*.toml` (user)
- `.imp/tools/*.toml` (project)
- Built-in `tools/` directory (shipped with imp)

```toml
# tools/youtube-transcript.toml
name = "youtube_transcript"
label = "YouTube Transcript"
description = "Extract transcript from a YouTube video via yt-dlp"
readonly = true

[params]
url = { type = "string", description = "YouTube URL or video ID" }
language = { type = "string", description = "Language code (default: en)", optional = true }

[exec]
command = "yt-dlp"
args = ["--write-subs", "--skip-download", "--sub-format", "json3", "--sub-lang", "{language|en}", "{url}"]
timeout = 30
truncate = "head"
```

The runtime:
1. Validates params against the schema
2. Interpolates `{param}` and `{param|default}` into args
3. Runs the command with `tokio::process::Command`
4. Captures stdout, applies truncation
5. Returns as tool result

If the command binary isn't found, returns a helpful error with install instructions (if specified in the TOML: `install_hint = "brew install yt-dlp"`).

### Lua Tools

Registered via the Lua extension API (see §7). Lua tools implement the same interface but execution is bridged through mlua:

```lua
imp.register_tool({
    name = "my_tool",
    label = "My Tool",
    description = "Does something useful",
    readonly = false,
    params = {
        input = { type = "string", description = "Input value" },
    },
    execute = function(call_id, params, ctx)
        local result = imp.exec("some-command", { params.input })
        return {
            content = { { type = "text", text = result.stdout } },
            details = {},
        }
    end,
})
```

---

## 4. Session Management

### Session Format

JSONL (JSON Lines) with a tree structure. Each entry has an `id` and `parent_id`, enabling in-place branching.

```jsonl
{"type":"header","version":1,"created_at":1710000000,"cwd":"/path/to/project"}
{"type":"message","id":"a1","parent_id":null,"message":{"role":"user","content":[{"type":"text","text":"Fix the auth bug"}],"timestamp":1710000001}}
{"type":"message","id":"a2","parent_id":"a1","message":{"role":"assistant","content":[{"type":"text","text":"I'll look into that..."},{"type":"tool_call","id":"tc1","name":"read","arguments":{"path":"src/auth.rs"}}],"timestamp":1710000002}}
{"type":"message","id":"a3","parent_id":"a2","message":{"role":"tool_result","tool_call_id":"tc1","tool_name":"read","content":[{"type":"text","text":"...file contents..."}],"timestamp":1710000003}}
{"type":"compaction","id":"c1","parent_id":"a3","summary":"User asked to fix auth bug. Read src/auth.rs.","first_kept_id":"a3","tokens_before":15000,"tokens_after":2000}
{"type":"custom","id":"x1","parent_id":"a3","custom_type":"lua-extension","data":{"key":"value"}}
```

### Session Manager

```rust
pub struct SessionManager {
    entries: Vec<SessionEntry>,
    path: Option<PathBuf>,
}

pub enum SessionEntry {
    Header { version: u32, created_at: u64, cwd: String },
    Message { id: String, parent_id: Option<String>, message: Message },
    Compaction { id: String, parent_id: Option<String>, summary: String, first_kept_id: String },
    Custom { id: String, parent_id: Option<String>, custom_type: String, data: serde_json::Value },
    Label { entry_id: String, label: String },
}

impl SessionManager {
    /// Create a new session.
    pub fn new(cwd: &Path, session_dir: &Path) -> Self { ... }

    /// Open an existing session file.
    pub fn open(path: &Path) -> Result<Self> { ... }

    /// Continue the most recent session for a directory.
    pub fn continue_recent(cwd: &Path, session_dir: &Path) -> Result<Self> { ... }

    /// In-memory session (no persistence).
    pub fn in_memory() -> Self { ... }

    /// Append an entry (writes to file immediately).
    pub fn append(&mut self, entry: SessionEntry) -> Result<()> { ... }

    /// Get the linear path from root to current leaf.
    pub fn get_branch(&self) -> Vec<&SessionEntry> { ... }

    /// Get messages for the current branch (for LLM context).
    pub fn get_messages(&self) -> Vec<&Message> { ... }

    /// Get the full tree structure.
    pub fn get_tree(&self) -> Tree { ... }

    /// Navigate to a different branch point.
    pub fn navigate(&mut self, target_id: &str) { ... }

    /// Fork from a specific entry (creates new session file).
    pub fn fork(&self, entry_id: &str, new_path: &Path) -> Result<SessionManager> { ... }

    /// List available sessions for a directory.
    pub fn list(session_dir: &Path) -> Result<Vec<SessionInfo>> { ... }
}
```

**Session storage:**
- Sessions stored in `~/.local/share/imp/sessions/`
- Organized by working directory (path encoded as directory name)
- Files named by session ID (UUID)

---

## 5. Context Management

Two-stage approach to prevent context overflow.

### Stage 1: Observation Masking (~60% context usage)

Replace old tool results with lightweight placeholders. Tool observations are 80%+ of context volume but become noise after a few turns.

```rust
pub fn mask_observations(messages: &mut Vec<Message>, window: usize) {
    // Keep the last `window` turns fully intact.
    // For older turns, replace tool result content with:
    // "[Output omitted — ran {tool}({summary_of_args}), returned {size}]"
}
```

Triggered by a hook at configurable threshold (default: 60% of context window).

**What's preserved:** All reasoning (assistant text), all actions (tool calls with arguments), the user's messages. Only raw tool output is masked.

### Stage 2: LLM Compaction (~80% context usage)

Summarize the conversation via the LLM. Triggered at 80% — not when full, because by 100% the model is already degraded.

```rust
pub struct CompactionResult {
    pub summary: String,
    pub first_kept_id: String,     // entries after this are kept verbatim
    pub tokens_before: u32,
    pub tokens_after: u32,
}

pub async fn compact(
    messages: &[Message],
    model: &Model,
    provider: &dyn Provider,
    instructions: Option<&str>,
) -> Result<CompactionResult> { ... }
```

The compaction prompt preserves:
- The user's original request (verbatim)
- Files touched, changes made
- Work remaining and blockers
- Things explicitly forbidden or that failed (negative constraints)

### Token Estimation

Fast approximate token counting for context budget checks:

```rust
pub fn estimate_tokens(text: &str) -> u32 {
    // ~4 chars per token for English (rough but fast).
    // Exact counting is expensive and provider-specific.
    (text.len() as u32) / 4
}

pub fn context_usage(messages: &[Message], model: &Model) -> ContextUsage {
    let used = messages.iter().map(|m| estimate_tokens(&m.to_string())).sum();
    let limit = model.meta.context_window;
    ContextUsage { used, limit, ratio: used as f64 / limit as f64 }
}
```

---

## 6. Hook System

Two types of hooks, both configured in TOML and extensible via Lua.

### TOML Hooks (simple, declarative)

```toml
# .imp/config.toml

[[hooks]]
event = "after_file_write"
match = "*.rs"
action = "shell"
command = "rustfmt {file}"
blocking = true

[[hooks]]
event = "after_file_write"
match = "*.rs"
action = "shell"
command = "cargo clippy --message-format=short 2>&1 | head -20"
blocking = false

[[hooks]]
event = "on_context_threshold"
threshold = 0.6
action = "observation_mask"
blocking = true

[[hooks]]
event = "on_context_threshold"
threshold = 0.8
action = "compact"
blocking = true
```

### Hook Events

```rust
pub enum HookEvent<'a> {
    /// After write/edit tool modifies a file.
    AfterFileWrite { file: &'a Path },

    /// Before a tool executes (can block).
    BeforeToolCall { tool_name: &'a str, args: &'a Value },

    /// After a tool executes (can modify result).
    AfterToolCall { tool_name: &'a str, result: &'a ToolResultMessage },

    /// Before the LLM is called (can inject context).
    BeforeLlmCall,

    /// Context usage hit a threshold.
    OnContextThreshold { ratio: f64 },

    /// Session started.
    OnSessionStart,

    /// Session shutting down.
    OnSessionShutdown,

    /// Agent started processing a prompt.
    OnAgentStart { prompt: &'a str },

    /// Agent finished processing.
    OnAgentEnd { messages: &'a [Message] },

    /// Turn completed.
    OnTurnEnd { index: u32, message: &'a AssistantMessage },
}

pub enum HookAction {
    /// Run a shell command. `{file}` is interpolated.
    Shell { command: String },

    /// Built-in action.
    BuiltIn(BuiltInAction),
}

pub enum BuiltInAction {
    ObservationMask,
    Compact,
}

pub struct HookResult {
    /// If true, block the triggering action (for BeforeToolCall).
    pub block: bool,
    /// Optional reason for blocking.
    pub reason: Option<String>,
    /// Modified content (for AfterToolCall result modification).
    pub modified_content: Option<Vec<ContentBlock>>,
}
```

### Lua Hooks

Lua extensions can register hooks for any event with full programmatic control:

```lua
imp.on("after_file_write", function(event, ctx)
    if event.file:match("%.rs$") then
        local result = imp.exec("cargo", { "clippy", "--message-format=short" })
        if result.exit_code ~= 0 then
            ctx.ui.notify("Clippy warnings after write", "warning")
        end
    end
end)

imp.on("before_tool_call", function(event, ctx)
    if event.tool_name == "bash" and event.args.command:match("rm %-rf") then
        local ok = ctx.ui.confirm("Dangerous!", "Allow rm -rf?")
        if not ok then
            return { block = true, reason = "Blocked by user" }
        end
    end
end)
```

### Hook Execution Order

1. TOML hooks run first, in config file order
2. Lua hooks run second, in extension load order
3. Blocking hooks execute before async hooks for the same event

---

## 7. Lua Extension Runtime (imp-lua)

### Discovery

Lua extensions are discovered from:
- `~/.config/imp/lua/*.lua` (user global)
- `~/.config/imp/lua/*/init.lua` (user global, directory)
- `.imp/lua/*.lua` (project local)
- `.imp/lua/*/init.lua` (project local, directory)

### Host API

Functions exposed to Lua via mlua:

```lua
-- Event hooks
imp.on(event_name, handler)              -- Subscribe to hook events

-- Tool registration
imp.register_tool(definition)            -- Register a custom tool
imp.get_active_tools()                   -- List active tool names
imp.set_active_tools(names)              -- Enable/disable tools

-- Command registration
imp.register_command(name, definition)   -- Register a /command

-- Shortcut registration
imp.register_shortcut(key, definition)   -- Register a keyboard shortcut

-- Shell execution
imp.exec(command, args, opts)            -- Run a shell command
                                          -- Returns { stdout, stderr, exit_code }

-- Session state
imp.session.get_entries()                -- All session entries
imp.session.get_branch()                 -- Current branch
imp.session.append_entry(type, data)     -- Persist extension state

-- Message injection
imp.send_message(message, opts)          -- Inject a message
imp.send_user_message(content, opts)     -- Send as user

-- Model control
imp.get_model()                          -- Current model info
imp.set_model(provider, model_id)        -- Switch model
imp.get_thinking_level()                 -- Current thinking level
imp.set_thinking_level(level)            -- Set thinking level

-- Inter-extension events
imp.events.on(name, handler)             -- Listen for custom events
imp.events.emit(name, data)              -- Emit custom events
```

```lua
-- UI methods (available via ctx in hook/tool/command handlers)
-- These are convenience wrappers around the declarative component model.
ctx.ui.notify(message, level)            -- "info" | "warning" | "error"
ctx.ui.confirm(title, message)           -- Returns boolean
ctx.ui.select(title, options)            -- Returns selected option or nil
ctx.ui.input(title, placeholder)         -- Returns string or nil
ctx.ui.set_status(key, text)             -- Footer status indicator
ctx.ui.set_widget(key, lines_or_fn)      -- Widget above/below editor
ctx.ui.set_working_message(text)         -- During streaming
ctx.ui.set_footer(render_fn)             -- Custom footer

-- Full declarative custom component API.
-- Temporarily replaces the editor with a Lua-described component.
-- Rust owns all rendering. Lua describes what to render and handles events.
-- Returns the value passed to done().
ctx.ui.custom(function(done)
    -- Return a declarative component table.
    -- Rust renders it. Key events call on_key, which returns new state.
    -- Call done(value) to close and return.
    local state = { selected = 1 }
    return {
        render = function(s)
            return {
                type = "box", border = true, title = "Pick one",
                children = {
                    { type = "list", items = items,
                      selected = s.selected,
                      style = { selected_fg = "accent" } },
                },
            }
        end,
        on_key = function(key, s)
            if key == "up" then s.selected = math.max(1, s.selected - 1)
            elseif key == "down" then s.selected = math.min(#items, s.selected + 1)
            elseif key == "return" then done(items[s.selected])
            elseif key == "escape" then done(nil)
            end
            return s  -- return new state, Rust re-renders
        end,
        state = state,
    }
end)
```

#### Declarative Component Types

Rust-side renderers for declarative tables. New component types can be added
over time without changing the Lua API.

| Type | Props | Description |
|------|-------|-------------|
| `text` | `content`, `style` | Styled text span |
| `list` | `items`, `selected`, `style` | Selectable list |
| `box` | `border`, `title`, `children`, `direction` | Container with optional border |
| `input` | `value`, `placeholder`, `label` | Text input field |
| `progress` | `percent`, `label` | Progress bar |
| `spacer` | `height` | Vertical spacer |
| `markdown` | `content` | Rendered markdown |

The simple methods (`confirm`, `select`, `input`) are sugar that build these
tables internally — Lua extensions that only need simple dialogs never touch
the declarative API directly.

### Async Bridging

Lua operations that involve I/O (shell execution, UI prompts, HTTP requests) are bridged through mlua's async support. From the Lua side, these appear synchronous (using coroutines internally):

```lua
-- This looks synchronous in Lua but is async on the Rust side
local result = imp.exec("cargo", { "test" })
local ok = ctx.ui.confirm("Continue?", "Tests passed")
```

mlua's `async` feature handles the coroutine ↔ Future bridging.

### Hot Reload

`/reload` command triggers:
1. Fire `on_session_shutdown` for current Lua state
2. Drop the Lua state (all Lua memory freed)
3. Re-discover and load extensions
4. Fire `on_session_start` for new state
5. Re-register all Lua tools and hooks

Extension state that needs to survive reloads should be persisted via `imp.session.append_entry()`.

---

## 8. Resource Discovery

### AGENTS.md

Auto-discovered and concatenated at startup:
- `~/.config/imp/AGENTS.md` (user global)
- Walk up from cwd: each `AGENTS.md` or `CLAUDE.md` found
- Current directory `AGENTS.md` or `CLAUDE.md`

All matching files concatenated into the system prompt.

### Skills

On-demand instruction packages. Discovered from:
- `~/.config/imp/skills/*/SKILL.md` (user global)
- `.imp/skills/*/SKILL.md` (project local)
- Walk up from cwd: `.agents/skills/*/SKILL.md`

Each skill has a `name`, `description`, and content. Skills are listed in the system prompt with their descriptions. The LLM can request a skill's content when the task matches.

### Prompt Templates

Reusable prompts as markdown files. Expanded via `/name` syntax.
- `~/.config/imp/prompts/*.md` (user global)
- `.imp/prompts/*.md` (project local)

Templates support `{{variable}}` interpolation from command arguments.

---

## 9. System Prompt

Minimal. Structured in layers, assembled deterministically.

```
Layer 1: Identity (~200 tokens)
  You are imp, a coding agent. You have access to tools for reading,
  writing, and executing code.
  {tool descriptions — name + description for each active tool}
  {tool guidelines — usage rules}

Layer 2: Project context (from AGENTS.md)
  {concatenated AGENTS.md content}

Layer 3: Skills index
  Available skills:
  - rust: Conventions for Rust code. Use when working with .rs files.
  - testing: Test writing guide. Use when writing tests.
  (LLM uses read tool to load skill content when relevant)

Layer 4: Mana facts (from .mana/ if present)
  Project facts:
  - "This project uses JWT for auth" [verified 2h ago]
  - "Test suite requires Docker running" [verified 1d ago]

Layer 5: Task context (only in headless/task mode)
  Task: Fix the failing auth test
  Description: ...
  Verify: cargo test auth::jwt_test
  Previous attempts: ...
  Dependencies: ...
```

No behavioral coaching. No "consider decomposing" nudges. The tools exist, the facts are injected, the hooks fire. The model figures out the rest.

---

## 10. Agent Roles

Named configurations for different agent types.

```rust
pub struct Role {
    pub name: String,
    pub model: Option<String>,           // override model
    pub thinking_level: Option<ThinkingLevel>,
    pub tools: ToolSet,                  // which tools are available
    pub readonly: bool,                  // restrict to read-only tools
    pub instructions: Option<String>,    // additional system prompt
    pub max_turns: Option<u32>,
}

pub enum ToolSet {
    All,
    Only(Vec<String>),
}
```

**Built-in roles:**

| Role | Model | Thinking | Tools | Purpose |
|------|-------|----------|-------|---------|
| `worker` | (default) | medium | all | Standard task execution |
| `explorer` | fast/cheap | off | read-only | Codebase exploration, returns summary |
| `reviewer` | smart | high | read-only + code intelligence | Code review, verification |

Configured in TOML:
```toml
[roles.explorer]
model = "haiku"
thinking = "off"
tools = ["read", "grep", "find", "ls", "probe_search", "probe_extract"]
readonly = true
instructions = "Explore and summarize. Do not modify files."

[roles.reviewer]
model = "sonnet"
thinking = "high"
readonly = true
```

Roles are used by imp_orch when dispatching sub-agents, and can be set interactively via `/role` command.

---

## 11. Configuration

TOML-based. Resolution hierarchy (highest wins):

```
CLI flags (--model, --thinking, etc.)
  ↓
Environment variables (IMP_MODEL, ANTHROPIC_API_KEY, etc.)
  ↓
Project config (.imp/config.toml — checked into git)
  ↓
User config (~/.config/imp/config.toml — personal defaults)
  ↓
Built-in defaults
```

### Config Structure

```toml
# ~/.config/imp/config.toml

# Agent defaults
model = "sonnet"                        # default model (alias or full ID)
thinking = "medium"                     # off | minimal | low | medium | high | xhigh
max_turns = 50

# Active built-in tools (all enabled by default)
# tools = ["read", "write", "edit", "bash", "grep", "find", "ls"]

# Roles
[roles.explorer]
model = "haiku"
thinking = "off"
readonly = true

[roles.reviewer]
model = "sonnet"
thinking = "high"
readonly = true

# Hooks
[[hooks]]
event = "after_file_write"
match = "*.rs"
action = "shell"
command = "rustfmt {file}"
blocking = true

# Context management
[context]
observation_mask_threshold = 0.6        # mask old tool outputs at 60%
compaction_threshold = 0.8              # LLM compaction at 80%
mask_window = 10                        # keep last N turns unmasked
```

---

## 12. CLI Modes

### Interactive (default)

`imp` — ratatui TUI. Full-featured conversational agent.

- Message display with streaming responses and syntax highlighting
- Input editor with `@` file references and path completion
- Tool call rendering with expandable output
- Session tree navigation (`/tree`)
- Model selector (`/model`, Ctrl+L)
- Thinking level cycling (Shift+Tab)
- Status bar: model, tokens, cost, context usage
- Message queue: Enter for steering, Alt+Enter for follow-up

### Print Mode

`imp -p "prompt"` — send prompt, print response, exit. No TUI. For scripting and pipelines.

### RPC Mode

`imp --mode rpc` — JSON-lines over stdin/stdout. For process integration (imp_orch, SDKs, custom UIs).

**Protocol:** Same events/commands as the imp_core ↔ imp_orch protocol (§ in vision.md). LF-delimited JSON objects. Client sends commands on stdin, receives events on stdout.

### Headless Mode

`imp run <mana-unit-id>` — read brief from mana, work on it, run verify, exit. Designed to be spawned by imp_orch. Outputs JSON-lines events to stdout.

---

## 13. Mana Integration

imp_core links `mana-core` as a Rust dependency. Mana operations are native function calls, not CLI invocations.

```rust
use mana_core::{Store, Unit, Fact};

// Read the mana store
let store = Store::open(".mana/")?;

// In headless mode: load the assigned unit
let unit = store.get_unit(unit_id)?;
let context = store.assemble_context(unit_id)?;  // description + deps + history + facts

// During execution: update notes
store.add_note(unit_id, "Tried approach X, failed because Y")?;

// After completion: run verify
let verify_result = store.run_verify(unit_id)?;

// Load project facts for system prompt injection
let facts: Vec<Fact> = store.get_facts()?;
```

Facts from mana are injected into Layer 4 of the system prompt (see §9). All project facts are loaded. A subagent periodically verifies fact correctness (checking citations against current code).

---

## 14. Performance Targets

The motivation for Rust. These should be noticeably better than pi (Node.js).

| Metric | Target | pi baseline |
|--------|--------|-------------|
| Startup to first prompt | < 100ms | ~500-800ms |
| Tool execution overhead | < 5ms per tool | ~20-50ms |
| Memory (idle session) | < 30MB | ~100-150MB |
| Memory (active session) | < 80MB | ~200-400MB |
| Binary size | < 30MB | ~60MB (node_modules) |
| File read (10KB file) | < 1ms | ~5ms |
| Grep (medium project) | < 50ms | ~50ms (already ripgrep) |
| Tree-sitter parse | < 10ms | N/A (shells out to probe) |

The big wins: startup time (no V8 boot), memory (no GC overhead), and tree-sitter being in-process.

---

## 15. Testing Strategy

### Unit Tests
- Each tool: test execution with known inputs, verify output format and truncation
- Provider modules: test message serialization with fixture data (no live API calls)
- Session manager: test branching, navigation, persistence
- Context management: test observation masking, token estimation
- Hook system: test event matching, execution order, blocking behavior
- Lua bridge: test tool registration, event handling, API surface

### Integration Tests
- Agent loop: mock provider returns canned responses with tool calls, verify loop behavior
- Full tool chain: write a file, edit it, read it back, verify content
- Session round-trip: create session, add entries, save, reload, verify

### Snapshot Tests
- Provider serialization: snapshot the wire format for each provider
- System prompt assembly: snapshot the assembled prompt for known inputs

### Live Tests (optional, behind feature flag)
- Real provider calls with small prompts to verify streaming works end-to-end
- Run behind `--features live-tests`, not in CI by default

---

## 16. Build Order

Sequential, each step produces a testable artifact:

1. **imp-llm types** — Message, StreamEvent, Model, Usage types. No I/O yet. Just the data model.
2. **imp-llm Anthropic provider** — First working provider. SSE streaming via reqwest. Test with live API.
3. **imp-core Tool trait + native tools** — read, write, edit, bash, grep, find, ls. Test each independently.
4. **imp-core Agent loop** — ReAct cycle with mock provider. Test with canned responses.
5. **imp-core Session manager** — JSONL persistence, branching, navigation.
6. **imp-core Context management** — Observation masking, compaction.
7. **imp-llm OpenAI + Google providers** — Second and third providers.
8. **imp-llm Anthropic OAuth** — Subscription auth flow.
9. **imp-core Hook system** — TOML hooks, event dispatch.
10. **imp-core Tree-sitter tools** — probe_search, probe_extract, scan, ast_grep.
11. **imp-core Config + resource discovery** — TOML config, AGENTS.md, skills, prompts.
12. **imp-core System prompt assembly** — All layers, role-aware.
13. **imp-lua** — mlua bridge, extension loading, hot reload.
14. **imp-core Shell tools** — TOML-defined tool loader.
15. **imp-tui** — ratatui interactive mode.
16. **imp-cli** — Binary entry point, all modes.
17. **imp-core Mana integration** — Link mana-core, fact injection, headless mode.

Steps 1-6 produce a working agent that can have a conversation and use tools. Steps 7-12 make it production-ready. Steps 13-17 make it a pi replacement.

---

## 17. Crate Name

`imp` on crates.io is taken by a 6-year-old crate (v0.1.0, last updated 2019). Crates.io policy allows reclaiming abandoned crates — file a support request at help@crates.io. If that fails:
- `imp-agent` as the published crate name
- `imp` as the binary name (Cargo supports `[[bin]] name = "imp"` regardless of crate name)

---

## 18. User Interface Abstraction

Tools and Lua extensions interact with the user through a `UserInterface` trait.
The implementation varies by mode — tools don't know or care which mode they're in.

```rust
#[async_trait]
pub trait UserInterface: Send + Sync {
    /// Whether this interface can show interactive UI.
    fn has_ui(&self) -> bool;

    /// Non-blocking notification.
    async fn notify(&self, message: &str, level: NotifyLevel);

    /// Yes/no confirmation. Returns None if no UI or cancelled.
    async fn confirm(&self, title: &str, message: &str) -> Option<bool>;

    /// Select from options. Returns None if no UI or cancelled.
    async fn select(&self, title: &str, options: &[SelectOption]) -> Option<usize>;

    /// Text input. Returns None if no UI or cancelled.
    async fn input(&self, title: &str, placeholder: &str) -> Option<String>;

    /// Persistent status in footer.
    async fn set_status(&self, key: &str, text: Option<&str>);

    /// Widget above/below editor.
    async fn set_widget(&self, key: &str, content: Option<WidgetContent>);

    /// Full declarative custom component. Returns the serialized result.
    /// The component table from Lua/native code describes what to render.
    /// Rust renders it, routes key events, and returns when done() is called.
    async fn custom(&self, component: ComponentSpec) -> Option<serde_json::Value>;
}

pub enum NotifyLevel { Info, Warning, Error }

pub struct SelectOption {
    pub label: String,
    pub description: Option<String>,
}

pub enum WidgetContent {
    Lines(Vec<String>),
    Component(ComponentSpec),
}

/// Declarative component specification (from Lua tables or native code).
pub struct ComponentSpec {
    pub component_type: String,
    pub props: serde_json::Value,
    pub children: Vec<ComponentSpec>,
}
```

**Implementations:**

| Implementation | Mode | Behavior |
|---|---|---|
| `TuiInterface` | Interactive (`imp`) | Renders ratatui widgets inline |
| `StdioInterface` | Headless/RPC (`imp run`, `imp --mode rpc`) | Sends JSON event to stdout, waits for response on stdin. The parent process (imp_orch, custom UI) handles the actual interaction. |
| `NullInterface` | Print (`imp -p`) | `has_ui() = false`, returns None for everything |

In headless mode, the protocol for interactive requests:

```
stdout: {"type":"ui_request","id":"q1","method":"confirm","params":{"title":"Delete?","message":"Sure?"}}
stdin:  {"type":"ui_response","id":"q1","result":true}
```

imp_orch can handle these by spawning an osascript dialog, routing to Slack,
or showing a prompt in its own TUI. If no response within a configurable
timeout (default: 60s), returns None.

---

## Appendix A: File Locations

| What | Path |
|------|------|
| User config | `~/.config/imp/config.toml` |
| Auth storage | `~/.config/imp/auth.json` |
| Custom models | `~/.config/imp/models.toml` |
| User Lua extensions | `~/.config/imp/lua/` |
| User skills | `~/.config/imp/skills/` |
| User prompt templates | `~/.config/imp/prompts/` |
| User shell tools | `~/.config/imp/tools/` |
| User AGENTS.md | `~/.config/imp/AGENTS.md` |
| Sessions | `~/.local/share/imp/sessions/` |
| Project config | `.imp/config.toml` |
| Project Lua extensions | `.imp/lua/` |
| Project skills | `.imp/skills/` |
| Project prompt templates | `.imp/prompts/` |
| Project shell tools | `.imp/tools/` |

## Appendix B: Environment Variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OPENAI_API_KEY` | OpenAI API key |
| `GOOGLE_API_KEY` | Google Gemini API key |
| `IMP_MODEL` | Default model override |
| `IMP_THINKING` | Default thinking level override |
| `IMP_CONFIG` | Custom config file path |
| `IMP_LOG` | Log level (trace, debug, info, warn, error) |
| `EDITOR` | Editor for `/edit` command |
