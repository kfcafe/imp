use std::collections::HashMap;
use std::pin::Pin;

use async_trait::async_trait;
use futures_core::Stream;
use serde::{Deserialize, Serialize};

use crate::auth::{ApiKey, AuthStore};
use crate::error::{Error, Result};
use crate::message::{AssistantMessage, ContentBlock, Message, StopReason};
use crate::model::{Model, ModelMeta};
use crate::provider::{Context, Provider, RequestOptions, ToolDefinition};
use crate::stream::StreamEvent;
use crate::usage::Usage;

// ---------------------------------------------------------------------------
// OpenAI Chat Completions wire-format types (request)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    messages: Vec<ApiMessage>,
    stream: bool,
    stream_options: ApiStreamOptions,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ApiToolDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ApiStreamOptions {
    include_usage: bool,
}

#[derive(Debug, Serialize)]
struct ApiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ApiToolCall>>,
}

#[derive(Debug, Serialize)]
struct ApiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: ApiToolCallFunction,
}

#[derive(Debug, Serialize)]
struct ApiToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize)]
struct ApiToolDef {
    #[serde(rename = "type")]
    tool_type: String,
    function: ApiToolDefFunction,
}

#[derive(Debug, Serialize)]
struct ApiToolDefFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

// ---------------------------------------------------------------------------
// OpenAI Chat Completions wire-format types (SSE response)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SseChunk {
    #[serde(default)]
    choices: Vec<SseChoice>,
    #[serde(default)]
    usage: Option<SseUsage>,
}

#[derive(Debug, Deserialize)]
struct SseChoice {
    delta: SseDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SseDelta {
    #[serde(default)]
    content: Option<String>,
    /// DeepSeek-style reasoning content field.
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<SseToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
struct SseToolCallDelta {
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<SseToolCallFunctionDelta>,
}

#[derive(Debug, Deserialize)]
struct SseToolCallFunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SseUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

// ---------------------------------------------------------------------------
// SSE stream state
// ---------------------------------------------------------------------------

struct ToolCallAccum {
    id: String,
    name: String,
    arguments: String,
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// OpenAI Chat Completions API provider.
///
/// Used by third-party providers (DeepSeek, Groq, Together, Mistral, xAI,
/// OpenRouter, Fireworks) that expose an OpenAI-compatible `/v1/chat/completions`
/// endpoint.
pub struct OpenAiCompatProvider {
    client: reqwest::Client,
    provider_id: String,
    base_url: String,
    models: Vec<ModelMeta>,
}

impl OpenAiCompatProvider {
    /// Create a new OpenAI-compatible provider.
    ///
    /// - `provider_id`: matches the provider's id in the registry (e.g. "deepseek")
    /// - `base_url`: API root, e.g. "https://api.deepseek.com" (no trailing slash)
    /// - `models`: models this provider can serve
    pub fn new(provider_id: &str, base_url: &str, models: Vec<ModelMeta>) -> Self {
        Self {
            client: reqwest::Client::new(),
            provider_id: provider_id.to_string(),
            base_url: base_url.to_string(),
            models,
        }
    }
}

// ---------------------------------------------------------------------------
// Request building
// ---------------------------------------------------------------------------

fn build_request(model: &Model, context: Context, options: RequestOptions) -> ApiRequest {
    let mut messages = Vec::new();

    // System prompt becomes a leading system message.
    if !options.system_prompt.is_empty() {
        messages.push(ApiMessage {
            role: "system".into(),
            content: Some(serde_json::Value::String(options.system_prompt.clone())),
            tool_call_id: None,
            tool_calls: None,
        });
    }

    for msg in &context.messages {
        messages.extend(convert_message(msg));
    }

    let tools = build_tool_defs(&options.tools);
    let max_tokens = options.max_tokens.or(Some(model.meta.max_output_tokens));

    ApiRequest {
        model: model.meta.id.clone(),
        messages,
        stream: true,
        stream_options: ApiStreamOptions {
            include_usage: true,
        },
        tools,
        temperature: options.temperature,
        max_tokens,
    }
}

fn build_tool_defs(tools: &[ToolDefinition]) -> Vec<ApiToolDef> {
    tools
        .iter()
        .map(|t| ApiToolDef {
            tool_type: "function".into(),
            function: ApiToolDefFunction {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            },
        })
        .collect()
}

/// Convert an internal Message to one or more Chat Completions API messages.
fn convert_message(msg: &Message) -> Vec<ApiMessage> {
    match msg {
        Message::User(u) => {
            let has_images = u
                .content
                .iter()
                .any(|b| matches!(b, ContentBlock::Image { .. }));

            let content = if has_images {
                // Content array with text + image_url items.
                let parts: Vec<serde_json::Value> = u
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text { text } => Some(serde_json::json!({
                            "type": "text",
                            "text": text
                        })),
                        ContentBlock::Image { media_type, data } => Some(serde_json::json!({
                            "type": "image_url",
                            "image_url": { "url": format!("data:{media_type};base64,{data}") }
                        })),
                        _ => None,
                    })
                    .collect();
                serde_json::Value::Array(parts)
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
                serde_json::Value::String(text)
            };

            vec![ApiMessage {
                role: "user".into(),
                content: Some(content),
                tool_call_id: None,
                tool_calls: None,
            }]
        }
        Message::Assistant(a) => {
            let text: String = a
                .content
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");

            let tool_calls: Vec<ApiToolCall> = a
                .content
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::ToolCall {
                        id,
                        name,
                        arguments,
                    } => Some(ApiToolCall {
                        id: id.clone(),
                        call_type: "function".into(),
                        function: ApiToolCallFunction {
                            name: name.clone(),
                            arguments: arguments.to_string(),
                        },
                    }),
                    _ => None,
                })
                .collect();

            let content = if text.is_empty() {
                None
            } else {
                Some(serde_json::Value::String(text))
            };
            let tool_calls_opt = if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            };

