use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures_core::Stream;
use serde::{Deserialize, Serialize};

use crate::auth::{ApiKey, AuthStore};
use crate::error::{Error, Result};
use crate::message::{AssistantMessage, ContentBlock, Message, StopReason};
use crate::model::{Capabilities, Model, ModelMeta, ModelPricing};
use crate::provider::{
    CacheOptions, Context, EffortLevel, Provider, RequestOptions, RetryPolicy, ThinkingLevel,
    ToolDefinition,
};
use crate::stream::StreamEvent;
use crate::usage::Usage;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

// ---------------------------------------------------------------------------
// Anthropic wire-format types (request)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ApiMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    system: Vec<ApiContentBlock>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ApiToolDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ApiThinking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_config: Option<ApiOutputConfig>,
}

#[derive(Debug, Serialize)]
struct ApiOutputConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    effort: Option<String>,
}

#[derive(Debug, Serialize)]
struct ApiMessage {
    role: String,
    content: Vec<ApiContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ApiContentBlock {
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: Vec<ApiContentBlock>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImageSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheControl {
    #[serde(rename = "type")]
    cache_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
}

fn ephemeral_cache() -> Option<CacheControl> {
    Some(CacheControl {
        cache_type: "ephemeral".into(),
        ttl: None,
        scope: None,
    })
}

fn make_cache_control(options: &CacheOptions) -> Option<CacheControl> {
    Some(CacheControl {
        cache_type: "ephemeral".into(),
        ttl: if options.extended_ttl {
            Some("1h".into())
        } else {
            None
        },
        scope: if options.global_scope {
            Some("global".into())
        } else {
            None
        },
    })
}

#[derive(Debug, Serialize)]
struct ApiToolDef {
    name: String,
    description: String,
    input_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<CacheControl>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ApiThinking {
    #[serde(rename = "enabled")]
    Enabled { budget_tokens: u32 },
    #[serde(rename = "adaptive")]
    Adaptive,
}

// ---------------------------------------------------------------------------
// Anthropic wire-format types (SSE response)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum SseEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: SseMessage },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: SseContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: SseDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: SseMessageDelta,
        usage: Option<SseUsage>,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error { error: SseError },
    /// Catch-all for unknown event types (forward compatibility).
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
struct SseMessage {
    model: Option<String>,
    usage: Option<SseUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum SseContentBlock {
    #[serde(rename = "text")]
    Text {
        #[allow(dead_code)]
        text: Option<String>,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        #[allow(dead_code)]
        input: Option<serde_json::Value>,
    },
    #[serde(rename = "thinking")]
    Thinking {
        #[allow(dead_code)]
        thinking: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(clippy::enum_variant_names)]
enum SseDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },
    /// Signature delta (model output verification) — safe to ignore.
    #[serde(rename = "signature_delta")]
    SignatureDelta {
        #[allow(dead_code)]
        signature: String,
    },
    /// Catch-all for future delta types.
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
struct SseMessageDelta {
    stop_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SseUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
    #[serde(default)]
    cache_read_input_tokens: u32,
    #[serde(default)]
    cache_creation_input_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct SseError {
    message: String,
}

// ---------------------------------------------------------------------------
// Non-streaming response types (for fallback when streaming fails mid-response)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct ApiResponse {
    model: String,
    content: Vec<ApiResponseBlock>,
    stop_reason: Option<String>,
    usage: SseUsage,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum ApiResponseBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

/// Convert a non-streaming API response into the sequence of StreamEvents
/// that the caller would have received from a streaming response.
#[allow(dead_code)]
pub(crate) fn non_streaming_response_to_events(resp: ApiResponse) -> Vec<StreamEvent> {
    let mut events = Vec::new();
    events.push(StreamEvent::MessageStart { model: resp.model });

    let mut content_blocks = Vec::new();
    for block in &resp.content {
        match block {
            ApiResponseBlock::Text { text } => {
                events.push(StreamEvent::TextDelta { text: text.clone() });
                content_blocks.push(ContentBlock::Text { text: text.clone() });
            }
            ApiResponseBlock::Thinking { thinking } => {
                events.push(StreamEvent::ThinkingDelta {
                    text: thinking.clone(),
                });
                content_blocks.push(ContentBlock::Thinking {
                    text: thinking.clone(),
                });
            }
            ApiResponseBlock::ToolUse { id, name, input } => {
                events.push(StreamEvent::ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: input.clone(),
                });
                content_blocks.push(ContentBlock::ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: input.clone(),
                });
            }
        }
    }

    let stop_reason = match resp.stop_reason.as_deref() {
        Some("end_turn") => StopReason::EndTurn,
        Some("tool_use") => StopReason::ToolUse,
        Some("max_tokens") => StopReason::MaxTokens,
        Some(other) => StopReason::Error(other.to_string()),
        None => StopReason::EndTurn,
    };

    let usage = Usage {
        input_tokens: resp.usage.input_tokens,
        output_tokens: resp.usage.output_tokens,
        cache_read_tokens: resp.usage.cache_read_input_tokens,
        cache_write_tokens: resp.usage.cache_creation_input_tokens,
    };

    events.push(StreamEvent::MessageEnd {
        message: AssistantMessage {
            content: content_blocks,
            usage: Some(usage),
            stop_reason,
            timestamp: crate::now(),
        },
    });

    events
}

// ---------------------------------------------------------------------------
// SSE stream state
// ---------------------------------------------------------------------------

/// Accumulated state for an in-flight content block.
#[derive(Debug)]
enum BlockState {
    Text,
    Thinking,
    ToolUse {
        id: String,
        name: String,
        json_buf: String,
    },
}

/// Tracks the SSE stream so we can assemble a final AssistantMessage.
struct StreamState {
    model: String,
    blocks: Vec<BlockState>,
    content: Vec<ContentBlock>,
    usage: Usage,
    stop_reason: StopReason,
}

impl StreamState {
    fn new() -> Self {
        Self {
            model: String::new(),
            blocks: Vec::new(),
            content: Vec::new(),
            usage: Usage::default(),
            stop_reason: StopReason::EndTurn,
        }
    }
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// Anthropic Messages API provider with streaming SSE support.
pub struct AnthropicProvider {
    client: reqwest::Client,
    retry_policy: RetryPolicy,
    models: Vec<ModelMeta>,
}

impl Default for AnthropicProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AnthropicProvider {
    pub fn new() -> Self {
        let client = super::streaming_http_client();

        Self {
            client,
            retry_policy: RetryPolicy::default(),
            models: builtin_models(),
        }
    }

    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

// ---------------------------------------------------------------------------
// Request building
// ---------------------------------------------------------------------------

fn thinking_budget(level: ThinkingLevel) -> Option<u32> {
    match level {
        ThinkingLevel::Off => None,
        ThinkingLevel::Minimal => Some(1024),
        ThinkingLevel::Low => Some(4096),
        ThinkingLevel::Medium => Some(10_000),
        ThinkingLevel::High => Some(32_000),
        ThinkingLevel::XHigh => Some(100_000),
    }
}

fn default_max_tokens(model: &Model, thinking_budget: Option<u32>, adaptive: bool) -> u32 {
    if adaptive {
        return model.meta.max_output_tokens;
    }

    // Anthropic is much happier when we do not default every request to the
    // model's absolute max output size, especially on larger Opus models.
    // Use a moderate default and only scale up when explicit thinking budgets
    // require it.
    let base = model.meta.max_output_tokens.min(8_192);
    match thinking_budget {
        Some(budget) => base.max(budget.saturating_add(1024)),
        None => base,
    }
}

fn model_supports_adaptive(model_id: &str) -> bool {
    model_id.contains("4-6") || model_id.contains("4.6")
}

fn beta_headers(model: &ModelMeta, effort: Option<EffortLevel>) -> Vec<&'static str> {
    let mut betas = vec![
        "interleaved-thinking-2025-05-14",
        "prompt-caching-scope-2026-01-05",
    ];

    if model.context_window > 200_000 {
        betas.push("context-1m-2025-08-07");
    }

    if effort.is_some() {
        betas.push("effort-2025-11-24");
    }

    betas
}

fn build_request(model: &Model, context: Context, options: RequestOptions) -> ApiRequest {
    let budget = thinking_budget(options.thinking_level);
    let supports_adaptive = model_supports_adaptive(&model.meta.id);
    let adaptive = supports_adaptive
        && matches!(
            options.thinking_level,
            ThinkingLevel::Medium | ThinkingLevel::High | ThinkingLevel::XHigh
        );

    let thinking = match budget {
        None => None,
        Some(b) if adaptive => Some(ApiThinking::Adaptive),
        Some(b) => Some(ApiThinking::Enabled { budget_tokens: b }),
    };

    // max_tokens: use explicit value, or a provider-tuned default, ensuring it
    // exceeds the requested thinking budget.
    let mut max_tokens = options
        .max_tokens
        .unwrap_or_else(|| default_max_tokens(model, budget, adaptive));
    if let Some(b) = budget {
        if !adaptive && max_tokens <= b {
            max_tokens = b + 1024;
        }
    }

    let system = build_system_blocks(&options.system_prompt, &options.cache_options);
    let tools = build_tool_defs(&options.tools, &options.cache_options);
    let messages = build_messages(&context.messages, &options.cache_options);

    // Temperature must not be set when thinking is enabled
    let temperature = if thinking.is_some() {
        None
    } else {
        options.temperature
    };

    let output_config = options.effort.map(|e| ApiOutputConfig {
        effort: Some(match e {
            EffortLevel::Low => "low".into(),
            EffortLevel::Medium => "medium".into(),
            EffortLevel::High => "high".into(),
        }),
    });

    ApiRequest {
        model: model.meta.id.clone(),
        max_tokens,
        messages,
        stream: true,
        system,
        tools,
        temperature,
        thinking,
        output_config,
    }
}

fn build_system_blocks(prompt: &str, cache: &CacheOptions) -> Vec<ApiContentBlock> {
    if prompt.is_empty() {
        return Vec::new();
    }
    vec![ApiContentBlock::Text {
        text: prompt.to_string(),
        cache_control: if cache.cache_system_prompt {
            ephemeral_cache()
        } else {
            None
        },
    }]
}

fn build_tool_defs(tools: &[ToolDefinition], cache: &CacheOptions) -> Vec<ApiToolDef> {
    // Sort tools alphabetically for prompt cache stability — prevents cache
    // busts when tools are registered in different orders between requests.
    let mut sorted: Vec<&ToolDefinition> = tools.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));

    let len = sorted.len();
    sorted
        .iter()
        .enumerate()
        .map(|(i, t)| {
            // Place cache breakpoint on the last tool definition
            let cc = if cache.cache_tools && i == len - 1 {
                make_cache_control(cache)
            } else {
                None
            };
            ApiToolDef {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.parameters.clone(),
                cache_control: cc,
            }
        })
        .collect()
}

fn build_messages(messages: &[Message], cache: &CacheOptions) -> Vec<ApiMessage> {
    let mut api_msgs: Vec<ApiMessage> = messages.iter().map(convert_message).collect();

    // Place cache breakpoints on the last N user-turn messages
    if cache.cache_recent_turns > 0 {
        let mut turns_tagged = 0;
        for msg in api_msgs.iter_mut().rev() {
            if msg.role == "user" {
                if let Some(last) = msg.content.last_mut() {
                    set_cache_control(last);
                }
                turns_tagged += 1;
                if turns_tagged >= cache.cache_recent_turns {
                    break;
                }
            }
        }
    }

    api_msgs
}

fn set_cache_control(block: &mut ApiContentBlock) {
    match block {
        ApiContentBlock::Text {
            ref mut cache_control,
            ..
        } => {
            *cache_control = ephemeral_cache();
        }
        ApiContentBlock::ToolResult { .. }
        | ApiContentBlock::Image { .. }
        | ApiContentBlock::ToolUse { .. }
        | ApiContentBlock::Thinking { .. } => {
            // Cache control not applicable to these block types in this context
        }
    }
}

fn convert_message(msg: &Message) -> ApiMessage {
    match msg {
        Message::User(u) => ApiMessage {
            role: "user".into(),
            content: u.content.iter().map(convert_content_block).collect(),
        },
        Message::Assistant(a) => ApiMessage {
            role: "assistant".into(),
            content: a.content.iter().map(convert_content_block).collect(),
        },
        Message::ToolResult(tr) => ApiMessage {
            role: "user".into(),
            content: vec![ApiContentBlock::ToolResult {
                tool_use_id: tr.tool_call_id.clone(),
                content: tr.content.iter().map(convert_content_block).collect(),
                is_error: if tr.is_error { Some(true) } else { None },
            }],
        },
    }
}

fn convert_content_block(block: &ContentBlock) -> ApiContentBlock {
    match block {
        ContentBlock::Text { text } => ApiContentBlock::Text {
            text: text.clone(),
            cache_control: None,
        },
        ContentBlock::Thinking { text } => ApiContentBlock::Thinking {
            thinking: text.clone(),
        },
        ContentBlock::ToolCall {
            id,
            name,
            arguments,
        } => ApiContentBlock::ToolUse {
            id: id.clone(),
            name: name.clone(),
            input: arguments.clone(),
        },
        ContentBlock::Image { media_type, data } => ApiContentBlock::Image {
            source: ImageSource {
                source_type: "base64".into(),
                media_type: media_type.clone(),
                data: data.clone(),
            },
        },
    }
}

// ---------------------------------------------------------------------------
// Tool definitions conversion
// ---------------------------------------------------------------------------

/// Convert a ToolDefinition to Anthropic's expected format.
#[cfg(test)]
fn convert_tool_def(tool: &ToolDefinition) -> ApiToolDef {
    ApiToolDef {
        name: tool.name.clone(),
        description: tool.description.clone(),
        input_schema: tool.parameters.clone(),
        cache_control: None,
    }
}

// ---------------------------------------------------------------------------
// SSE parsing
// ---------------------------------------------------------------------------

/// Parse a complete SSE frame from Anthropic's streaming response.
/// Returns None for non-data lines (comments, empty lines, event-type only lines).
fn parse_sse_event(data: &str) -> Result<Option<SseEvent>> {
    let trimmed = data.trim();
    if trimmed.is_empty() || trimmed == "[DONE]" {
        return Ok(None);
    }
    // SSE sends "event: <type>\ndata: <json>" — we only care about data lines.
    // Parse as JSON; unknown event types are caught by #[serde(other)].
    match serde_json::from_str(trimmed) {
        Ok(event) => Ok(Some(event)),
        Err(e) => {
            // Log but don't fail on unparseable events — forward compatibility
            eprintln!("[imp-llm] SSE parse warning: {e} (data: {:.200})", trimmed);
            Ok(None)
        }
    }
}

/// Process a sequence of SSE events into StreamEvents.
/// This is the core state machine for Anthropic's streaming protocol.
fn process_sse_event(event: SseEvent, state: &mut StreamState) -> Vec<StreamEvent> {
    let mut out = Vec::new();

    match event {
        SseEvent::MessageStart { message } => {
            if let Some(model) = message.model {
                state.model = model.clone();
                out.push(StreamEvent::MessageStart { model });
            }
            if let Some(u) = message.usage {
                state.usage.input_tokens = u.input_tokens;
                state.usage.cache_read_tokens = u.cache_read_input_tokens;
                state.usage.cache_write_tokens = u.cache_creation_input_tokens;
            }
        }
        SseEvent::ContentBlockStart {
            index,
            content_block,
        } => {
            // Ensure blocks vec is large enough
            while state.blocks.len() <= index {
                state.blocks.push(BlockState::Text);
            }
            match content_block {
                SseContentBlock::Text { .. } => {
                    state.blocks[index] = BlockState::Text;
                }
                SseContentBlock::ToolUse { id, name, .. } => {
                    state.blocks[index] = BlockState::ToolUse {
                        id,
                        name,
                        json_buf: String::new(),
                    };
                }
                SseContentBlock::Thinking { .. } => {
                    state.blocks[index] = BlockState::Thinking;
                }
            }
        }
        SseEvent::ContentBlockDelta { index, delta } => {
            if index < state.blocks.len() {
                match delta {
                    SseDelta::TextDelta { text } => {
                        out.push(StreamEvent::TextDelta { text });
                    }
                    SseDelta::ThinkingDelta { thinking } => {
                        out.push(StreamEvent::ThinkingDelta { text: thinking });
                    }
                    SseDelta::InputJsonDelta { partial_json } => {
                        if let BlockState::ToolUse {
                            ref mut json_buf, ..
                        } = state.blocks[index]
                        {
                            json_buf.push_str(&partial_json);
                        }
                    }
                    // Signature and unknown deltas are safely ignored
                    SseDelta::SignatureDelta { .. } | SseDelta::Unknown => {}
                }
            }
        }
        SseEvent::ContentBlockStop { index } => {
            if index < state.blocks.len() {
                match &state.blocks[index] {
                    BlockState::ToolUse { id, name, json_buf } => {
                        let arguments: serde_json::Value = serde_json::from_str(json_buf)
                            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                        let tc = StreamEvent::ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: arguments.clone(),
                        };
                        state.content.push(ContentBlock::ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            arguments,
                        });
                        out.push(tc);
                    }
                    BlockState::Text | BlockState::Thinking => {
                        // Text/thinking deltas were already emitted incrementally
                    }
                }
            }
        }
        SseEvent::MessageDelta { delta, usage } => {
            if let Some(reason) = delta.stop_reason {
                state.stop_reason = match reason.as_str() {
                    "end_turn" => StopReason::EndTurn,
                    "tool_use" => StopReason::ToolUse,
                    "max_tokens" => StopReason::MaxTokens,
                    other => StopReason::Error(other.to_string()),
                };
            }
            if let Some(u) = usage {
                state.usage.output_tokens = u.output_tokens;
            }
        }
        SseEvent::MessageStop => {
            let message = AssistantMessage {
                content: std::mem::take(&mut state.content),
                usage: Some(state.usage.clone()),
                stop_reason: state.stop_reason.clone(),
                timestamp: crate::now(),
            };
            out.push(StreamEvent::MessageEnd { message });
        }
        SseEvent::Ping | SseEvent::Unknown => {}
        SseEvent::Error { error } => {
            out.push(StreamEvent::Error {
                error: error.message,
            });
        }
    }

