use imp_core::agent::AgentEvent;
use imp_llm::message::{ContentBlock as LlmContentBlock, Message, StopReason as LlmStopReason};

use super::protocol::{
    ContentBlock, EmbeddedResource, SessionUpdate, StopReason, ToolCallContent, ToolCallStatus,
    ToolKind,
};

pub(crate) fn prompt_blocks_to_text(blocks: &[ContentBlock]) -> Result<String, String> {
    let mut parts = Vec::new();
    for block in blocks {
        match block {
            ContentBlock::Text { text } => parts.push(text.clone()),
            ContentBlock::Resource { resource } => {
                let text = resource.text.as_deref().ok_or_else(|| {
                    format!(
                        "unsupported resource without embedded text: {}",
                        resource.uri
                    )
                })?;
                parts.push(format!(
                    "<resource uri=\"{}\"{}>\n{}\n</resource>",
                    resource.uri,
                    resource
                        .mime_type
                        .as_ref()
                        .map(|mime| format!(" mime_type=\"{mime}\""))
                        .unwrap_or_default(),
                    text
                ));
            }
            ContentBlock::ResourceLink { uri, name } => {
                let label = name.as_deref().unwrap_or(uri);
                parts.push(format!("Resource link: {label} ({uri})"));
            }
            ContentBlock::Unknown => {
                return Err("unsupported ACP content block type".to_string());
            }
        }
    }

    Ok(parts.join("\n\n"))
}

pub(crate) fn message_to_session_updates(
    message: &Message,
    from_history: bool,
) -> Vec<SessionUpdate> {
    match message {
        Message::User(user) => user
            .content
            .iter()
            .filter_map(llm_content_to_acp)
            .map(|content| SessionUpdate::UserMessageChunk { content })
            .collect(),
        Message::Assistant(assistant) => assistant
            .content
            .iter()
            .filter_map(llm_content_to_acp)
            .map(|content| SessionUpdate::AgentMessageChunk { content })
            .collect(),
        Message::ToolResult(result) if from_history => vec![SessionUpdate::ToolCallUpdate {
            tool_call_id: result.tool_call_id.clone(),
            status: Some(if result.is_error {
                ToolCallStatus::Failed
            } else {
                ToolCallStatus::Completed
            }),
            content: result
                .content
                .iter()
                .filter_map(llm_content_to_acp)
                .map(|content| ToolCallContent::Content { content })
                .collect(),
            raw_output: Some(result.details.clone()),
        }],
        Message::ToolResult(_) => Vec::new(),
    }
}

pub(crate) fn agent_event_to_session_updates(event: &AgentEvent) -> Vec<SessionUpdate> {
    match event {
        AgentEvent::MessageDelta { delta } => match delta {
            imp_llm::stream::StreamEvent::TextDelta { text } => {
                vec![SessionUpdate::AgentMessageChunk {
                    content: ContentBlock::Text { text: text.clone() },
                }]
            }
            imp_llm::stream::StreamEvent::ToolCall {
                id,
                name,
                arguments,
            } => vec![SessionUpdate::ToolCall {
                tool_call_id: id.clone(),
                title: tool_title(name, arguments),
                kind: tool_kind(name),
                status: ToolCallStatus::Pending,
                raw_input: Some(arguments.clone()),
            }],
            _ => Vec::new(),
        },
        AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            args,
        } => vec![SessionUpdate::ToolCallUpdate {
            tool_call_id: tool_call_id.clone(),
            status: Some(ToolCallStatus::InProgress),
            content: vec![ToolCallContent::Content {
                content: ContentBlock::Text {
                    text: format!("Running {tool_name}"),
                },
            }],
            raw_output: Some(args.clone()),
        }],
        AgentEvent::ToolOutputDelta { tool_call_id, text } => vec![SessionUpdate::ToolCallUpdate {
            tool_call_id: tool_call_id.clone(),
            status: Some(ToolCallStatus::InProgress),
            content: vec![ToolCallContent::Content {
                content: ContentBlock::Text { text: text.clone() },
            }],
            raw_output: None,
        }],
        AgentEvent::ToolExecutionEnd {
            tool_call_id,
            result,
            ..
        } => {
            vec![SessionUpdate::ToolCallUpdate {
                tool_call_id: tool_call_id.clone(),
                status: Some(if result.is_error {
                    ToolCallStatus::Failed
                } else {
                    ToolCallStatus::Completed
                }),
                content: result
                    .content
                    .iter()
                    .filter_map(llm_content_to_acp)
                    .map(|content| ToolCallContent::Content { content })
                    .collect(),
                raw_output: Some(result.details.clone()),
            }]
        }
        AgentEvent::Warning { message } | AgentEvent::Error { error: message } => {
            vec![SessionUpdate::AgentMessageChunk {
                content: ContentBlock::Text {
                    text: format!("\n[{message}]\n"),
                },
            }]
        }
        _ => Vec::new(),
    }
}

