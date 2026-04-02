use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures_core::Stream;
use serde::{Deserialize, Serialize};

use crate::auth::{ApiKey, AuthStore};
use crate::error::{Error, Result};
use crate::message::{AssistantMessage, ContentBlock, Message, StopReason};
use crate::model::{Model, ModelMeta};
use crate::provider::{Context, Provider, RequestOptions, ThinkingLevel, ToolDefinition};
use crate::stream::StreamEvent;
use crate::usage::Usage;

const API_URL: &str = "https://api.openai.com/v1/responses";

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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
struct SseResponse {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    usage: Option<SseUsage>,
}

#[derive(Debug, Deserialize)]
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
}

impl StreamState {
    fn new() -> Self {
        Self {
            model: String::new(),
            items: Vec::new(),
            content: Vec::new(),
            usage: Usage::default(),
            stop_reason: StopReason::EndTurn,
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

    let max_output_tokens = options.max_tokens.or(Some(model.meta.max_output_tokens));

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
    tools
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

fn process_sse_event(event: SseEvent, state: &mut StreamState) -> Vec<StreamEvent> {
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
        "response.completed" => {
            if let Some(resp) = event.response {
                if let Some(u) = resp.usage {
                    state.usage.input_tokens = u.input_tokens;
                    state.usage.output_tokens = u.output_tokens;
                    if let Some(details) = u.input_tokens_details {
                        state.usage.cache_read_tokens = details.cached_tokens;
                    }
                }

                state.stop_reason = match resp.status.as_deref() {
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
        let client = self.client.clone();
        let api_key = api_key.to_string();
        stream_response(client, api_key, request)
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

    // -- Message serialization tests --

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
}