    out
}

/// Parse raw SSE text from the Anthropic API into StreamEvents.
///
/// The SSE protocol sends lines like:
/// ```text
/// event: message_start
/// data: {"type": "message_start", ...}
///
/// event: content_block_delta
/// data: {"type": "content_block_delta", ...}
/// ```
///
/// We extract "data:" lines and parse them as JSON.
#[cfg(test)]
fn parse_sse_stream(raw: &str, state: &mut StreamState) -> Vec<Result<StreamEvent>> {
    let mut events = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if let Some(data) = line.strip_prefix("data: ") {
            match parse_sse_event(data) {
                Ok(Some(sse)) => {
                    for ev in process_sse_event(sse, state) {
                        events.push(Ok(ev));
                    }
                }
                Ok(None) => {}
                Err(e) => events.push(Err(e)),
            }
        }
    }

    events
}

// ---------------------------------------------------------------------------
// Streaming implementation using channels
// ---------------------------------------------------------------------------

/// Create a streaming response from the Anthropic API.
/// Returns a Stream of StreamEvents.
/// Maximum number of retries for transient errors.
const MAX_RETRIES: u32 = 8;

/// Maximum consecutive 529 (overloaded) errors before giving up.
const MAX_CONSECUTIVE_529: u32 = 3;

/// Floor for max_tokens when recovering from context overflow.
pub const FLOOR_OUTPUT_TOKENS: u32 = 3_000;

