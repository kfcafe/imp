use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use futures_core::Stream;
use serde::{Deserialize, Serialize};

use tokio_tungstenite::{connect_async, tungstenite::Message as WebSocketMessage};
use tungstenite::client::IntoClientRequest;

use crate::auth::{ApiKey, AuthStore};
use crate::error::{Error, Result};
use crate::message::{AssistantMessage, ContentBlock, Message, StopReason};
use crate::model::{Model, ModelMeta};
use crate::provider::{
    CancellationMode, Context, ContinuationMode, PersistentSessionMode, Provider, RequestOptions,
    ResumabilityMode, ThinkingLevel, ToolDefinition, TransportCapabilities,
};
use crate::stream::StreamEvent;
use crate::usage::Usage;

const API_URL: &str = "https://api.openai.com/v1/responses";
const WS_URL: &str = "wss://api.openai.com/v1/responses";
const PERSISTENT_TRANSPORT_ENV: &str = "IMP_OPENAI_PERSISTENT_TRANSPORT";

// ---------------------------------------------------------------------------
// OpenAI Responses API wire-format types (request)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    input: Vec<serde_json::Value>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ApiToolDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<ApiReasoning>,
}

#[derive(Debug, Serialize)]
struct ApiToolDef {
    #[serde(rename = "type")]
    tool_type: String,
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ApiReasoning {
    effort: String,
}

// ---------------------------------------------------------------------------
// OpenAI Responses API wire-format types (SSE response)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct SseEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    response: Option<SseResponse>,
    #[serde(default)]
    item: Option<SseOutputItem>,
    #[serde(default)]
    delta: Option<String>,
    #[serde(default)]
    output_index: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct SseResponse {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    usage: Option<SseUsage>,
    #[serde(default)]
    error: Option<SseResponseError>,
    #[serde(default)]
    incomplete_details: Option<SseIncompleteDetails>,
}

#[derive(Debug, Clone, Deserialize)]
struct SseResponseError {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SseIncompleteDetails {
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SseOutputItem {
    #[serde(rename = "type")]
    item_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    call_id: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SseUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
    #[serde(default)]
    input_tokens_details: Option<SseInputTokenDetails>,
}

#[derive(Debug, Clone, Deserialize)]
struct SseInputTokenDetails {
    #[serde(default)]
    cached_tokens: u32,
}

// ---------------------------------------------------------------------------
// SSE stream state
// ---------------------------------------------------------------------------

#[derive(Debug)]
#[allow(dead_code)]
enum OutputItemState {
    Message,
    FunctionCall {
        name: String,
        call_id: String,
        args_buf: String,
    },
}

struct StreamState {
    model: String,
    items: Vec<OutputItemState>,
    content: Vec<ContentBlock>,
    usage: Usage,
    stop_reason: StopReason,
    finished: bool,
}

impl StreamState {
    fn new() -> Self {
        Self {
            model: String::new(),
            items: Vec::new(),
            content: Vec::new(),
            usage: Usage::default(),
            stop_reason: StopReason::EndTurn,
            finished: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// OpenAI Responses API provider with streaming SSE support.
pub struct OpenAiProvider {
    client: reqwest::Client,
    models: Vec<ModelMeta>,
}

impl Default for OpenAiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenAiProvider {
    pub fn new() -> Self {
        Self {
            client: super::streaming_http_client(),
            models: builtin_models(),
        }
    }

    fn persistent_transport_enabled() -> bool {
        Self::persistent_transport_enabled_value(
            std::env::var(PERSISTENT_TRANSPORT_ENV).ok().as_deref(),
        )
    }

    fn persistent_transport_enabled_value(value: Option<&str>) -> bool {
        value
            .map(|value| matches!(value, "1" | "true" | "TRUE" | "yes" | "on"))
            .unwrap_or(false)
    }

    pub(crate) fn persistent_transport_capabilities() -> TransportCapabilities {
        TransportCapabilities {
            request_response: true,
            streaming: true,
            continuation: ContinuationMode::ProviderManagedId,
            persistent_session: PersistentSessionMode::WebSocket,
            cancellation: CancellationMode::DropLocalStream,
            resumability: ResumabilityMode::ResumeProviderState,
        }
    }

    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

// ---------------------------------------------------------------------------
// Request building
// ---------------------------------------------------------------------------

fn reasoning_effort(level: ThinkingLevel) -> Option<String> {
    match level {
        ThinkingLevel::Off => None,
        ThinkingLevel::Minimal | ThinkingLevel::Low => Some("low".into()),
        ThinkingLevel::Medium => Some("medium".into()),
        ThinkingLevel::High | ThinkingLevel::XHigh => Some("high".into()),
    }
}

fn default_max_output_tokens(model: &Model) -> u32 {
    model.meta.max_output_tokens.min(8_192)
}

fn build_request(model: &Model, context: Context, options: RequestOptions) -> ApiRequest {
    let instructions = if options.system_prompt.is_empty() {
        None
    } else {
        Some(options.system_prompt.clone())
    };

    let tools = build_tool_defs(&options.tools);
    let input = convert_messages(&context.messages);

    // Only include reasoning for models with reasoning capability
    let reasoning = if model.meta.capabilities.reasoning {
        reasoning_effort(options.thinking_level).map(|effort| ApiReasoning { effort })
    } else {
        None
    };

    // Temperature must not be set when reasoning is active
    let temperature = if reasoning.is_some() {
        None
    } else {
        options.temperature
    };

    let max_output_tokens = options
        .max_tokens
        .or(Some(default_max_output_tokens(model)));

    ApiRequest {
        model: model.meta.id.clone(),
        input,
        stream: true,
        instructions,
        tools,
        temperature,
        max_output_tokens,
        reasoning,
    }
}

fn build_tool_defs(tools: &[ToolDefinition]) -> Vec<ApiToolDef> {
    let mut sorted: Vec<&ToolDefinition> = tools.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    sorted
        .iter()
        .map(|t| ApiToolDef {
            tool_type: "function".into(),
            name: t.name.clone(),
            description: t.description.clone(),
            parameters: t.parameters.clone(),
        })
        .collect()
}

/// Convert internal messages to OpenAI Responses API input items.
///
/// Handles the image workaround: OpenAI cannot accept images in tool results,
/// so images are replaced with a placeholder and injected as a subsequent user
/// message.
fn convert_messages(messages: &[Message]) -> Vec<serde_json::Value> {
    let mut items = Vec::new();

    for msg in messages {
        match msg {
            Message::User(u) => {
                let has_images = u
                    .content
                    .iter()
                    .any(|b| matches!(b, ContentBlock::Image { .. }));

                if has_images {
                    let parts: Vec<serde_json::Value> = u
                        .content
                        .iter()
                        .filter_map(|block| match block {
                            ContentBlock::Text { text } => Some(serde_json::json!({
                                "type": "input_text",
                                "text": text
                            })),
                            ContentBlock::Image { media_type, data } => Some(serde_json::json!({
                                "type": "input_image",
                                "image_url": format!("data:{media_type};base64,{data}")
                            })),
                            _ => None,
                        })
                        .collect();
                    items.push(serde_json::json!({
                        "role": "user",
                        "content": parts
                    }));
                } else {
                    let text: String = u
                        .content
                        .iter()
                        .filter_map(|b| match b {
                            ContentBlock::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    items.push(serde_json::json!({
                        "role": "user",
                        "content": text
                    }));
                }
            }
            Message::Assistant(a) => {
                // Text blocks → message item
                let text_parts: Vec<serde_json::Value> = a
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text { text } => Some(serde_json::json!({
                            "type": "output_text",
                            "text": text
                        })),
                        _ => None,
                    })
                    .collect();

                if !text_parts.is_empty() {
                    items.push(serde_json::json!({
                        "type": "message",
                        "role": "assistant",
                        "content": text_parts
                    }));
                }

                // Tool calls → individual function_call items
                for block in &a.content {
                    if let ContentBlock::ToolCall {
                        id,
                        name,
                        arguments,
                    } = block
                    {
                        items.push(serde_json::json!({
                            "type": "function_call",
                            "call_id": id,
                            "name": name,
                            "arguments": arguments.to_string()
                        }));
                    }
                }
            }
            Message::ToolResult(tr) => {
                let mut output_parts = Vec::new();
                let mut images_to_inject = Vec::new();

                for block in &tr.content {
                    match block {
                        ContentBlock::Text { text } => {
                            output_parts.push(text.clone());
                        }
                        ContentBlock::Image { media_type, data } => {
                            output_parts.push("[Image attached below]".to_string());
                            images_to_inject.push((media_type.clone(), data.clone()));
                        }
                        _ => {}
                    }
                }

                let output = output_parts.join("\n");
                items.push(serde_json::json!({
                    "type": "function_call_output",
                    "call_id": tr.tool_call_id,
                    "output": output
                }));

                // Image workaround: inject user message with images after the tool result
                if !images_to_inject.is_empty() {
                    let image_parts: Vec<serde_json::Value> = images_to_inject
                        .iter()
                        .map(|(mime, data)| {
                            serde_json::json!({
                                "type": "input_image",
                                "image_url": format!("data:{mime};base64,{data}")
                            })
                        })
                        .collect();
                    items.push(serde_json::json!({
                        "role": "user",
                        "content": image_parts
                    }));
                }
            }
        }
    }

    items
}

// ---------------------------------------------------------------------------
// SSE parsing
// ---------------------------------------------------------------------------

fn parse_sse_event(data: &str) -> Result<Option<SseEvent>> {
    let trimmed = data.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    serde_json::from_str(trimmed)
        .map(Some)
        .map_err(|e| Error::Stream(format!("Failed to parse SSE data: {e}: {trimmed}")))
}

fn push_text_block(content: &mut Vec<ContentBlock>, text: String) {
    if text.is_empty() {
        return;
    }

    if let Some(ContentBlock::Text { text: existing }) = content.last_mut() {
        existing.push_str(&text);
    } else {
        content.push(ContentBlock::Text { text });
    }
}

fn push_thinking_block(content: &mut Vec<ContentBlock>, text: String) {
    if text.is_empty() {
        return;
    }

    if let Some(ContentBlock::Thinking { text: existing }) = content.last_mut() {
        existing.push_str(&text);
    } else {
        content.push(ContentBlock::Thinking { text });
    }
}

fn format_openai_response_error(resp: &SseResponse) -> String {
    match resp.error.as_ref() {
        Some(error) => match (error.code.as_deref(), error.message.as_deref()) {
            (Some(code), Some(message)) if !message.is_empty() => format!("{code}: {message}"),
            (Some(code), _) => code.to_string(),
            (_, Some(message)) if !message.is_empty() => message.to_string(),
            _ => "response.failed".to_string(),
        },
        None => "response.failed".to_string(),
    }
}

fn process_sse_event(event: SseEvent, state: &mut StreamState) -> Vec<StreamEvent> {
    process_openai_stream_event(event, state)
}

fn process_openai_stream_event(event: SseEvent, state: &mut StreamState) -> Vec<StreamEvent> {
    let mut out = Vec::new();

    match event.event_type.as_str() {
        "response.created" => {
            if let Some(resp) = event.response {
                if let Some(model) = resp.model {
                    state.model.clone_from(&model);
                    out.push(StreamEvent::MessageStart { model });
                }
            }
        }
        "response.output_item.added" => {
            if let Some(item) = event.item {
                let idx = event.output_index.unwrap_or(0);
                let item_state = match item.item_type.as_str() {
                    "function_call" => OutputItemState::FunctionCall {
                        name: item.name.unwrap_or_default(),
                        call_id: item.call_id.unwrap_or_default(),
                        args_buf: String::new(),
                    },
                    _ => OutputItemState::Message,
                };
                while state.items.len() <= idx {
                    state.items.push(OutputItemState::Message);
                }
                state.items[idx] = item_state;
            }
        }
        "response.content_part.delta" | "response.output_text.delta" => {
            if let Some(delta) = event.delta {
                push_text_block(&mut state.content, delta.clone());
                out.push(StreamEvent::TextDelta { text: delta });
            }
        }
        "response.reasoning_text.delta" => {
            if let Some(delta) = event.delta {
                push_thinking_block(&mut state.content, delta.clone());
                out.push(StreamEvent::ThinkingDelta { text: delta });
            }
        }
        "response.function_call_arguments.delta" => {
            if let Some(delta) = event.delta {
                let idx = event.output_index.unwrap_or(0);
                if idx < state.items.len() {
                    if let OutputItemState::FunctionCall {
                        ref mut args_buf, ..
                    } = state.items[idx]
                    {
                        args_buf.push_str(&delta);
                    }
                }
            }
        }
        "response.output_item.done" => {
            if let Some(item) = event.item {
                if item.item_type == "function_call" {
                    let name = item.name.unwrap_or_default();
                    let call_id = item.call_id.unwrap_or_default();
                    let args_str = item.arguments.unwrap_or_else(|| "{}".to_string());
                    let arguments: serde_json::Value = serde_json::from_str(&args_str)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                    state.content.push(ContentBlock::ToolCall {
                        id: call_id.clone(),
                        name: name.clone(),
                        arguments: arguments.clone(),
                    });
                    out.push(StreamEvent::ToolCall {
                        id: call_id,
                        name,
                        arguments,
                    });
                }
            }
        }
        "response.completed" | "response.incomplete" | "response.failed" => {
            state.finished = true;
            if let Some(resp) = event.response {
                if let Some(ref u) = resp.usage {
                    state.usage.input_tokens = u.input_tokens;
                    state.usage.output_tokens = u.output_tokens;
                    if let Some(details) = &u.input_tokens_details {
                        state.usage.cache_read_tokens = details.cached_tokens;
                    }
                }

                state.stop_reason = match event.event_type.as_str() {
                    "response.failed" => StopReason::Error(format_openai_response_error(&resp)),
                    "response.incomplete" => match resp
                        .incomplete_details
                        .as_ref()
                        .and_then(|details| details.reason.as_deref())
                    {
                        Some("max_output_tokens") | None => StopReason::MaxTokens,
                        Some(reason) => StopReason::Error(reason.to_string()),
                    },
                    _ => match resp.status.as_deref() {
                        Some("completed") => {
                            if state
                                .content
                                .iter()
                                .any(|c| matches!(c, ContentBlock::ToolCall { .. }))
                            {
                                StopReason::ToolUse
                            } else {
                                StopReason::EndTurn
                            }
                        }
                        Some("incomplete") => StopReason::MaxTokens,
                        Some(other) => StopReason::Error(other.to_string()),
                        None => StopReason::EndTurn,
                    },
                };
            }

            let message = AssistantMessage {
                content: std::mem::take(&mut state.content),
                usage: Some(state.usage.clone()),
                stop_reason: state.stop_reason.clone(),
                timestamp: crate::now(),
            };
            out.push(StreamEvent::MessageEnd { message });
        }
        _ => {
            // Ignore other event types (response.in_progress, content_part.added, etc.)
        }
    }

    out
}

#[cfg(test)]
#[allow(dead_code)]
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
// Streaming implementation
// ---------------------------------------------------------------------------

pub(crate) fn build_request_json(
    model: &Model,
    context: Context,
    options: RequestOptions,
) -> serde_json::Value {
    serde_json::to_value(build_request(model, context, options))
        .expect("OpenAI request should always serialize")
}

pub(crate) fn stream_response_json(
    client: reqwest::Client,
    url: String,
    headers: Vec<(String, String)>,
    request: serde_json::Value,
) -> Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>> {
    let (tx, rx) = futures::channel::mpsc::unbounded();

    tokio::spawn(async move {
        let mut builder = client.post(&url);
        for (name, value) in headers {
            builder = builder.header(&name, value);
        }

        let result = builder.json(&request).send().await;

        let resp = match result {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.unbounded_send(Err(Error::Http(e)));
                return;
            }
        };

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            let _ = tx.unbounded_send(Err(Error::Provider(format!("HTTP {status}: {body}"))));
            return;
        }

        let mut state = StreamState::new();
        let mut buf = String::new();
        let mut byte_stream = resp.bytes_stream();

        use futures::StreamExt;
        while let Some(chunk) = byte_stream.next().await {
            match chunk {
                Ok(bytes) => {
                    buf.push_str(&String::from_utf8_lossy(&bytes));

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

        let trimmed = buf.trim();
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
                    let _ = tx.unbounded_send(Err(e));
                    return;
                }
            }
        }

        if !state.finished {
            let _ = tx.unbounded_send(Err(Error::Stream(
                "OpenAI stream ended before response.completed".into(),
            )));
        }
    });

    Box::pin(rx)
}

fn stream_response(
    client: reqwest::Client,
    api_key: String,
    request: ApiRequest,
) -> Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>> {
    stream_response_json(
        client,
        API_URL.to_string(),
        vec![
            ("authorization".to_string(), format!("Bearer {api_key}")),
            ("content-type".to_string(), "application/json".to_string()),
        ],
        serde_json::to_value(request).expect("OpenAI request should always serialize"),
    )
}

fn stream_response_websocket(
    api_key: String,
    request: ApiRequest,
) -> Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>> {
    let (tx, rx) = futures::channel::mpsc::unbounded();

    tokio::spawn(async move {
        let mut ws_request = match WS_URL.into_client_request() {
            Ok(request) => request,
            Err(error) => {
                let _ = tx.unbounded_send(Err(Error::Provider(format!(
                    "failed to build OpenAI websocket request: {error}"
                ))));
                return;
            }
        };

        let headers = ws_request.headers_mut();
        let auth_value = match format!("Bearer {api_key}").parse() {
            Ok(value) => value,
            Err(error) => {
                let _ = tx.unbounded_send(Err(Error::Provider(format!(
                    "failed to build OpenAI websocket auth header: {error}"
                ))));
                return;
            }
        };
        headers.insert("authorization", auth_value);

        let (mut socket, _) = match connect_async(ws_request).await {
            Ok(connection) => connection,
            Err(error) => {
                let _ = tx.unbounded_send(Err(Error::Provider(format!(
                    "OpenAI websocket connection failed before streaming; stateless fallback is available by unsetting {PERSISTENT_TRANSPORT_ENV}: {error}"
                ))));
                return;
            }
        };

        let mut payload = match serde_json::to_value(request) {
            Ok(value) => value,
            Err(error) => {
                let _ = tx.unbounded_send(Err(Error::Serialization(error)));
                return;
            }
        };
        if let serde_json::Value::Object(ref mut map) = payload {
            map.remove("stream");
            map.insert("store".to_string(), serde_json::Value::Bool(false));
            map.insert(
                "type".to_string(),
                serde_json::Value::String("response.create".to_string()),
            );
        }

        if let Err(error) = socket
            .send(WebSocketMessage::Text(payload.to_string().into()))
            .await
        {
            let _ = tx.unbounded_send(Err(Error::Provider(format!(
                "OpenAI websocket send failed before streaming; stateless fallback is available by unsetting {PERSISTENT_TRANSPORT_ENV}: {error}"
            ))));
            return;
        }

        let mut state = StreamState::new();
        while let Some(message) = socket.next().await {
            let message = match message {
                Ok(message) => message,
                Err(error) => {
                    let _ = tx.unbounded_send(Err(Error::Stream(format!(
                        "OpenAI websocket stream error: {error}"
                    ))));
                    return;
                }
            };

            let text = match message {
                WebSocketMessage::Text(text) => text,
                WebSocketMessage::Binary(bytes) => match String::from_utf8(bytes.to_vec()) {
                    Ok(text) => text.into(),
                    Err(error) => {
                        let _ = tx.unbounded_send(Err(Error::Stream(format!(
                            "OpenAI websocket sent non-UTF8 binary event: {error}"
                        ))));
                        return;
                    }
                },
                WebSocketMessage::Ping(bytes) => {
                    let _ = socket.send(WebSocketMessage::Pong(bytes)).await;
                    continue;
                }
                WebSocketMessage::Pong(_) => continue,
                WebSocketMessage::Close(_) => break,
                WebSocketMessage::Frame(_) => continue,
            };

            let event: SseEvent = match serde_json::from_str(&text) {
                Ok(event) => event,
                Err(error) => {
                    let _ = tx.unbounded_send(Err(Error::Stream(format!(
                        "Failed to parse OpenAI websocket event: {error}"
                    ))));
                    return;
                }
            };

            if event.event_type == "error" {
                let _ = tx.unbounded_send(Err(Error::Provider(
                    "OpenAI websocket returned an error event".to_string(),
                )));
                return;
            }

            for stream_event in process_openai_stream_event(event, &mut state) {
                if tx.unbounded_send(Ok(stream_event)).is_err() {
                    return;
                }
            }

            if state.finished {
                let _ = socket.close(None).await;
                return;
            }
        }

        if !state.finished {
            let _ = tx.unbounded_send(Err(Error::Stream(
                "OpenAI websocket ended before response.completed".into(),
            )));
        }
    });

    Box::pin(rx)
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn stream(
        &self,
        model: &Model,
        context: Context,
        options: RequestOptions,
        api_key: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>> {
        let request = build_request(model, context, options);
        let api_key = api_key.to_string();
        if Self::persistent_transport_enabled() {
            stream_response_websocket(api_key, request)
        } else {
            let client = self.client.clone();
            stream_response(client, api_key, request)
        }
    }

    async fn resolve_auth(&self, auth: &AuthStore) -> Result<ApiKey> {
        auth.resolve("openai")
    }

    fn id(&self) -> &str {
        "openai"
    }

    fn models(&self) -> &[ModelMeta] {
        &self.models
    }

    fn transport_capabilities(&self) -> TransportCapabilities {
        if Self::persistent_transport_enabled() {
            Self::persistent_transport_capabilities()
        } else {
            TransportCapabilities::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Built-in models
// ---------------------------------------------------------------------------

fn builtin_models() -> Vec<ModelMeta> {
    crate::model::builtin_openai_models()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{ToolResultMessage, UserMessage};
    use crate::model::{Capabilities, ModelPricing};

    #[test]
    fn openai_tool_defs_are_sorted_for_prompt_cache_stability() {
        let write = ToolDefinition {
            name: "write".into(),
            description: "Write".into(),
            parameters: serde_json::json!({ "type": "object" }),
        };
        let bash = ToolDefinition {
            name: "bash".into(),
            description: "Bash".into(),
            parameters: serde_json::json!({ "type": "object" }),
        };
        let read = ToolDefinition {
            name: "read".into(),
            description: "Read".into(),
            parameters: serde_json::json!({ "type": "object" }),
        };

        let names = build_tool_defs(&[write.clone(), bash.clone(), read.clone()])
            .into_iter()
            .map(|tool| tool.name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["bash", "read", "write"]);

        let first_request = ApiRequest {
            model: "gpt-test".into(),
            input: vec![serde_json::json!({ "role": "user", "content": "hello" })],
            stream: true,
            instructions: Some("system".into()),
            tools: build_tool_defs(&[write.clone(), bash.clone(), read.clone()]),
            temperature: None,
            max_output_tokens: None,
            reasoning: None,
        };
        let second_request = ApiRequest {
            model: "gpt-test".into(),
            input: vec![serde_json::json!({ "role": "user", "content": "hello" })],
            stream: true,
            instructions: Some("system".into()),
            tools: build_tool_defs(&[read, write, bash]),
            temperature: None,
            max_output_tokens: None,
            reasoning: None,
        };

        assert_eq!(
            serde_json::to_value(first_request).unwrap(),
            serde_json::to_value(second_request).unwrap()
        );
    }

    #[test]
    fn openai_serialize_text_user_message() {
        let messages = vec![Message::user("Hello, world!")];
        let items = convert_messages(&messages);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["role"], "user");
        assert_eq!(items[0]["content"], "Hello, world!");
    }

    #[test]
    fn openai_serialize_user_message_with_image() {
        let messages = vec![Message::User(UserMessage {
            content: vec![
                ContentBlock::Text {
                    text: "What's in this image?".into(),
                },
                ContentBlock::Image {
                    media_type: "image/png".into(),
                    data: "iVBOR".into(),
                },
            ],
            timestamp: 0,
        })];
        let items = convert_messages(&messages);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["role"], "user");
        let content = items[0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "input_text");
        assert_eq!(content[0]["text"], "What's in this image?");
        assert_eq!(content[1]["type"], "input_image");
        assert_eq!(content[1]["image_url"], "data:image/png;base64,iVBOR");
    }

    #[test]
    fn openai_serialize_assistant_text_message() {
        let messages = vec![Message::Assistant(AssistantMessage {
            content: vec![ContentBlock::Text {
                text: "Hello!".into(),
            }],
            usage: None,
            stop_reason: StopReason::EndTurn,
            timestamp: 0,
        })];
        let items = convert_messages(&messages);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["type"], "message");
        assert_eq!(items[0]["role"], "assistant");
        let content = items[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "output_text");
        assert_eq!(content[0]["text"], "Hello!");
    }

    #[test]
    fn openai_serialize_assistant_with_tool_call() {
        let messages = vec![Message::Assistant(AssistantMessage {
            content: vec![
                ContentBlock::Text {
                    text: "Let me check.".into(),
                },
                ContentBlock::ToolCall {
                    id: "call_1".into(),
                    name: "bash".into(),
                    arguments: serde_json::json!({"command": "ls"}),
                },
            ],
            usage: None,
            stop_reason: StopReason::ToolUse,
            timestamp: 0,
        })];
        let items = convert_messages(&messages);
        // Text → message item, tool call → function_call item
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["type"], "message");
        assert_eq!(items[0]["role"], "assistant");
        assert_eq!(items[1]["type"], "function_call");
        assert_eq!(items[1]["call_id"], "call_1");
        assert_eq!(items[1]["name"], "bash");
        assert_eq!(items[1]["arguments"], "{\"command\":\"ls\"}");
    }

    #[test]
    fn openai_serialize_tool_result() {
        let messages = vec![Message::ToolResult(ToolResultMessage {
            tool_call_id: "call_1".into(),
            tool_name: "bash".into(),
            content: vec![ContentBlock::Text {
                text: "README.md\nsrc/".into(),
            }],
            is_error: false,
            details: serde_json::Value::Null,
            timestamp: 0,
        })];
        let items = convert_messages(&messages);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["type"], "function_call_output");
        assert_eq!(items[0]["call_id"], "call_1");
        assert_eq!(items[0]["output"], "README.md\nsrc/");
    }

    #[test]
    fn openai_image_workaround_tool_result_with_image() {
        let messages = vec![Message::ToolResult(ToolResultMessage {
            tool_call_id: "call_screenshot".into(),
            tool_name: "screenshot".into(),
            content: vec![
                ContentBlock::Text {
                    text: "Screenshot taken".into(),
                },
                ContentBlock::Image {
                    media_type: "image/png".into(),
                    data: "iVBOR_screenshot".into(),
                },
            ],
            is_error: false,
            details: serde_json::Value::Null,
            timestamp: 0,
        })];
        let items = convert_messages(&messages);

        // Should produce 2 items: function_call_output + user message with image
        assert_eq!(items.len(), 2);

        // First: function_call_output with placeholder
        assert_eq!(items[0]["type"], "function_call_output");
    }

    // -- SSE parsing tests --

    #[test]
    fn openai_parse_text_delta() {
        let data = r#"{"type":"response.content_part.delta","delta":"Hello world"}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        let mut state = StreamState::new();
        let events = process_sse_event(event, &mut state);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StreamEvent::TextDelta { text } if text == "Hello world"));
        assert!(matches!(
            state.content.as_slice(),
            [ContentBlock::Text { text }] if text == "Hello world"
        ));
    }