pub(crate) fn stop_reason_from_agent_status(
    status: Option<&imp_core::agent::RunFinalStatus>,
    cancelled: bool,
) -> StopReason {
    if cancelled {
        return StopReason::Cancelled;
    }

    match status {
        Some(imp_core::agent::RunFinalStatus::Failed { .. }) => StopReason::Refusal,
        Some(imp_core::agent::RunFinalStatus::Blocked { .. }) => StopReason::Refusal,
        _ => StopReason::EndTurn,
    }
}

pub(crate) fn stop_reason_from_llm(reason: &LlmStopReason) -> StopReason {
    match reason {
        LlmStopReason::EndTurn | LlmStopReason::ToolUse => StopReason::EndTurn,
        LlmStopReason::MaxTokens => StopReason::MaxTokens,
        LlmStopReason::Error(_) => StopReason::Refusal,
    }
}

fn llm_content_to_acp(block: &LlmContentBlock) -> Option<ContentBlock> {
    match block {
        LlmContentBlock::Text { text } | LlmContentBlock::Thinking { text } => {
            Some(ContentBlock::Text { text: text.clone() })
        }
        LlmContentBlock::ToolCall {
            id,
            name,
            arguments,
        } => Some(ContentBlock::Resource {
            resource: EmbeddedResource {
                uri: format!("imp://tool-call/{id}"),
                mime_type: Some("application/json".to_string()),
                text: Some(
                    serde_json::json!({
                        "id": id,
                        "name": name,
                        "arguments": arguments,
                    })
                    .to_string(),
                ),
            },
        }),
        LlmContentBlock::Image { .. } => None,
    }
}

fn tool_title(name: &str, arguments: &serde_json::Value) -> String {
    match name {
        "bash" => arguments
            .get("command")
            .and_then(serde_json::Value::as_str)
            .map(|command| format!("Run `{command}`"))
            .unwrap_or_else(|| "Run shell command".to_string()),
        "read" => "Read file".to_string(),
        "edit" => "Edit file".to_string(),
        "write" => "Write file".to_string(),
        other => other.replace('_', " "),
    }
}

fn tool_kind(name: &str) -> ToolKind {
    match name {
        "read" | "scan" | "references" | "impact" => ToolKind::Read,
        "edit" | "write" => ToolKind::Edit,
        "bash" | "audit_scan" => ToolKind::Execute,
        "web" => ToolKind::Fetch,
        "workflow" => ToolKind::Think,
        other if other.contains("search") => ToolKind::Search,
        _ => ToolKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use imp_llm::message::{AssistantMessage, UserMessage};
    use serde_json::json;

    #[test]
    fn prompt_blocks_to_text_includes_embedded_resource() {
        let prompt = prompt_blocks_to_text(&[
            ContentBlock::Text {
                text: "Review this".to_string(),
            },
            ContentBlock::Resource {
                resource: EmbeddedResource {
                    uri: "file:///tmp/main.rs".to_string(),
                    mime_type: Some("text/rust".to_string()),
                    text: Some("fn main() {}".to_string()),
                },
            },
        ])
        .unwrap();

        assert!(prompt.contains("Review this"));
        assert!(prompt.contains("file:///tmp/main.rs"));
        assert!(prompt.contains("fn main() {}"));
    }

    #[test]
    fn message_to_session_updates_maps_user_and_assistant_text() {
        let user = Message::User(UserMessage {
            content: vec![LlmContentBlock::Text {
                text: "hi".to_string(),
            }],
            timestamp: 1,
        });
        let assistant = Message::Assistant(AssistantMessage {
            content: vec![LlmContentBlock::Text {
                text: "hello".to_string(),
            }],
            usage: None,
            stop_reason: LlmStopReason::EndTurn,
            timestamp: 2,
        });

        assert!(matches!(
            &message_to_session_updates(&user, false)[0],
            SessionUpdate::UserMessageChunk { content: ContentBlock::Text { text } } if text == "hi"
        ));
        assert!(matches!(
            &message_to_session_updates(&assistant, false)[0],
            SessionUpdate::AgentMessageChunk { content: ContentBlock::Text { text } } if text == "hello"
        ));
    }

    #[test]
    fn agent_event_to_session_updates_maps_tool_call() {
        let event = AgentEvent::MessageDelta {
            delta: imp_llm::stream::StreamEvent::ToolCall {
                id: "call_1".to_string(),
                name: "bash".to_string(),
                arguments: json!({"command": "cargo test"}),
            },
        };

        let updates = agent_event_to_session_updates(&event);
        assert!(matches!(
            &updates[0],
            SessionUpdate::ToolCall { tool_call_id, kind: ToolKind::Execute, .. } if tool_call_id == "call_1"
        ));
    }
}
