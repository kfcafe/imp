use serde::{Deserialize, Serialize};

/// A message in the conversation, tagged by role.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    /// Content from the human user.
    #[serde(rename = "user")]
    User(UserMessage),
    /// Content from the LLM assistant.
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    /// Result of a tool execution returned to the model.
    #[serde(rename = "tool_result")]
    ToolResult(ToolResultMessage),
}

/// A message sent by the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    /// One or more content blocks (text, images, etc.).
    pub content: Vec<ContentBlock>,
    /// Unix timestamp in seconds when the message was created.
    pub timestamp: u64,
}

/// A response from the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    /// Content blocks produced by the model.
    pub content: Vec<ContentBlock>,
    /// Token usage for this response, if reported by the provider.
    pub usage: Option<crate::usage::Usage>,
    /// Why the model stopped generating.
    pub stop_reason: StopReason,
    /// Unix timestamp in seconds.
    pub timestamp: u64,
}

/// The result of executing a tool, sent back to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultMessage {
    /// Provider-assigned call id that pairs this result with its tool call.
    pub tool_call_id: String,
    /// Name of the tool that was executed.
    pub tool_name: String,
    /// Output content blocks.
    pub content: Vec<ContentBlock>,
    /// Whether the tool execution failed.
    pub is_error: bool,
    /// Arbitrary metadata about the execution.
    #[serde(default)]
    pub details: serde_json::Value,
    /// Unix timestamp in seconds.
    pub timestamp: u64,
}

/// A single block of content within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// Plain text content.
    #[serde(rename = "text")]
    Text { text: String },
    /// Extended thinking / chain-of-thought output.
    #[serde(rename = "thinking")]
    Thinking { text: String },
    /// A request from the model to call a tool.
    #[serde(rename = "tool_call")]
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    /// Base64-encoded image data.
    #[serde(rename = "image")]
    Image {
        media_type: String,
        data: String,
    },
}

/// Reason the model stopped generating tokens.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StopReason {
    /// Natural end of response.
    EndTurn,
    /// Model wants to call one or more tools.
    ToolUse,
    /// Hit the max_tokens limit.
    MaxTokens,
    /// An error occurred during generation.
    Error(String),
}

impl Message {
    /// Convenience constructor for a simple text user message.
    pub fn user(text: impl Into<String>) -> Self {
        Message::User(UserMessage {
            content: vec![ContentBlock::Text { text: text.into() }],
            timestamp: crate::now(),
        })
    }

    /// True if this is a user message.
    pub fn is_user(&self) -> bool {
        matches!(self, Message::User(_))
    }

    /// True if this is an assistant message.
    pub fn is_assistant(&self) -> bool {
        matches!(self, Message::Assistant(_))
    }

    /// True if this is a tool result.
    pub fn is_tool_result(&self) -> bool {
        matches!(self, Message::ToolResult(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_user_round_trip() {
        let msg = Message::User(UserMessage {
            content: vec![ContentBlock::Text {
                text: "Hello".into(),
            }],
            timestamp: 1700000000,
        });
        let json = serde_json::to_string(&msg).unwrap();
        let restored: Message = serde_json::from_str(&json).unwrap();
        assert!(restored.is_user());
        if let Message::User(u) = &restored {
            assert_eq!(u.timestamp, 1700000000);
            assert_eq!(u.content.len(), 1);
        } else {
            panic!("expected User variant");
        }
    }

    #[test]
    fn message_assistant_round_trip() {
        let msg = Message::Assistant(AssistantMessage {
            content: vec![
                ContentBlock::Text {
                    text: "Sure!".into(),
                },
                ContentBlock::Thinking {
                    text: "Let me think...".into(),
                },
            ],
            usage: Some(crate::usage::Usage {
                input_tokens: 100,
                output_tokens: 50,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
            }),
            stop_reason: StopReason::EndTurn,
            timestamp: 1700000001,
        });
        let json = serde_json::to_string(&msg).unwrap();
        let restored: Message = serde_json::from_str(&json).unwrap();
        assert!(restored.is_assistant());
        if let Message::Assistant(a) = &restored {
            assert_eq!(a.content.len(), 2);
            assert_eq!(a.stop_reason, StopReason::EndTurn);
            assert_eq!(a.usage.as_ref().unwrap().input_tokens, 100);
        } else {
            panic!("expected Assistant variant");
        }
    }

    #[test]
    fn message_tool_result_round_trip() {
        let msg = Message::ToolResult(ToolResultMessage {
            tool_call_id: "call_123".into(),
            tool_name: "read_file".into(),
            content: vec![ContentBlock::Text {
                text: "file contents".into(),
            }],
            is_error: false,
            details: serde_json::json!({"path": "/tmp/test"}),
            timestamp: 1700000002,
        });
        let json = serde_json::to_string(&msg).unwrap();
        let restored: Message = serde_json::from_str(&json).unwrap();
        assert!(restored.is_tool_result());
        if let Message::ToolResult(t) = &restored {
            assert_eq!(t.tool_call_id, "call_123");
            assert_eq!(t.tool_name, "read_file");
            assert!(!t.is_error);
        } else {
            panic!("expected ToolResult variant");
        }
    }

    #[test]
    fn tool_call_content_block_round_trip() {
        let block = ContentBlock::ToolCall {
            id: "tc_1".into(),
            name: "bash".into(),
            arguments: serde_json::json!({"command": "ls"}),
        };
        let json = serde_json::to_string(&block).unwrap();
        let restored: ContentBlock = serde_json::from_str(&json).unwrap();
        if let ContentBlock::ToolCall {
            id,
            name,
            arguments,
        } = restored
        {
            assert_eq!(id, "tc_1");
            assert_eq!(name, "bash");
            assert_eq!(arguments["command"], "ls");
        } else {
            panic!("expected ToolCall variant");
        }
    }
}