    #[test]
    fn openai_parse_output_text_delta_builds_message_content() {
        let mut state = StreamState::new();

        for data in [
            r#"{"type":"response.output_text.delta","delta":"Hello"}"#,
            r#"{"type":"response.output_text.delta","delta":" world"}"#,
        ] {
            let event = parse_sse_event(data).unwrap().unwrap();
            let events = process_sse_event(event, &mut state);
            assert_eq!(events.len(), 1);
            assert!(matches!(events[0], StreamEvent::TextDelta { .. }));
        }

        let completed = r#"{"type":"response.completed","response":{"status":"completed","usage":{"input_tokens":10,"output_tokens":2}}}"#;
        let event = parse_sse_event(completed).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);

        assert_eq!(events.len(), 1);
        if let StreamEvent::MessageEnd { message } = &events[0] {
            assert!(matches!(
                message.content.as_slice(),
                [ContentBlock::Text { text }] if text == "Hello world"
            ));
            let usage = message.usage.as_ref().unwrap();
            assert_eq!(usage.input_tokens, 10);
            assert_eq!(usage.output_tokens, 2);
        } else {
            panic!("expected MessageEnd");
        }
    }

    #[test]
    fn openai_parse_reasoning_text_delta() {
        let data = r#"{"type":"response.reasoning_text.delta","delta":"Planning"}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        let mut state = StreamState::new();
        let events = process_sse_event(event, &mut state);

        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StreamEvent::ThinkingDelta { text } if text == "Planning"));
        assert!(matches!(
            state.content.as_slice(),
            [ContentBlock::Thinking { text }] if text == "Planning"
        ));
    }

    #[test]
    fn openai_parse_function_call_accumulation() {
        let mut state = StreamState::new();

        // output_item.added for a function_call
        let added = r#"{"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","name":"bash","call_id":"call_42"}}"#;
        let event = parse_sse_event(added).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);
        assert!(events.is_empty());

        // argument deltas
        let d1 = r#"{"type":"response.function_call_arguments.delta","output_index":0,"delta":"{\"com"}"#;
        let event = parse_sse_event(d1).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);
        assert!(events.is_empty());

        let d2 = r#"{"type":"response.function_call_arguments.delta","output_index":0,"delta":"mand\":\"ls\"}"}"#;
        let event = parse_sse_event(d2).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);
        assert!(events.is_empty());

        // Verify the args buffer accumulated correctly
        if let OutputItemState::FunctionCall { args_buf, .. } = &state.items[0] {
            assert_eq!(args_buf, r#"{"command":"ls"}"#);
        } else {
            panic!("expected FunctionCall state");
        }
    }

    #[test]
    fn openai_parse_response_completed() {
        let mut state = StreamState::new();
        state.model = "gpt-4o".into();

        let data = r#"{"type":"response.completed","response":{"model":"gpt-4o","status":"completed","usage":{"input_tokens":50,"output_tokens":25,"input_tokens_details":{"cached_tokens":10}}}}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);

        assert_eq!(events.len(), 1);
        if let StreamEvent::MessageEnd { message } = &events[0] {
            assert_eq!(message.stop_reason, StopReason::EndTurn);
            let usage = message.usage.as_ref().unwrap();
            assert_eq!(usage.input_tokens, 50);
            assert_eq!(usage.output_tokens, 25);
            assert_eq!(usage.cache_read_tokens, 10);
        } else {
            panic!("expected MessageEnd");
        }
    }

    #[test]
    fn openai_response_incomplete_maps_to_max_tokens() {
        let mut state = StreamState::new();
        let data = r#"{"type":"response.completed","response":{"status":"incomplete","usage":{"input_tokens":0,"output_tokens":0}}}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);

        assert_eq!(events.len(), 1);
        if let StreamEvent::MessageEnd { message } = &events[0] {
            assert_eq!(message.stop_reason, StopReason::MaxTokens);
        } else {
            panic!("expected MessageEnd");
        }
    }

    #[test]
    fn openai_response_incomplete_event_maps_to_max_tokens_and_finishes() {
        let mut state = StreamState::new();
        let data = r#"{"type":"response.incomplete","response":{"status":"incomplete","incomplete_details":{"reason":"max_output_tokens"},"usage":{"input_tokens":7,"output_tokens":11}}}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);

        assert!(state.finished);
        assert_eq!(events.len(), 1);
        if let StreamEvent::MessageEnd { message } = &events[0] {
            assert_eq!(message.stop_reason, StopReason::MaxTokens);
            let usage = message.usage.as_ref().unwrap();
            assert_eq!(usage.input_tokens, 7);
            assert_eq!(usage.output_tokens, 11);
        } else {
            panic!("expected MessageEnd");
        }
    }

    #[test]
    fn openai_response_failed_event_finishes_with_error_reason() {
        let mut state = StreamState::new();
        let data = r#"{"type":"response.failed","response":{"status":"failed","error":{"code":"server_error","message":"upstream disconnected"}}}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        let events = process_sse_event(event, &mut state);

        assert!(state.finished);
        assert_eq!(events.len(), 1);
        if let StreamEvent::MessageEnd { message } = &events[0] {
            assert_eq!(
                message.stop_reason,
                StopReason::Error("server_error: upstream disconnected".into())
            );
        } else {
            panic!("expected MessageEnd");
        }
    }

    #[test]
    fn openai_reasoning_effort_off_returns_none() {
        assert!(reasoning_effort(ThinkingLevel::Off).is_none());
    }

    #[test]
    fn openai_reasoning_effort_levels() {
        assert_eq!(
            reasoning_effort(ThinkingLevel::Minimal).as_deref(),
            Some("low")
        );
        assert_eq!(reasoning_effort(ThinkingLevel::Low).as_deref(), Some("low"));
        assert_eq!(
            reasoning_effort(ThinkingLevel::Medium).as_deref(),
            Some("medium")
        );
        assert_eq!(
            reasoning_effort(ThinkingLevel::High).as_deref(),
            Some("high")
        );
        assert_eq!(
            reasoning_effort(ThinkingLevel::XHigh).as_deref(),
            Some("high")
        );
    }

    #[test]
    fn openai_empty_instructions_omitted() {
        let model_meta = ModelMeta {
            id: "gpt-4o".into(),
            provider: "openai".into(),
            name: "GPT-4o".into(),
            context_window: 128_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = OpenAiProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };
        let options = RequestOptions {
            system_prompt: "".into(),
            ..Default::default()
        };
        let req = build_request(&model, Context::default(), options);
        assert!(req.instructions.is_none());
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("instructions").is_none());
    }

    #[test]
    fn openai_default_max_output_tokens_are_capped() {
        let model_meta = ModelMeta {
            id: "gpt-5.4".into(),
            provider: "openai".into(),
            name: "GPT-5.4".into(),
            context_window: 400_000,
            max_output_tokens: 32_768,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = OpenAiProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };

        let req = build_request(&model, Context::default(), RequestOptions::default());
        assert_eq!(req.max_output_tokens, Some(8_192));
    }

    #[test]
    fn openai_explicit_max_output_tokens_override_cap() {
        let model_meta = ModelMeta {
            id: "gpt-5.4".into(),
            provider: "openai".into(),
            name: "GPT-5.4".into(),
            context_window: 400_000,
            max_output_tokens: 32_768,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider = OpenAiProvider::new();
        let model = Model {
            meta: model_meta,
            provider: Arc::new(provider),
        };

        let req = build_request(
            &model,
            Context::default(),
            RequestOptions {
                max_tokens: Some(12_000),
                ..Default::default()
            },
        );
        assert_eq!(req.max_output_tokens, Some(12_000));
    }

    #[test]
    fn openai_parse_sse_event_malformed_json_returns_error() {
        let result = parse_sse_event("{garbage}");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Stream(_)));
    }

    #[test]
    fn openai_parse_sse_event_empty_returns_none() {
        let result = parse_sse_event("").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn openai_unknown_event_type_ignored() {
        let data = r#"{"type":"response.in_progress"}"#;
        let event = parse_sse_event(data).unwrap().unwrap();
        let mut state = StreamState::new();
        let events = process_sse_event(event, &mut state);
        assert!(events.is_empty());
    }
    #[test]
    fn openai_transport_capabilities_are_stateless_by_default() {
        assert!(!OpenAiProvider::persistent_transport_enabled_value(None));
        assert!(!OpenAiProvider::persistent_transport_enabled_value(Some(
            "0"
        )));

        let capabilities = TransportCapabilities::default();

        assert_eq!(capabilities, TransportCapabilities::default());
        assert_eq!(capabilities.persistent_session, PersistentSessionMode::None);
        assert_eq!(capabilities.continuation, ContinuationMode::None);
        assert_eq!(capabilities.resumability, ResumabilityMode::RestartRequest);
    }

    #[test]
    fn openai_transport_capabilities_are_persistent_only_when_enabled() {
        for value in ["1", "true", "TRUE", "yes", "on"] {
            assert!(OpenAiProvider::persistent_transport_enabled_value(Some(
                value
            )));
        }

        let capabilities = OpenAiProvider::persistent_transport_capabilities();

        assert_eq!(
            capabilities.persistent_session,
            PersistentSessionMode::WebSocket
        );
        assert_eq!(
            capabilities.continuation,
            ContinuationMode::ProviderManagedId
        );
        assert_eq!(
            capabilities.resumability,
            ResumabilityMode::ResumeProviderState
        );
        assert!(capabilities.streaming);
    }

    #[test]
    fn openai_websocket_payload_is_redacted_and_uses_create_event_type() {
        let model_meta = ModelMeta {
            id: "gpt-5.4".into(),
            provider: "openai".into(),
            name: "GPT-5.4".into(),
            context_window: 400_000,
            max_output_tokens: 32_768,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let model = Model {
            meta: model_meta,
            provider: Arc::new(OpenAiProvider::new()),
        };
        let request = build_request(&model, Context::default(), RequestOptions::default());
        let mut payload = serde_json::to_value(request).unwrap();
        if let serde_json::Value::Object(ref mut map) = payload {
            map.remove("stream");
            map.insert("store".to_string(), serde_json::Value::Bool(false));
            map.insert(
                "type".to_string(),
                serde_json::Value::String("response.create".to_string()),
            );
        }

        assert_eq!(payload["type"], "response.create");
        assert_eq!(payload["store"], false);
        assert!(payload.get("stream").is_none());
        let encoded = serde_json::to_string(&payload).unwrap();
        assert!(!encoded.contains("previous_response_id"));
        assert!(!encoded.contains("session_id"));
    }
}
