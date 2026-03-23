use imp_llm::{ContentBlock, Message};

/// Check if the session was complex enough to warrant a learning nudge.
///
/// Counts tool calls across all assistant messages. Returns true if the count
/// meets or exceeds the threshold, suggesting the agent should consider saving
/// the approach as a skill or persisting something to memory.
pub fn should_nudge_learning(messages: &[Message], threshold: u32) -> bool {
    if threshold == 0 {
        return false;
    }

    let tool_call_count: u32 = messages
        .iter()
        .filter_map(|m| match m {
            Message::Assistant(a) => Some(&a.content),
            _ => None,
        })
        .flat_map(|blocks| blocks.iter())
        .filter(|b| matches!(b, ContentBlock::ToolCall { .. }))
        .count() as u32;

    tool_call_count >= threshold
}

/// The nudge message injected after complex sessions.
pub const LEARNING_NUDGE: &str = "\
Before we finish — this was a complex session. Consider:
1. Is there anything worth saving to memory (environment facts, lessons learned)?
2. Should the approach be saved as a skill for future reuse?
3. If you used a skill that was wrong or incomplete, patch it.";

/// Learning instructions injected into Layer 1 of the system prompt.
pub const LEARNING_INSTRUCTIONS: &str = "\
## Memory & Learning

You have persistent memory across sessions. Use the memory tool to save:
- Environment facts (OS, tools, project setup) → target: memory
- User preferences and corrections → target: user
- Lessons learned and tool quirks → target: memory

When you complete a complex task (5+ tool calls, error recovery, or user
correction), consider saving the approach as a skill via skill_manage.

When you load a skill and find it incomplete or wrong, patch it.

Do NOT save: trivial facts, easily re-discovered info, raw data dumps, or
anything already in AGENTS.md.";

#[cfg(test)]
mod tests {
    use super::*;
    use imp_llm::{AssistantMessage, StopReason, Usage, UserMessage};

    fn user_msg(text: &str) -> Message {
        Message::User(UserMessage {
            content: vec![ContentBlock::Text {
                text: text.to_string(),
            }],
            timestamp: 0,
        })
    }

    fn assistant_text(text: &str) -> Message {
        Message::Assistant(AssistantMessage {
            content: vec![ContentBlock::Text {
                text: text.to_string(),
            }],
            usage: None,
            stop_reason: StopReason::EndTurn,
            timestamp: 0,
        })
    }

    fn assistant_with_tool_calls(n: usize) -> Message {
        let mut content = Vec::new();
        for i in 0..n {
            content.push(ContentBlock::ToolCall {
                id: format!("call_{i}"),
                name: "read".to_string(),
                arguments: serde_json::json!({}),
            });
        }
        Message::Assistant(AssistantMessage {
            content,
            usage: None,
            stop_reason: StopReason::ToolUse,
            timestamp: 0,
        })
    }

    fn tool_result(call_id: &str) -> Message {
        Message::ToolResult(imp_llm::ToolResultMessage {
            tool_call_id: call_id.to_string(),
            tool_name: "read".to_string(),
            content: vec![ContentBlock::Text {
                text: "ok".to_string(),
            }],
            is_error: false,
            details: serde_json::Value::Null,
            timestamp: 0,
        })
    }

    #[test]
    fn learning_nudge_below_threshold() {
        let messages = vec![
            user_msg("hello"),
            assistant_with_tool_calls(2),
            tool_result("call_0"),
            tool_result("call_1"),
            assistant_text("done"),
        ];
        assert!(!should_nudge_learning(&messages, 8));
    }

    #[test]
    fn learning_nudge_at_threshold() {
        // 8 tool calls spread across 2 assistant messages
        let messages = vec![
            user_msg("do stuff"),
            assistant_with_tool_calls(4),
            tool_result("call_0"),
            assistant_with_tool_calls(4),
            tool_result("call_0"),
            assistant_text("done"),
        ];
        assert!(should_nudge_learning(&messages, 8));
    }

    #[test]
    fn learning_nudge_above_threshold() {
        let messages = vec![
            user_msg("big task"),
            assistant_with_tool_calls(10),
            assistant_text("done"),
        ];
        assert!(should_nudge_learning(&messages, 8));
    }

    #[test]
    fn learning_nudge_zero_threshold_never_nudges() {
        let messages = vec![assistant_with_tool_calls(100)];
        assert!(!should_nudge_learning(&messages, 0));
    }

    #[test]
    fn learning_nudge_empty_messages() {
        assert!(!should_nudge_learning(&[], 8));
    }

    #[test]
    fn learning_nudge_only_text_messages() {
        let messages = vec![user_msg("hello"), assistant_text("hi back")];
        assert!(!should_nudge_learning(&messages, 1));
    }
}
