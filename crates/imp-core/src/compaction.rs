use futures::StreamExt;
use imp_llm::{ContentBlock, Message, Model, RequestOptions, StreamEvent};

use crate::error::Result;

/// Options for the compaction process.
#[derive(Debug, Clone)]
pub struct CompactionOptions {
    /// Custom instructions to include in the compaction prompt.
    pub custom_instructions: Option<String>,
    /// Number of recent turns to keep intact (default: 3).
    pub keep_recent_turns: usize,
}

impl Default for CompactionOptions {
    fn default() -> Self {
        Self {
            custom_instructions: None,
            keep_recent_turns: 3,
        }
    }
}

/// Result of compacting the conversation.
#[derive(Debug, Clone)]
pub struct CompactionResult {
    pub summary: String,
    pub first_kept_id: String,
    pub tokens_before: u32,
    pub tokens_after: u32,
}

/// Compact the conversation via an LLM summarization call.
///
/// Splits the conversation into "old" (to summarize) and "recent" (to keep).
/// Sends the old messages to the model with a compaction prompt that preserves:
/// - The user's original request (verbatim, quoted)
/// - The current goal and what's been accomplished
/// - Files read, written, and edited
/// - Tests added, commands run
/// - Work remaining: next steps, blockers
/// - Things that were tried and FAILED
/// - Things explicitly forbidden by the user
pub async fn compact(
    messages: &[Message],
    model: &Model,
    options: CompactionOptions,
    api_key: &str,
) -> Result<CompactionResult> {
    // Identify turn boundaries — each assistant message starts a new turn.
    let turn_starts: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, m)| m.is_assistant())
        .map(|(i, _)| i)
        .collect();

    let keep = options.keep_recent_turns;
    let split_idx = if turn_starts.len() > keep {
        turn_starts[turn_starts.len() - keep]
    } else {
        0
    };

    let old_messages = &messages[..split_idx];
    let recent_messages = &messages[split_idx..];

    let tokens_before = crate::context::estimate_tokens(
        &serde_json::to_string(&messages).unwrap_or_default(),
    );

    // Build the compaction prompt.
    let old_json = serde_json::to_string(old_messages).unwrap_or_default();
    let mut prompt = format!(
        "You are a conversation compaction assistant. Create a concise summary of the \
         conversation so far that preserves all critical information needed to continue.\n\n\
         Summarize the following conversation messages. Your summary MUST preserve:\n\
         1. The user's original request (quote it verbatim)\n\
         2. The current goal and what has been accomplished so far\n\
         3. Files that were read, written, or edited (list all paths)\n\
         4. Tests that were added and commands that were run\n\
         5. Work remaining: next steps and any blockers\n\
         6. Things that were tried and FAILED (critical — must not retry these)\n\
         7. Things the user explicitly said NOT to do\n\n\
         Be concise but complete. Do not lose any actionable information.\n\n\
         <conversation>\n{old_json}\n</conversation>"
    );

    if let Some(ref instructions) = options.custom_instructions {
        prompt.push_str(&format!("\n\nAdditional instructions: {instructions}"));
    }

    // Send to the model.
    let context = imp_llm::Context {
        messages: vec![Message::user(&prompt)],
    };
    let request_options = RequestOptions {
        system_prompt: "You are a precise conversation summarizer.".into(),
        max_tokens: Some(4096),
        ..Default::default()
    };

    let mut stream = model.provider.stream(model, context, request_options, api_key);
    let mut summary_parts: Vec<String> = Vec::new();

    while let Some(event_result) = stream.next().await {
        match event_result {
            Ok(StreamEvent::TextDelta { text }) => {
                summary_parts.push(text);
            }
            Ok(StreamEvent::Error { error }) => {
                return Err(crate::error::Error::Tool(format!(
                    "Compaction LLM error: {error}"
                )));
            }
            Ok(_) => {}
            Err(e) => {
                return Err(crate::error::Error::Llm(e));
            }
        }
    }

    let summary = summary_parts.concat();
    if summary.is_empty() {
        return Err(crate::error::Error::Tool(
            "Compaction produced empty summary".into(),
        ));
    }

    let first_kept_id = find_first_message_id(recent_messages);

    let tokens_after = crate::context::estimate_tokens(&summary)
        + crate::context::estimate_tokens(
            &serde_json::to_string(recent_messages).unwrap_or_default(),
        );

    Ok(CompactionResult {
        summary,
        first_kept_id,
        tokens_before,
        tokens_after,
    })
}