            vec![ApiMessage {
                role: "assistant".into(),
                content,
                tool_call_id: None,
                tool_calls: tool_calls_opt,
            }]
        }
        Message::ToolResult(tr) => {
            let output: String = tr
                .content
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            vec![ApiMessage {
                role: "tool".into(),
                content: Some(serde_json::Value::String(output)),
                tool_call_id: Some(tr.tool_call_id.clone()),
                tool_calls: None,
            }]
        }
    }
}

// ---------------------------------------------------------------------------
// SSE parsing
// ---------------------------------------------------------------------------

fn parse_sse_chunk(data: &str) -> Result<Option<SseChunk>> {
    let trimmed = data.trim();
    if trimmed.is_empty() || trimmed == "[DONE]" {
        return Ok(None);
    }
    serde_json::from_str(trimmed)
        .map(Some)
        .map_err(|e| Error::Stream(format!("Failed to parse SSE chunk: {e}: {trimmed}")))
}

// ---------------------------------------------------------------------------
// Streaming implementation
// ---------------------------------------------------------------------------

fn stream_response(
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    request: ApiRequest,
) -> Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>> {
    let (tx, rx) = futures::channel::mpsc::unbounded();

    tokio::spawn(async move {
        let url = format!("{base_url}/v1/chat/completions");

        let result = client
            .post(&url)
            .bearer_auth(&api_key)
            .json(&request)
            .send()
            .await;

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

        // Emit MessageStart once we have a successful response. The model id
        // comes from the request since Chat Completions doesn't include it in
        // every SSE chunk (unlike Anthropic).
        if tx
            .unbounded_send(Ok(StreamEvent::MessageStart {
                model: request.model.clone(),
            }))
            .is_err()
        {
            return;
        }

        let mut tool_accum: HashMap<usize, ToolCallAccum> = HashMap::new();
        let mut content_buf = Vec::new();
        let mut usage = Usage::default();
        let mut stop_reason = StopReason::EndTurn;
        let mut buf = String::new();

        use futures::StreamExt;
        let mut byte_stream = resp.bytes_stream();

        while let Some(chunk) = byte_stream.next().await {
            match chunk {
                Ok(bytes) => {
                    buf.push_str(&String::from_utf8_lossy(&bytes));

                    while let Some(pos) = buf.find('\n') {
                        let line = buf[..pos].to_string();
                        buf = buf[pos + 1..].to_string();

                        let trimmed = line.trim();
                        if let Some(data) = trimmed.strip_prefix("data: ") {
                            match parse_sse_chunk(data) {
                                Ok(Some(chunk)) => {
                                    if let Some(u) = chunk.usage {
                                        usage.input_tokens = u.prompt_tokens;
                                        usage.output_tokens = u.completion_tokens;
                                    }

                                    for choice in chunk.choices {
                                        let delta = choice.delta;

                                        // Thinking/reasoning content (DeepSeek, etc.)
                                        if let Some(reasoning) = delta.reasoning_content {
                                            if !reasoning.is_empty()
                                                && tx
                                                    .unbounded_send(Ok(
                                                        StreamEvent::ThinkingDelta {
                                                            text: reasoning,
                                                        },
                                                    ))
                                                    .is_err()
                                            {
                                                return;
                                            }
                                        }

                                        // Regular text content
                                        if let Some(text) = delta.content {
                                            if !text.is_empty() {
                                                content_buf.push(ContentBlock::Text {
                                                    text: text.clone(),
                                                });
                                                if tx
                                                    .unbounded_send(Ok(StreamEvent::TextDelta {
                                                        text,
                                                    }))
                                                    .is_err()
                                                {
                                                    return;
                                                }
                                            }
                                        }

                                        // Tool call deltas
                                        if let Some(tc_deltas) = delta.tool_calls {
                                            for tc in tc_deltas {
                                                let entry = tool_accum
                                                    .entry(tc.index)
                                                    .or_insert_with(|| ToolCallAccum {
                                                        id: String::new(),
                                                        name: String::new(),
                                                        arguments: String::new(),
                                                    });
                                                if let Some(id) = tc.id {
                                                    entry.id = id;
                                                }
                                                if let Some(func) = tc.function {
                                                    if let Some(name) = func.name {
                                                        entry.name = name;
                                                    }
                                                    if let Some(args) = func.arguments {
                                                        entry.arguments.push_str(&args);
                                                    }
                                                }
                                            }
                                        }

                                        // Finish reason
                                        if let Some(reason) = choice.finish_reason {
                                            stop_reason = match reason.as_str() {
                                                "stop" => StopReason::EndTurn,
                                                "tool_calls" => StopReason::ToolUse,
                                                "length" => StopReason::MaxTokens,
                                                other => StopReason::Error(other.to_string()),
                                            };
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
                    }
                }
                Err(e) => {
                    let _ = tx.unbounded_send(Err(Error::Http(e)));
                    return;
                }
            }
        }

        // Emit complete tool calls after stream ends.
        let mut tc_indices: Vec<usize> = tool_accum.keys().copied().collect();
        tc_indices.sort();
        for idx in tc_indices {
            if let Some(tc) = tool_accum.remove(&idx) {
                let arguments: serde_json::Value = serde_json::from_str(&tc.arguments)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                content_buf.push(ContentBlock::ToolCall {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    arguments: arguments.clone(),
                });
                if tx
                    .unbounded_send(Ok(StreamEvent::ToolCall {
                        id: tc.id,
                        name: tc.name,
                        arguments,
                    }))
                    .is_err()
                {
                    return;
                }
            }
        }

        let message = AssistantMessage {
            content: content_buf,
            usage: Some(usage),
            stop_reason,
            timestamp: crate::now(),
        };
        let _ = tx.unbounded_send(Ok(StreamEvent::MessageEnd { message }));
    });

    Box::pin(rx)
}

#[async_trait]
impl Provider for OpenAiCompatProvider {
    fn stream(
        &self,
        model: &Model,
        context: Context,
        options: RequestOptions,
        api_key: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>> {
        let request = build_request(model, context, options);
        stream_response(
            self.client.clone(),
            self.base_url.clone(),
            api_key.to_string(),
            request,
        )
    }

    async fn resolve_auth(&self, auth: &AuthStore) -> Result<ApiKey> {
        auth.resolve(&self.provider_id)
    }

    fn id(&self) -> &str {
        &self.provider_id
    }

    fn models(&self) -> &[ModelMeta] {
        &self.models
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::message::{AssistantMessage, ToolResultMessage, UserMessage};
    use crate::model::{Capabilities, ModelPricing};
    use crate::provider::{Context, RequestOptions};

    fn test_model() -> Model {
        let meta = ModelMeta {
            id: "deepseek-chat".into(),
            provider: "deepseek".into(),
            name: "DeepSeek Chat".into(),
            context_window: 64_000,
            max_output_tokens: 4_096,
            pricing: ModelPricing::default(),
            capabilities: Capabilities {
                reasoning: false,
                images: false,
                tool_use: true,
            },
        };
        let provider =
            OpenAiCompatProvider::new("deepseek", "https://api.deepseek.com", vec![meta.clone()]);
        Model {
            meta,
            provider: Arc::new(provider),
        }
    }

    // -- build_request tests --

    #[test]
    fn openai_compat_system_prompt_becomes_system_message() {
        let model = test_model();
        let context = Context { messages: vec![] };
        let options = RequestOptions {
            system_prompt: "You are a helpful assistant.".into(),
            ..Default::default()
        };

        let req = build_request(&model, context, options);

        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].role, "system");
        assert_eq!(
            req.messages[0].content,
            Some(serde_json::Value::String(
                "You are a helpful assistant.".into()
            ))
        );
    }

    #[test]
    fn openai_compat_empty_system_prompt_omitted() {
        let model = test_model();
        let options = RequestOptions {
            system_prompt: "".into(),
            ..Default::default()
        };

        let req = build_request(&model, Context::default(), options);
        assert!(req.messages.is_empty());
    }

    #[test]
    fn openai_compat_user_text_message() {
        let model = test_model();
        let context = Context {
            messages: vec![Message::user("Hello!")],
        };
        let options = RequestOptions::default();

        let req = build_request(&model, context, options);

        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].role, "user");
        assert_eq!(
            req.messages[0].content,
            Some(serde_json::Value::String("Hello!".into()))
        );
    }

    #[test]
    fn openai_compat_user_message_with_image() {
        let model = test_model();
        let context = Context {
            messages: vec![Message::User(UserMessage {
                content: vec![
                    ContentBlock::Text {
                        text: "What is this?".into(),
                    },
                    ContentBlock::Image {
                        media_type: "image/png".into(),
                        data: "abc123".into(),
                    },
                ],
                timestamp: 0,
            })],
        };

        let req = build_request(&model, context, RequestOptions::default());

        let content = req.messages[0].content.as_ref().unwrap();
        let arr = content.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["type"], "text");
        assert_eq!(arr[0]["text"], "What is this?");
        assert_eq!(arr[1]["type"], "image_url");
        assert_eq!(arr[1]["image_url"]["url"], "data:image/png;base64,abc123");
    }

    #[test]
    fn openai_compat_assistant_with_tool_call() {
        let msg = Message::Assistant(AssistantMessage {
            content: vec![
                ContentBlock::Text {
                    text: "Running bash.".into(),
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
        });

        let converted = convert_message(&msg);
        assert_eq!(converted.len(), 1);
        let api_msg = &converted[0];
        assert_eq!(api_msg.role, "assistant");
        assert_eq!(
            api_msg.content,
            Some(serde_json::Value::String("Running bash.".into()))
        );
        let tcs = api_msg.tool_calls.as_ref().unwrap();
        assert_eq!(tcs.len(), 1);
        assert_eq!(tcs[0].id, "call_1");
        assert_eq!(tcs[0].function.name, "bash");
        assert_eq!(tcs[0].function.arguments, r#"{"command":"ls"}"#);
    }

    #[test]
    fn openai_compat_tool_result_message() {
        let msg = Message::ToolResult(ToolResultMessage {
            tool_call_id: "call_1".into(),
            tool_name: "bash".into(),
            content: vec![ContentBlock::Text {
                text: "README.md\nsrc/".into(),
            }],
            is_error: false,
            details: serde_json::Value::Null,
            timestamp: 0,
        });

        let converted = convert_message(&msg);
        assert_eq!(converted.len(), 1);
        let api_msg = &converted[0];
        assert_eq!(api_msg.role, "tool");
        assert_eq!(api_msg.tool_call_id.as_deref(), Some("call_1"));
        assert_eq!(
            api_msg.content,
            Some(serde_json::Value::String("README.md\nsrc/".into()))
        );
    }

    #[test]
    fn openai_compat_tool_definitions() {
        let tools = vec![crate::provider::ToolDefinition {
            name: "read_file".into(),
            description: "Read a file from disk".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
        }];

        let defs = build_tool_defs(&tools);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].tool_type, "function");
        assert_eq!(defs[0].function.name, "read_file");
        assert_eq!(defs[0].function.description, "Read a file from disk");
        assert_eq!(defs[0].function.parameters["type"], "object");
    }

    #[test]
    fn openai_compat_temperature_included_when_set() {
        let model = test_model();
        let options = RequestOptions {
            temperature: Some(0.7),
            ..Default::default()
        };

        let req = build_request(&model, Context::default(), options);
        assert_eq!(req.temperature, Some(0.7));
    }

    #[test]
    fn openai_compat_temperature_omitted_when_none() {
        let model = test_model();
        let req = build_request(&model, Context::default(), RequestOptions::default());
        assert!(req.temperature.is_none());
    }

    #[test]
    fn openai_compat_max_tokens_falls_back_to_model_default() {
        let model = test_model();
        let req = build_request(&model, Context::default(), RequestOptions::default());
        // model.meta.max_output_tokens is 4096
        assert_eq!(req.max_tokens, Some(4_096));
    }

    #[test]
    fn openai_compat_stream_options_always_include_usage() {
        let model = test_model();
        let req = build_request(&model, Context::default(), RequestOptions::default());
        assert!(req.stream_options.include_usage);
    }

    #[test]
    fn openai_compat_request_serializes_correctly() {
        let model = test_model();
        let context = Context {
            messages: vec![Message::user("Hi")],
        };
        let options = RequestOptions {
            system_prompt: "Be helpful.".into(),
            temperature: Some(0.5),
            ..Default::default()
        };

        let req = build_request(&model, context, options);
        let json = serde_json::to_value(&req).unwrap();

        assert_eq!(json["model"], "deepseek-chat");
        assert!(json["stream"].as_bool().unwrap());
        assert_eq!(json["stream_options"]["include_usage"], true);
        assert_eq!(json["temperature"], 0.5);

        let messages = json["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "Be helpful.");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "Hi");
    }

    // -- SSE parsing tests --

    #[test]
    fn openai_compat_parse_text_chunk() {
        let data = r#"{"choices":[{"delta":{"content":"Hello"},"index":0,"finish_reason":null}]}"#;
        let chunk = parse_sse_chunk(data).unwrap().unwrap();
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hello"));
    }

    #[test]
    fn openai_compat_parse_done_returns_none() {
        let result = parse_sse_chunk("[DONE]").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn openai_compat_parse_empty_returns_none() {
        let result = parse_sse_chunk("").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn openai_compat_parse_malformed_returns_error() {
        let result = parse_sse_chunk("{bad json}");
        assert!(result.is_err());
    }

    #[test]
    fn openai_compat_parse_tool_call_delta() {
        let data = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_abc","type":"function","function":{"name":"bash","arguments":""}}]},"index":0,"finish_reason":null}]}"#;
        let chunk = parse_sse_chunk(data).unwrap().unwrap();
        let tcs = chunk.choices[0].delta.tool_calls.as_ref().unwrap();
        assert_eq!(tcs.len(), 1);
        assert_eq!(tcs[0].index, 0);
        assert_eq!(tcs[0].id.as_deref(), Some("call_abc"));
        assert_eq!(
            tcs[0].function.as_ref().unwrap().name.as_deref(),
            Some("bash")
        );
    }

    #[test]
    fn openai_compat_parse_usage_chunk() {
        let data = r#"{"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":20,"total_tokens":30}}"#;
        let chunk = parse_sse_chunk(data).unwrap().unwrap();
        let usage = chunk.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 20);
    }

    #[test]
    fn openai_compat_parse_reasoning_content() {
        let data = r#"{"choices":[{"delta":{"reasoning_content":"Let me think...","content":null},"index":0,"finish_reason":null}]}"#;
        let chunk = parse_sse_chunk(data).unwrap().unwrap();
        assert_eq!(
            chunk.choices[0].delta.reasoning_content.as_deref(),
            Some("Let me think...")
        );
        assert!(chunk.choices[0].delta.content.is_none());
    }

    #[test]
    fn openai_compat_provider_id() {
        let provider = OpenAiCompatProvider::new("deepseek", "https://api.deepseek.com", vec![]);
        assert_eq!(provider.id(), "deepseek");
    }

    #[test]
    fn openai_compat_provider_models() {
        let meta = ModelMeta {
            id: "deepseek-chat".into(),
            provider: "deepseek".into(),
            name: "DeepSeek Chat".into(),
            context_window: 64_000,
            max_output_tokens: 4_096,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        let provider =
            OpenAiCompatProvider::new("deepseek", "https://api.deepseek.com", vec![meta]);
        assert_eq!(provider.models().len(), 1);
        assert_eq!(provider.models()[0].id, "deepseek-chat");
    }
}