/// Default max_tokens cap (matches Claude Code's capped default).
pub const DEFAULT_MAX_TOKENS: u32 = 8_192;

/// Escalated max_tokens for retry after truncation.
pub const ESCALATED_MAX_TOKENS: u32 = 64_000;

/// Check if an HTTP status code is retryable.
fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 401 | 429 | 500 | 502 | 503 | 529)
}

/// Compute backoff delay, honoring retry-after header if present.
fn retry_delay(attempt: u32) -> std::time::Duration {
    let base_ms = 1000u64 * 2u64.pow(attempt.min(5)); // cap at 32s
    let jitter_ms = rand::random::<u64>() % 500;
    std::time::Duration::from_millis(base_ms + jitter_ms)
}

/// Parse retry-after header value (seconds) into a Duration.
fn retry_after_delay(headers: &reqwest::header::HeaderMap) -> Option<std::time::Duration> {
    let val = headers.get("retry-after")?.to_str().ok()?;
    let secs: u64 = val.parse().ok()?;
    Some(std::time::Duration::from_secs(secs))
}

/// Parse context overflow error: "input length and `max_tokens` exceed context limit: X + Y > Z"
pub fn parse_context_overflow(body: &str) -> Option<(u32, u32, u32)> {
    let needle = "input length and `max_tokens` exceed context limit: ";
    let rest = body.find(needle).map(|i| &body[i + needle.len()..])?;
    let parts: Vec<&str> = rest
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() >= 3 {
        let input: u32 = parts[0].parse().ok()?;
        let max: u32 = parts[1].parse().ok()?;
        let limit: u32 = parts[2].parse().ok()?;
        Some((input, max, limit))
    } else {
        None
    }
}