/// Derive a stable identifier from the first message in a slice.
fn find_first_message_id(messages: &[Message]) -> String {
    match messages.first() {
        Some(Message::ToolResult(tr)) => tr.tool_call_id.clone(),
        Some(Message::Assistant(a)) => {
            for block in &a.content {
                if let ContentBlock::ToolCall { id, .. } = block {
                    return id.clone();
                }
            }
            format!("assistant_{}", a.timestamp)
        }
        Some(Message::User(u)) => format!("user_{}", u.timestamp),
        None => "unknown".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;
    use std::sync::Arc;

    use async_trait::async_trait;
    use futures_core::Stream;
    use imp_llm::model::{Capabilities, ModelMeta, ModelPricing};
    use imp_llm::provider::Provider;
    use imp_llm::{AssistantMessage, StopReason, ToolResultMessage, Usage};
    use tokio::sync::Mutex;

    // -- helpers --

    fn make_user(text: &str) -> Message {
        Message::user(text)
    }

    fn make_assistant_tool_call(
        call_id: &str,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Message {
        Message::Assistant(AssistantMessage {
            content: vec![ContentBlock::ToolCall {
                id: call_id.into(),
                name: tool_name.into(),
                arguments: args,
            }],
            usage: None,
            stop_reason: StopReason::ToolUse,
            timestamp: 1000,
        })
    }

    fn make_assistant_text(text: &str) -> Message {
        Message::Assistant(AssistantMessage {
            content: vec![ContentBlock::Text {
                text: text.into(),
            }],
            usage: None,
            stop_reason: StopReason::EndTurn,
            timestamp: 2000,
        })
    }

    fn make_tool_result(call_id: &str, tool_name: &str, output: &str) -> Message {
        Message::ToolResult(ToolResultMessage {
            tool_call_id: call_id.into(),
            tool_name: tool_name.into(),
            content: vec![ContentBlock::Text {
                text: output.into(),
            }],
            is_error: false,
            details: serde_json::Value::Null,
            timestamp: 1000,
        })
    }

    /// Mock provider that returns a pre-programmed summary.
    struct MockCompactionProvider {
        response_text: Mutex<String>,
    }

    impl MockCompactionProvider {
        fn new(text: &str) -> Self {
            Self {
                response_text: Mutex::new(text.into()),
            }
        }
    }

    #[async_trait]
    impl Provider for MockCompactionProvider {
        fn stream(
            &self,
            _model: &Model,
            _context: imp_llm::Context,
            _options: RequestOptions,
            _api_key: &str,
        ) -> Pin<Box<dyn Stream<Item = imp_llm::Result<StreamEvent>> + Send>> {
            let text = self
                .response_text
                .try_lock()
                .expect("mock lock")
                .clone();
            let events = vec![
                StreamEvent::MessageStart {
                    model: "mock".into(),
                },
                StreamEvent::TextDelta { text: text.clone() },
                StreamEvent::MessageEnd {
                    message: AssistantMessage {
                        content: vec![ContentBlock::Text { text }],
                        usage: Some(Usage {
                            input_tokens: 500,
                            output_tokens: 100,
                            cache_read_tokens: 0,
                            cache_write_tokens: 0,
                        }),
                        stop_reason: StopReason::EndTurn,
                        timestamp: 3000,
                    },
                },
            ];
            Box::pin(futures::stream::iter(events.into_iter().map(Ok)))
        }

        async fn resolve_auth(
            &self,
            _auth: &imp_llm::auth::AuthStore,
        ) -> imp_llm::Result<imp_llm::auth::ApiKey> {
            Ok("mock-key".into())
        }

        fn id(&self) -> &str {
            "mock"
        }

        fn models(&self) -> &[ModelMeta] {
            &[]
        }
    }

    fn mock_model(summary: &str) -> Model {
        Model {
            meta: ModelMeta {
                id: "mock-model".into(),
                provider: "mock".into(),
                name: "Mock".into(),
                context_window: 200_000,
                max_output_tokens: 4096,
                pricing: ModelPricing::default(),
                capabilities: Capabilities::default(),
            },
            provider: Arc::new(MockCompactionProvider::new(summary)),
        }
    }

    // -- tests --

    #[tokio::test]
    async fn compact_returns_mock_summary() {
        let summary_text = "## Summary\nUser asked to fix bug #42. \
                            Files modified: src/main.rs, src/lib.rs. \
                            Tried approach A (failed: type mismatch). \
                            Next: try approach B.";

        let model = mock_model(summary_text);

        // Build a conversation with 6 turns, keep last 2.
        let mut messages = vec![make_user("Fix bug #42 in src/main.rs")];
        for i in 0..6 {
            let cid = format!("call_{i}");
            messages.push(make_assistant_tool_call(
                &cid,
                "read_file",
                serde_json::json!({"path": format!("src/file_{i}.rs")}),
            ));
            messages.push(make_tool_result(
                &cid,
                "read_file",
                &format!("// contents of file {i}"),
            ));
        }

        let options = CompactionOptions {
            keep_recent_turns: 2,
            ..Default::default()
        };

        let result = compact(&messages, &model, options, "test-key").await.unwrap();

        assert_eq!(result.summary, summary_text);
        assert!(result.tokens_before > 0);
        assert!(result.tokens_after > 0);
        assert!(
            result.tokens_after < result.tokens_before,
            "compacted output ({}) should be smaller than input ({})",
            result.tokens_after,
            result.tokens_before
        );
    }

    #[tokio::test]
    async fn compact_preserves_recent_turns() {
        let model = mock_model("Summary of old turns.");

        // 5 turns total.
        let mut messages = vec![make_user("initial prompt")];
        for i in 0..5 {
            let cid = format!("call_{i}");
            messages.push(make_assistant_tool_call(
                &cid,
                "bash",
                serde_json::json!({"cmd": format!("cmd_{i}")}),
            ));
            messages.push(make_tool_result(&cid, "bash", &format!("out_{i}")));
        }

        let options = CompactionOptions {
            keep_recent_turns: 3,
            ..Default::default()
        };

        let result = compact(&messages, &model, options, "test-key").await.unwrap();

        // The first_kept_id should correspond to turn 2 (the 3rd-from-last turn).
        // Turn 2 starts at assistant message for call_2.
        assert_eq!(result.first_kept_id, "call_2");
    }

    #[tokio::test]
    async fn compact_replaces_old_messages_with_summary() {
        let summary = "Compacted: user asked to refactor, touched 3 files, tests pass.";
        let model = mock_model(summary);

        let mut messages = vec![make_user("Refactor the module")];
        for i in 0..10 {
            let cid = format!("c{i}");
            messages.push(make_assistant_tool_call(
                &cid,
                "edit",
                serde_json::json!({"file": format!("f{i}.rs")}),
            ));
            messages.push(make_tool_result(&cid, "edit", "ok"));
        }
        messages.push(make_assistant_text("Done refactoring"));

        let options = CompactionOptions {
            keep_recent_turns: 3,
            ..Default::default()
        };

        let result = compact(&messages, &model, options, "test-key").await.unwrap();

        // Simulate what the agent loop would do: replace old messages with summary + keep recent.
        let turn_starts: Vec<usize> = messages
            .iter()
            .enumerate()
            .filter(|(_, m)| m.is_assistant())
            .map(|(i, _)| i)
            .collect();
        let split = turn_starts[turn_starts.len() - 3];

        let mut new_messages = vec![
            make_user(&result.summary),
        ];
        new_messages.extend_from_slice(&messages[split..]);

        // New conversation should be much shorter.
        assert!(new_messages.len() < messages.len());
        // First message is the summary.
        if let Message::User(u) = &new_messages[0] {
            if let ContentBlock::Text { text } = &u.content[0] {
                assert_eq!(text, summary);
            }
        }
        // Recent turns preserved.
        assert!(new_messages.len() >= 4); // summary + 3 turns (each = assistant + tool_result) + final text
    }
}