fn stream_response(
    client: reqwest::Client,
    api_key: String,
    request: ApiRequest,
    betas: Vec<&'static str>,
) -> Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>> {
    let (tx, rx) = futures::channel::mpsc::unbounded();

    tokio::spawn(async move {
        let is_oauth = api_key.starts_with("sk-ant-oat");

        // Retry loop for transient failures (connection drops, 429, 5xx, 529)
        let mut attempt = 0u32;
        let mut consecutive_529 = 0u32;
        let mut had_401 = false;
        let resp = loop {
            let mut req = client
                .post(API_URL)
                .header("anthropic-version", API_VERSION)
                .header("content-type", "application/json");

            let mut request_betas = betas.clone();

            if is_oauth {
                request_betas.insert(0, "oauth-2025-04-20");
                req = req
                    .header("authorization", format!("Bearer {api_key}"))
                    .header("anthropic-dangerous-direct-browser-access", "true");
            } else {
                req = req.header("x-api-key", &api_key);
            }

            req = req.header("anthropic-beta", request_betas.join(","));

            let result = req.json(&request).send().await;

            match result {
                Ok(r) => {
                    let status = r.status();
                    if status.is_success() {
                        break r;
                    }

                    // Track consecutive 529 (overloaded) errors
                    if status.as_u16() == 529 {
                        consecutive_529 += 1;
                        if consecutive_529 >= MAX_CONSECUTIVE_529 {
                            let body = r.text().await.unwrap_or_default();
                            let _ = tx.unbounded_send(Err(Error::Provider(format!(
                                "API overloaded after {} consecutive 529 errors: {body}",
                                MAX_CONSECUTIVE_529
                            ))));
                            return;
                        }
                    } else {
                        consecutive_529 = 0;
                    }

                    // 401: retry once (token may have expired)
                    if status.as_u16() == 401 {
                        if had_401 {
                            let body = r.text().await.unwrap_or_default();
                            let _ = tx.unbounded_send(Err(Error::Provider(format!(
                                "HTTP 401 (authentication failed): {body}"
                            ))));
                            return;
                        }
                        had_401 = true;
                        eprintln!(
                            "[imp-llm] HTTP 401, retrying once (credentials may have expired)"
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        continue;
                    }

                    // Retryable HTTP error
                    if is_retryable_status(status) && attempt < MAX_RETRIES {
                        // Honor retry-after header if present
                        let delay =
                            retry_after_delay(r.headers()).unwrap_or_else(|| retry_delay(attempt));
                        eprintln!(
                            "[imp-llm] HTTP {status}, retrying in {}s (attempt {}/{})",
                            delay.as_secs(),
                            attempt + 1,
                            MAX_RETRIES
                        );
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                        continue;
                    }
                    // Non-retryable or exhausted retries
                    let body = r.text().await.unwrap_or_default();
                    let _ =
                        tx.unbounded_send(Err(Error::Provider(format!("HTTP {status}: {body}"))));
                    return;
                }
                Err(e) => {
                    // Connection/timeout errors are retryable
                    let is_transient = e.is_connect() || e.is_timeout() || e.is_request();
                    if is_transient && attempt < MAX_RETRIES {
                        let delay = retry_delay(attempt);
                        eprintln!(
                            "[imp-llm] Connection error: {e}, retrying in {}s (attempt {}/{})",
                            delay.as_secs(),
                            attempt + 1,
                            MAX_RETRIES
                        );
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                        continue;
                    }
                    let _ = tx.unbounded_send(Err(Error::Http(e)));
                    return;
                }
            }
        };

        let mut state = StreamState::new();
        let mut buf = String::new();
        let mut byte_stream = resp.bytes_stream();

        use futures::StreamExt;
        while let Some(chunk) = byte_stream.next().await {
            match chunk {
                Ok(bytes) => {
                    buf.push_str(&String::from_utf8_lossy(&bytes));

                    // Process complete lines
                    while let Some(pos) = buf.find('\n') {
                        let line = buf[..pos].to_string();
                        buf = buf[pos + 1..].to_string();

                        let trimmed = line.trim();
                        if let Some(data) = trimmed.strip_prefix("data: ") {
                            match parse_sse_event(data) {
                                Ok(Some(sse)) => {
                                    for ev in process_sse_event(sse, &mut state) {
                                        if tx.unbounded_send(Ok(ev)).is_err() {
                                            return;
                                        }
                                    }
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    if tx.unbounded_send(Err(e)).is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.unbounded_send(Err(Error::Http(e)));
                    return;
                }
            }
        }
    });

    Box::pin(rx)
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn stream(
        &self,
        model: &Model,
        context: Context,
        options: RequestOptions,
        api_key: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>> {
        // OAuth tokens are scoped to Claude Code's identity. Anthropic rejects
        // requests with custom system prompts or tool definitions that don't match
        // the expected Claude Code format. When using OAuth:
        // 1. Always use the required system prompt
        // 2. Prepend any custom instructions to the first user message
        let mut options = options;
        let mut context = context;
        let oauth_system = "You are Claude Code, Anthropic's official CLI for Claude.".to_string();
        if api_key.starts_with("sk-ant-oat") {
            if !options.system_prompt.is_empty() && options.system_prompt != oauth_system {
                // Move custom system prompt into user message context
                let prefix = format!(
                    "<instructions>\n{}\n</instructions>\n\n",
                    options.system_prompt
                );
                if let Some(crate::message::Message::User(user_msg)) = context.messages.first_mut()
                {
                    let original = user_msg
                        .content
                        .iter()
                        .filter_map(|b| match b {
                            crate::message::ContentBlock::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    user_msg.content = vec![crate::message::ContentBlock::Text {
                        text: format!("{prefix}{original}"),
                    }];
                }
            }
            options.system_prompt = oauth_system;
        }
        let effort = options.effort;
        let request = build_request(model, context, options);
        let client = self.client.clone();
        let api_key = api_key.to_string();
        let betas = beta_headers(&model.meta, effort);
        stream_response(client, api_key, request, betas)
    }

    async fn resolve_auth(&self, auth: &AuthStore) -> Result<ApiKey> {
        auth.resolve("anthropic")
    }

    fn id(&self) -> &str {
        "anthropic"
    }

    fn models(&self) -> &[ModelMeta] {
        &self.models
    }
}

// ---------------------------------------------------------------------------
// Built-in models
// ---------------------------------------------------------------------------

fn builtin_models() -> Vec<ModelMeta> {
    vec![
        ModelMeta {
            id: "claude-sonnet-4-6".into(),
            provider: "anthropic".into(),
            name: "Claude Sonnet 4.6".into(),
            context_window: 1_000_000,
            max_output_tokens: 128_000,
            pricing: ModelPricing {
                input_per_mtok: 3.0,
                output_per_mtok: 15.0,
                cache_read_per_mtok: 0.3,
                cache_write_per_mtok: 3.75,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "claude-sonnet-4-20250514".into(),
            provider: "anthropic".into(),
            name: "Claude Sonnet 4".into(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing {
                input_per_mtok: 3.0,
                output_per_mtok: 15.0,
                cache_read_per_mtok: 0.3,
                cache_write_per_mtok: 3.75,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "claude-haiku-3-5-20241022".into(),
            provider: "anthropic".into(),
            name: "Claude 3.5 Haiku".into(),
            context_window: 200_000,
            max_output_tokens: 8_192,
            pricing: ModelPricing {
                input_per_mtok: 0.80,
                output_per_mtok: 4.0,
                cache_read_per_mtok: 0.08,
                cache_write_per_mtok: 1.0,
            },
            capabilities: Capabilities {
                reasoning: false,
                images: true,
                tool_use: true,
            },
        },
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{ToolResultMessage, UserMessage};
    use crate::provider::CacheOptions;

    // -- Request serialization tests --

    #[test]
    fn serialize_text_user_message() {
        let msg = Message::User(UserMessage {
            content: vec![ContentBlock::Text {
                text: "Hello".into(),
            }],
            timestamp: 0,
        });
        let api = convert_message(&msg);
        assert_eq!(api.role, "user");
        let json = serde_json::to_value(&api.content).unwrap();
        assert_eq!(json[0]["type"], "text");
        assert_eq!(json[0]["text"], "Hello");
    }

    #[test]
    fn serialize_image_content_block() {
        let block = ContentBlock::Image {
            media_type: "image/png".into(),
            data: "iVBOR...".into(),
        };
        let api = convert_content_block(&block);
        let json = serde_json::to_value(&api).unwrap();
        assert_eq!(json["type"], "image");
        assert_eq!(json["source"]["type"], "base64");
        assert_eq!(json["source"]["media_type"], "image/png");
        assert_eq!(json["source"]["data"], "iVBOR...");
    }

    #[test]
    fn serialize_tool_call_block() {
        let block = ContentBlock::ToolCall {
            id: "call_1".into(),
            name: "bash".into(),
            arguments: serde_json::json!({"command": "ls"}),
        };
        let api = convert_content_block(&block);
        let json = serde_json::to_value(&api).unwrap();
        assert_eq!(json["type"], "tool_use");
        assert_eq!(json["id"], "call_1");
        assert_eq!(json["name"], "bash");
        assert_eq!(json["input"]["command"], "ls");
    }

    #[test]
    fn serialize_tool_result_message() {
        let msg = Message::ToolResult(ToolResultMessage {
            tool_call_id: "call_1".into(),
            tool_name: "bash".into(),
            content: vec![ContentBlock::Text {
                text: "file.txt".into(),
            }],
            is_error: false,
            details: serde_json::Value::Null,
            timestamp: 0,
        });
        let api = convert_message(&msg);
        assert_eq!(api.role, "user");
        let json = serde_json::to_value(&api.content).unwrap();
        assert_eq!(json[0]["type"], "tool_result");
        assert_eq!(json[0]["tool_use_id"], "call_1");
    }

    #[test]
    fn serialize_tool_result_with_error() {
        let msg = Message::ToolResult(ToolResultMessage {
            tool_call_id: "call_2".into(),
            tool_name: "bash".into(),
            content: vec![ContentBlock::Text {
                text: "permission denied".into(),
            }],
            is_error: true,
            details: serde_json::Value::Null,
            timestamp: 0,
        });
        let api = convert_message(&msg);
        let json = serde_json::to_value(&api.content).unwrap();
        assert_eq!(json[0]["is_error"], true);
    }

    #[test]
    fn serialize_thinking_block() {
        let block = ContentBlock::Thinking {
            text: "Let me think...".into(),
        };
        let api = convert_content_block(&block);
        let json = serde_json::to_value(&api).unwrap();
        assert_eq!(json["type"], "thinking");
        assert_eq!(json["thinking"], "Let me think...");
    }

    #[test]
    fn serialize_assistant_message() {
        let msg = Message::Assistant(AssistantMessage {
            content: vec![
                ContentBlock::Text {
                    text: "Here:".into(),
                },
                ContentBlock::ToolCall {
                    id: "tc_1".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({"path": "a.rs"}),
                },
            ],
            usage: None,
            stop_reason: StopReason::ToolUse,
            timestamp: 0,
        });
        let api = convert_message(&msg);
        assert_eq!(api.role, "assistant");
        assert_eq!(api.content.len(), 2);
        let json = serde_json::to_value(&api.content).unwrap();
        assert_eq!(json[0]["type"], "text");
        assert_eq!(json[1]["type"], "tool_use");
    }

    // -- Cache control tests --

    #[test]
    fn cache_system_prompt() {
        let cache = CacheOptions {
            cache_system_prompt: true,
            cache_tools: false,
            cache_recent_turns: 0,
            ..Default::default()
        };
        let blocks = build_system_blocks("You are helpful.", &cache);
        let json = serde_json::to_value(&blocks[0]).unwrap();
        assert_eq!(json["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn no_cache_system_prompt() {
        let cache = CacheOptions::default();
        let blocks = build_system_blocks("You are helpful.", &cache);
        let json = serde_json::to_value(&blocks[0]).unwrap();
        assert!(json.get("cache_control").is_none());
    }

    #[test]
    fn cache_on_last_tool_def() {
        let tools = vec![
            ToolDefinition {
                name: "read".into(),
                description: "Read file".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
            ToolDefinition {
                name: "write".into(),
                description: "Write file".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
        ];
        let cache = CacheOptions {
            cache_system_prompt: false,
            cache_tools: true,
            cache_recent_turns: 0,
            ..Default::default()
        };
        let api_tools = build_tool_defs(&tools, &cache);
        assert!(api_tools[0].cache_control.is_none());
        assert!(api_tools[1].cache_control.is_some());
    }

    #[test]
    fn cache_recent_user_turns() {
        let messages = vec![
            Message::user("first"),
            Message::Assistant(AssistantMessage {
                content: vec![ContentBlock::Text {
                    text: "reply".into(),
                }],
                usage: None,
                stop_reason: StopReason::EndTurn,
                timestamp: 0,
            }),
            Message::user("second"),
            Message::Assistant(AssistantMessage {
                content: vec![ContentBlock::Text {
                    text: "reply2".into(),
                }],
                usage: None,
                stop_reason: StopReason::EndTurn,
                timestamp: 0,
            }),
            Message::user("third"),
        ];
        let cache = CacheOptions {
            cache_system_prompt: false,
            cache_tools: false,
            cache_recent_turns: 2,
            ..Default::default()
        };
        let api_msgs = build_messages(&messages, &cache);

        // Last 2 user messages (indices 2 and 4) should have cache_control
        // First user (index 0) should not
        let json0 = serde_json::to_value(&api_msgs[0].content).unwrap();
        assert!(json0[0].get("cache_control").is_none());

        let json2 = serde_json::to_value(&api_msgs[2].content).unwrap();
        assert_eq!(json2[0]["cache_control"]["type"], "ephemeral");

        let json4 = serde_json::to_value(&api_msgs[4].content).unwrap();
        assert_eq!(json4[0]["cache_control"]["type"], "ephemeral");
    }

    // -- Thinking budget tests --

    #[test]
    fn thinking_budget_off() {
        assert_eq!(thinking_budget(ThinkingLevel::Off), None);
    }

    #[test]
    fn thinking_budget_minimal() {
        assert_eq!(thinking_budget(ThinkingLevel::Minimal), Some(1024));
    }

    #[test]
    fn thinking_budget_low() {
        assert_eq!(thinking_budget(ThinkingLevel::Low), Some(4096));
    }

    #[test]
    fn thinking_budget_medium() {
        assert_eq!(thinking_budget(ThinkingLevel::Medium), Some(10_000));
    }

    #[test]
    fn thinking_budget_high() {
        assert_eq!(thinking_budget(ThinkingLevel::High), Some(32_000));
    }

    #[test]
    fn thinking_budget_xhigh() {
        assert_eq!(thinking_budget(ThinkingLevel::XHigh), Some(100_000));
    }

    #[test]
    fn test_beta_headers_large_context() {
        let model_meta = ModelMeta {
            id: "claude-sonnet-4-6".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 1_000_000,
            max_output_tokens: 128_000,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };

        let betas = beta_headers(&model_meta, None);
        assert!(betas.contains(&"interleaved-thinking-2025-05-14"));
        assert!(betas.contains(&"prompt-caching-scope-2026-01-05"));
        assert!(betas.contains(&"context-1m-2025-08-07"));
    }

    #[test]
    fn test_beta_headers_standard_context() {
        let model_meta = ModelMeta {
            id: "claude-haiku-3-5-20241022".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 200_000,
            max_output_tokens: 8_192,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };

        let betas = beta_headers(&model_meta, None);
        assert!(betas.contains(&"interleaved-thinking-2025-05-14"));
        assert!(betas.contains(&"prompt-caching-scope-2026-01-05"));
        assert!(!betas.contains(&"context-1m-2025-08-07"));
    }

    #[test]
    fn test_beta_headers_always_includes_interleaved() {
        let standard = ModelMeta {
            id: "claude-haiku-3-5-20241022".into(),
            provider: "anthropic".into(),
            name: "standard".into(),
            context_window: 200_000,
            max_output_tokens: 8_192,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let large = ModelMeta {
            id: "claude-opus-4-6".into(),
            provider: "anthropic".into(),
            name: "large".into(),
            context_window: 1_000_000,
            max_output_tokens: 128_000,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };

        assert!(beta_headers(&standard, None).contains(&"interleaved-thinking-2025-05-14"));
        assert!(beta_headers(&large, None).contains(&"interleaved-thinking-2025-05-14"));
    }

    #[test]
    fn default_max_tokens_caps_large_models() {
        let model_meta = ModelMeta {
            id: "claude-opus-4-6".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 1_000_000,
            max_output_tokens: 128_000,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = AnthropicProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };
        let req = build_request(&model, Context::default(), RequestOptions::default());
        assert_eq!(req.max_tokens, 8_192);
        assert!(req.thinking.is_none());
    }

    #[test]
    fn thinking_forces_max_tokens_above_budget() {
        let model_meta = ModelMeta {
            id: "claude-sonnet-4-20250514".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 200_000,
            max_output_tokens: 4096,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = AnthropicProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };
        let context = Context::default();
        let options = RequestOptions {
            thinking_level: ThinkingLevel::High,
            max_tokens: None,
            ..Default::default()
        };
        let req = build_request(&model, context, options);
        // Budget is 32000, max_output is 4096. Should be bumped to 33024.
        assert!(req.max_tokens > 32_000);
        assert!(req.thinking.is_some());
        let t = serde_json::to_value(req.thinking.unwrap()).unwrap();
        assert_eq!(
            t,
            serde_json::json!({"type": "enabled", "budget_tokens": 32_000})
        );
    }

    #[test]
    fn thinking_off_allows_temperature() {
        let model_meta = ModelMeta {
            id: "claude-haiku-3-5-20241022".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 200_000,
            max_output_tokens: 8192,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = AnthropicProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };
        let options = RequestOptions {
            thinking_level: ThinkingLevel::Off,
            temperature: Some(0.5),
            ..Default::default()
        };
        let req = build_request(&model, Context::default(), options);
        assert_eq!(req.temperature, Some(0.5));
        assert!(req.thinking.is_none());
    }

    #[test]
    fn test_adaptive_thinking_sonnet_46() {
        let model_meta = ModelMeta {
            id: "claude-sonnet-4-6".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 1_000_000,
            max_output_tokens: 128_000,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = AnthropicProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };
        let options = RequestOptions {
            thinking_level: ThinkingLevel::Medium,
            ..Default::default()
        };

        let req = build_request(&model, Context::default(), options);
        let thinking_json = serde_json::to_value(req.thinking.unwrap()).unwrap();
        assert_eq!(thinking_json, serde_json::json!({"type": "adaptive"}));
    }

    #[test]
    fn test_budget_thinking_sonnet_40() {
        let model_meta = ModelMeta {
            id: "claude-sonnet-4-20250514".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = AnthropicProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };
        let options = RequestOptions {
            thinking_level: ThinkingLevel::Medium,
            ..Default::default()
        };

        let req = build_request(&model, Context::default(), options);
        let thinking_json = serde_json::to_value(req.thinking.unwrap()).unwrap();
        assert_eq!(
            thinking_json,
            serde_json::json!({"type": "enabled", "budget_tokens": 10_000})
        );
    }

    #[test]
    fn test_adaptive_still_caps_low_levels() {
        let model_meta = ModelMeta {
            id: "claude-sonnet-4-6".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 1_000_000,
            max_output_tokens: 128_000,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = AnthropicProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };
        let options = RequestOptions {
            thinking_level: ThinkingLevel::Minimal,
            ..Default::default()
        };

        let req = build_request(&model, Context::default(), options);
        let thinking_json = serde_json::to_value(req.thinking.unwrap()).unwrap();
        assert_eq!(
            thinking_json,
            serde_json::json!({"type": "enabled", "budget_tokens": 1024})
        );
    }

    #[test]
    fn thinking_enabled_strips_temperature() {
        let model_meta = ModelMeta {
            id: "claude-sonnet-4-20250514".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = AnthropicProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };
        let options = RequestOptions {
            thinking_level: ThinkingLevel::Medium,
            temperature: Some(0.7),
            ..Default::default()
        };
        let req = build_request(&model, Context::default(), options);
        assert!(req.temperature.is_none());
        assert!(req.thinking.is_some());
    }

    #[test]
    fn test_adaptive_max_tokens_not_capped() {
        let model_meta = ModelMeta {
            id: "claude-sonnet-4.6".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 1_000_000,
            max_output_tokens: 128_000,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = AnthropicProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };
        let options = RequestOptions {
            thinking_level: ThinkingLevel::Medium,
            ..Default::default()
        };

        let req = build_request(&model, Context::default(), options);
        assert_eq!(req.max_tokens, 128_000);
        let thinking_json = serde_json::to_value(req.thinking.unwrap()).unwrap();
        assert_eq!(thinking_json, serde_json::json!({"type": "adaptive"}));
    }

    // -- SSE parsing tests --

    #[test]
    fn parse_message_start_event() {
        let data = r#"{"type":"message_start","message":{"model":"claude-sonnet-4-20250514","usage":{"input_tokens":100,"output_tokens":0,"cache_read_input_tokens":50,"cache_creation_input_tokens":10}}}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        let mut state = StreamState::new();
        let events = process_sse_event(event, &mut state);
        assert_eq!(events.len(), 1);
        assert!(
            matches!(&events[0], StreamEvent::MessageStart { model } if model == "claude-sonnet-4-20250514")
        );
        assert_eq!(state.usage.input_tokens, 100);
        assert_eq!(state.usage.cache_read_tokens, 50);
        assert_eq!(state.usage.cache_write_tokens, 10);
    }

    #[test]
    fn parse_text_delta_event() {
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        let mut state = StreamState::new();
        state.blocks.push(BlockState::Text);
        let events = process_sse_event(event, &mut state);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StreamEvent::TextDelta { text } if text == "Hello"));
    }

    #[test]
    fn parse_thinking_delta_event() {
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"reasoning..."}}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        let mut state = StreamState::new();
        state.blocks.push(BlockState::Thinking);
        let events = process_sse_event(event, &mut state);
        assert_eq!(events.len(), 1);
        assert!(
            matches!(&events[0], StreamEvent::ThinkingDelta { text } if text == "reasoning...")
        );
    }

    #[test]
    fn parse_tool_use_accumulates_json() {
        let mut state = StreamState::new();

        // content_block_start for tool_use
        let start = r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"bash","input":{}}}"#;
        let event = parse_sse_event(start).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);
        assert!(events.is_empty());

        // Two delta chunks
        let d1 = r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"com"}}"#;
        let event = parse_sse_event(d1).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);
        assert!(events.is_empty());

        let d2 = r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"mand\":\"ls\"}"}}"#;
        let event = parse_sse_event(d2).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);
        assert!(events.is_empty());

        // content_block_stop emits the tool call
        let stop = r#"{"type":"content_block_stop","index":0}"#;
        let event = parse_sse_event(stop).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);
        assert_eq!(events.len(), 1);
        if let StreamEvent::ToolCall {
            id,
            name,
            arguments,
        } = &events[0]
        {
            assert_eq!(id, "toolu_1");
            assert_eq!(name, "bash");
            assert_eq!(arguments["command"], "ls");
        } else {
            panic!("expected ToolCall event");
        }
    }

    #[test]
    fn parse_message_delta_and_stop() {
        let mut state = StreamState::new();
        state.model = "claude-sonnet-4-20250514".into();

        let delta = r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":42}}"#;
        let event = parse_sse_event(delta).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);
        assert!(events.is_empty());
        assert_eq!(state.stop_reason, StopReason::EndTurn);
        assert_eq!(state.usage.output_tokens, 42);

        let stop = r#"{"type":"message_stop"}"#;
        let event = parse_sse_event(stop).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);
        assert_eq!(events.len(), 1);
        assert!(
            matches!(&events[0], StreamEvent::MessageEnd { message } if message.stop_reason == StopReason::EndTurn)
        );
    }

    #[test]
    fn parse_tool_use_stop_reason() {
        let mut state = StreamState::new();
        let delta = r#"{"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":10}}"#;
        let event = parse_sse_event(delta).unwrap().unwrap();
        process_sse_event(event, &mut state);
        assert_eq!(state.stop_reason, StopReason::ToolUse);
    }

    #[test]
    fn parse_max_tokens_stop_reason() {
        let mut state = StreamState::new();
        let delta = r#"{"type":"message_delta","delta":{"stop_reason":"max_tokens"},"usage":{"output_tokens":10}}"#;
        let event = parse_sse_event(delta).unwrap().unwrap();
        process_sse_event(event, &mut state);
        assert_eq!(state.stop_reason, StopReason::MaxTokens);
    }

    #[test]
    fn parse_error_event() {
        let data = r#"{"type":"error","error":{"message":"Overloaded"}}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        let mut state = StreamState::new();
        let events = process_sse_event(event, &mut state);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StreamEvent::Error { error } if error == "Overloaded"));
    }

    #[test]
    fn parse_ping_event() {
        let data = r#"{"type":"ping"}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        let mut state = StreamState::new();
        let events = process_sse_event(event, &mut state);
        assert!(events.is_empty());
    }

    #[test]
    fn parse_full_sse_stream() {
        let raw = "\
event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"model\":\"claude-sonnet-4-20250514\",\"usage\":{\"input_tokens\":10,\"output_tokens\":0,\"cache_read_input_tokens\":0,\"cache_creation_input_tokens\":0}}}\n\
\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi!\"}}\n\
\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\
\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":5}}\n\
\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\
";
        let mut state = StreamState::new();
        let events = parse_sse_stream(raw, &mut state);
        let events: Vec<_> = events
            .into_iter()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();

        // MessageStart, TextDelta("Hi!"), MessageEnd
        assert_eq!(events.len(), 3);
        assert!(matches!(&events[0], StreamEvent::MessageStart { .. }));
        assert!(matches!(&events[1], StreamEvent::TextDelta { text } if text == "Hi!"));
        assert!(matches!(&events[2], StreamEvent::MessageEnd { .. }));
    }

    #[test]
    fn full_request_round_trip_json() {
        // Build a realistic request and verify it serializes to expected Anthropic format
        let model_meta = ModelMeta {
            id: "claude-sonnet-4-20250514".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = AnthropicProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };

        let context = Context {
            messages: vec![
                Message::user("What files are in this directory?"),
                Message::Assistant(AssistantMessage {
                    content: vec![ContentBlock::ToolCall {
                        id: "tc_1".into(),
                        name: "bash".into(),
                        arguments: serde_json::json!({"command": "ls"}),
                    }],
                    usage: None,
                    stop_reason: StopReason::ToolUse,
                    timestamp: 0,
                }),
                Message::ToolResult(ToolResultMessage {
                    tool_call_id: "tc_1".into(),
                    tool_name: "bash".into(),
                    content: vec![ContentBlock::Text {
                        text: "README.md\nsrc/".into(),
                    }],
                    is_error: false,
                    details: serde_json::Value::Null,
                    timestamp: 0,
                }),
            ],
        };

        let options = RequestOptions {
            system_prompt: "You are a helpful assistant.".into(),
            tools: vec![ToolDefinition {
                name: "bash".into(),
                description: "Run a bash command".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" }
                    },
                    "required": ["command"]
                }),
            }],
            cache_options: CacheOptions {
                cache_system_prompt: true,
                cache_tools: true,
                cache_recent_turns: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        let req = build_request(&model, context, options);
        let json = serde_json::to_value(&req).unwrap();

        // Verify structure
        assert_eq!(json["model"], "claude-sonnet-4-20250514");
        assert_eq!(json["stream"], true);
        assert!(json["max_tokens"].as_u64().unwrap() > 0);

        // System has cache_control
        assert_eq!(json["system"][0]["cache_control"]["type"], "ephemeral");

        // Tools has cache_control on last
        assert_eq!(json["tools"][0]["cache_control"]["type"], "ephemeral");

        // Messages structure
        assert_eq!(json["messages"].as_array().unwrap().len(), 3);
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][1]["role"], "assistant");
        assert_eq!(json["messages"][1]["content"][0]["type"], "tool_use");
        assert_eq!(json["messages"][2]["role"], "user");
        assert_eq!(json["messages"][2]["content"][0]["type"], "tool_result");
    }

    // -- Tool definition conversion test --

    #[test]
    fn convert_tool_definition() {
        let tool = ToolDefinition {
            name: "read_file".into(),
            description: "Read a file from disk".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" }
                },
                "required": ["path"]
            }),
        };
        let api = convert_tool_def(&tool);
        let json = serde_json::to_value(&api).unwrap();
        assert_eq!(json["name"], "read_file");
        assert_eq!(json["description"], "Read a file from disk");
        assert_eq!(json["input_schema"]["type"], "object");
        assert_eq!(json["input_schema"]["properties"]["path"]["type"], "string");
    }

    // -- Edge case: SSE parsing --

    #[test]
    fn parse_sse_event_empty_string_returns_none() {
        let result = parse_sse_event("").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn parse_sse_event_whitespace_only_returns_none() {
        let result = parse_sse_event("   \n  ").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn parse_sse_event_malformed_json_returns_none() {
        // Malformed JSON is treated as an unparseable event and skipped
        // (forward compatibility — don't crash on unknown SSE event formats).
        let result = parse_sse_event("{not valid json}");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn sse_stream_skips_non_data_lines() {
        // Lines without "data: " prefix should be ignored
        let raw = "\
event: message_start\n\
: this is a comment\n\
data: {\"type\":\"message_start\",\"message\":{\"model\":\"claude-sonnet-4-20250514\",\"usage\":{\"input_tokens\":5,\"output_tokens\":0,\"cache_read_input_tokens\":0,\"cache_creation_input_tokens\":0}}}\n\
\n\
some random line\n\
data: {\"type\":\"message_stop\"}\n";
        let mut state = StreamState::new();
        let events = parse_sse_stream(raw, &mut state);
        let events: Vec<_> = events.into_iter().filter_map(|e| e.ok()).collect();
        // Should get MessageStart and MessageEnd only
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], StreamEvent::MessageStart { .. }));
        assert!(matches!(&events[1], StreamEvent::MessageEnd { .. }));
    }

    #[test]
    fn tool_call_with_empty_json_arguments() {
        let mut state = StreamState::new();

        let start = r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_empty","name":"noop","input":{}}}"#;
        let event = parse_sse_event(start).unwrap().unwrap();
        process_sse_event(event, &mut state);

        // Empty JSON object as the accumulated buffer
        let d1 = r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{}"}}"#;
        let event = parse_sse_event(d1).unwrap().unwrap();
        process_sse_event(event, &mut state);

        let stop = r#"{"type":"content_block_stop","index":0}"#;
        let event = parse_sse_event(stop).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);

        assert_eq!(events.len(), 1);
        if let StreamEvent::ToolCall { arguments, .. } = &events[0] {
            assert!(arguments.is_object());
            assert!(arguments.as_object().unwrap().is_empty());
        } else {
            panic!("expected ToolCall");
        }
    }

    #[test]
    fn message_delta_missing_usage_defaults_to_zero() {
        let mut state = StreamState::new();
        let data = r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        process_sse_event(event, &mut state);
        // output_tokens should remain 0 since no usage was provided
        assert_eq!(state.usage.output_tokens, 0);
        assert_eq!(state.stop_reason, StopReason::EndTurn);
    }

    #[test]
    fn unknown_stop_reason_maps_to_error() {
        let mut state = StreamState::new();
        let data = r#"{"type":"message_delta","delta":{"stop_reason":"content_filter"},"usage":{"output_tokens":0}}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        process_sse_event(event, &mut state);
        assert!(matches!(state.stop_reason, StopReason::Error(ref s) if s == "content_filter"));
    }

    #[test]
    fn content_block_delta_out_of_range_ignored() {
        let mut state = StreamState::new();
        // index 5, but no blocks exist — should not panic
        let data = r#"{"type":"content_block_delta","index":5,"delta":{"type":"text_delta","text":"oops"}}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);
        assert!(events.is_empty());
    }

    #[test]
    fn content_block_stop_out_of_range_ignored() {
        let mut state = StreamState::new();
        // index 3, but no blocks — should not panic
        let data = r#"{"type":"content_block_stop","index":3}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);
        assert!(events.is_empty());
    }

    // -- Edge case: request building --

    #[test]
    fn build_request_empty_system_prompt_produces_no_system_blocks() {
        let model_meta = ModelMeta {
            id: "claude-sonnet-4-20250514".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = AnthropicProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };
        let options = RequestOptions {
            system_prompt: "".into(),
            ..Default::default()
        };
        let req = build_request(&model, Context::default(), options);
        assert!(req.system.is_empty());
    }

    #[test]
    fn build_request_empty_tools_produces_no_tools() {
        let model_meta = ModelMeta {
            id: "claude-sonnet-4-20250514".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = AnthropicProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };
        let options = RequestOptions {
            tools: vec![],
            ..Default::default()
        };
        let req = build_request(&model, Context::default(), options);
        assert!(req.tools.is_empty());
        // Verify it serializes without a "tools" key
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("tools").is_none());
    }

    #[test]
    fn cache_zero_recent_turns_adds_no_breakpoints() {
        let messages = vec![Message::user("first"), Message::user("second")];
        let cache = CacheOptions {
            cache_system_prompt: false,
            cache_tools: false,
            cache_recent_turns: 0,
            ..Default::default()
        };
        let api_msgs = build_messages(&messages, &cache);
        for msg in &api_msgs {
            for block in &msg.content {
                let json = serde_json::to_value(block).unwrap();
                assert!(json.get("cache_control").is_none());
            }
        }
    }

    // -- Effort level tests (41.3) --

    #[test]
    fn test_effort_level_serialization() {
        let model_meta = ModelMeta {
            id: "claude-sonnet-4-20250514".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = AnthropicProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };
        for (level, expected) in [
            (EffortLevel::Low, "low"),
            (EffortLevel::Medium, "medium"),
            (EffortLevel::High, "high"),
        ] {
            let options = RequestOptions {
                effort: Some(level),
                ..Default::default()
            };
            let req = build_request(&model, Context::default(), options);
            let json = serde_json::to_value(&req.output_config).unwrap();
            assert_eq!(json["effort"], expected);
        }
    }

    #[test]
    fn test_effort_none_omits_field() {
        let model_meta = ModelMeta {
            id: "claude-sonnet-4-20250514".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = AnthropicProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };
        let req = build_request(&model, Context::default(), RequestOptions::default());
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("output_config").is_none());
    }

    #[test]
    fn test_effort_adds_beta_header() {
        let model_meta = ModelMeta {
            id: "claude-sonnet-4-20250514".into(),
            provider: "anthropic".into(),
            name: "test".into(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let betas = beta_headers(&model_meta, Some(EffortLevel::Medium));
        assert!(betas.contains(&"effort-2025-11-24"));

        let betas_none = beta_headers(&model_meta, None);
        assert!(!betas_none.contains(&"effort-2025-11-24"));
    }

    // -- Retry logic tests (41.4) --

    #[test]
    fn test_retry_after_header_parsing() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("retry-after", "5".parse().unwrap());
        let delay = retry_after_delay(&headers).unwrap();
        assert_eq!(delay, std::time::Duration::from_secs(5));
    }

    #[test]
    fn test_retry_after_missing() {
        let headers = reqwest::header::HeaderMap::new();
        assert!(retry_after_delay(&headers).is_none());
    }

    #[test]
    fn test_context_overflow_parsing() {
        let body = r#"{"type":"error","error":{"type":"invalid_request_error","message":"input length and `max_tokens` exceed context limit: 188059 + 20000 > 200000"}}"#;
        let result = parse_context_overflow(body).unwrap();
        assert_eq!(result, (188059, 20000, 200000));
    }

    #[test]
    fn test_context_overflow_no_match() {
        assert!(parse_context_overflow("some other error").is_none());
    }

    // -- Tool sorting tests (41.7) --

    #[test]
    fn test_tool_defs_sorted_alphabetically() {
        let tools = vec![
            ToolDefinition {
                name: "write".into(),
                description: "Write".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
            ToolDefinition {
                name: "bash".into(),
                description: "Bash".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
            ToolDefinition {
                name: "read".into(),
                description: "Read".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
        ];
        let cache = CacheOptions::default();
        let api_tools = build_tool_defs(&tools, &cache);
        assert_eq!(api_tools[0].name, "bash");
        assert_eq!(api_tools[1].name, "read");
        assert_eq!(api_tools[2].name, "write");
    }

    #[test]
    fn test_tool_cache_breakpoint_on_last_sorted() {
        let tools = vec![
            ToolDefinition {
                name: "write".into(),
                description: "Write".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
            ToolDefinition {
                name: "bash".into(),
                description: "Bash".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
        ];
        let cache = CacheOptions {
            cache_tools: true,
            ..Default::default()
        };
        let api_tools = build_tool_defs(&tools, &cache);
        // After sorting: bash, write. Cache on write (last).
        assert_eq!(api_tools[0].name, "bash");
        assert!(api_tools[0].cache_control.is_none());
        assert_eq!(api_tools[1].name, "write");
        assert!(api_tools[1].cache_control.is_some());
    }

    // -- Cache TTL/scope tests (41.9) --

    #[test]
    fn test_cache_ttl_default() {
        let cache = CacheOptions::default();
        let cc = make_cache_control(&cache).unwrap();
        let json = serde_json::to_value(&cc).unwrap();
        assert_eq!(json["type"], "ephemeral");
        assert!(json.get("ttl").is_none());
        assert!(json.get("scope").is_none());
    }

    #[test]
    fn test_cache_ttl_extended() {
        let cache = CacheOptions {
            extended_ttl: true,
            ..Default::default()
        };
        let cc = make_cache_control(&cache).unwrap();
        let json = serde_json::to_value(&cc).unwrap();
        assert_eq!(json["type"], "ephemeral");
        assert_eq!(json["ttl"], "1h");
    }

    #[test]
    fn test_cache_ttl_global_scope() {
        let cache = CacheOptions {
            global_scope: true,
            ..Default::default()
        };
        let cc = make_cache_control(&cache).unwrap();
        let json = serde_json::to_value(&cc).unwrap();
        assert_eq!(json["scope"], "global");
    }

    #[test]
    fn test_cache_ttl_both() {
        let cache = CacheOptions {
            extended_ttl: true,
            global_scope: true,
            ..Default::default()
        };
        let cc = make_cache_control(&cache).unwrap();
        let json = serde_json::to_value(&cc).unwrap();
        assert_eq!(json["ttl"], "1h");
        assert_eq!(json["scope"], "global");
    }

    // -- max_tokens escalation constants (41.5) --

    #[test]
    fn test_max_tokens_escalation_constants() {
        assert_eq!(DEFAULT_MAX_TOKENS, 8_192);
        assert_eq!(ESCALATED_MAX_TOKENS, 64_000);
        assert!(ESCALATED_MAX_TOKENS > DEFAULT_MAX_TOKENS);
    }

    // -- Non-streaming fallback tests (41.6) --

    #[test]
    fn test_non_streaming_response_parsing() {
        let json = r#"{
            "model": "claude-sonnet-4-20250514",
            "content": [
                {"type": "text", "text": "Hello world"},
                {"type": "tool_use", "id": "t1", "name": "bash", "input": {"command": "ls"}}
            ],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 100, "output_tokens": 50, "cache_read_input_tokens": 10, "cache_creation_input_tokens": 5}
        }"#;
        let resp: ApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.model, "claude-sonnet-4-20250514");
        assert_eq!(resp.content.len(), 2);
        assert_eq!(resp.stop_reason, Some("tool_use".to_string()));
    }

    #[test]
    fn test_nonstreaming_response_to_events_conversion() {
        let resp = ApiResponse {
            model: "claude-sonnet-4-20250514".into(),
            content: vec![
                ApiResponseBlock::Text { text: "Hi".into() },
                ApiResponseBlock::ToolUse {
                    id: "t1".into(),
                    name: "read".into(),
                    input: serde_json::json!({"path": "foo.rs"}),
                },
            ],
            stop_reason: Some("tool_use".into()),
            usage: SseUsage {
                input_tokens: 100,
                output_tokens: 50,
                cache_read_input_tokens: 0,
                cache_creation_input_tokens: 0,
            },
        };
        let events = non_streaming_response_to_events(resp);
        // MessageStart, TextDelta, ToolCall, MessageEnd
        assert_eq!(events.len(), 4);
        assert!(
            matches!(&events[0], StreamEvent::MessageStart { model } if model == "claude-sonnet-4-20250514")
        );
        assert!(matches!(&events[1], StreamEvent::TextDelta { text } if text == "Hi"));
        assert!(matches!(&events[2], StreamEvent::ToolCall { name, .. } if name == "read"));
        assert!(
            matches!(&events[3], StreamEvent::MessageEnd { message } if message.stop_reason == StopReason::ToolUse)
        );
    }
}
