use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::{future::join_all, StreamExt};
use imp_llm::{
    AssistantMessage, ContentBlock, Context, Cost, Message, Model, RequestOptions, StopReason,
    StreamEvent, ThinkingLevel, Usage,
};
use tokio::sync::mpsc;

use crate::error::Result;
use crate::hooks::{HookEvent, HookRunner};
use crate::roles::Role;
use crate::tools::ToolRegistry;

/// Events emitted by the agent during execution.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    AgentStart {
        model: String,
        timestamp: u64,
    },
    AgentEnd {
        usage: Usage,
        cost: Cost,
    },
    TurnStart {
        index: u32,
    },
    TurnEnd {
        index: u32,
        message: AssistantMessage,
    },
    MessageStart {
        message: Message,
    },
    MessageDelta {
        delta: StreamEvent,
    },
    MessageEnd {
        message: Message,
    },
    ToolExecutionStart {
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
    },
    ToolExecutionEnd {
        tool_call_id: String,
        result: imp_llm::ToolResultMessage,
    },
    CompactionStart,
    CompactionEnd {
        summary: String,
    },
    Error {
        error: String,
    },
}

/// Commands sent to the agent (from UI or orchestrator).
#[derive(Debug, Clone)]
pub enum AgentCommand {
    Cancel,
    Steer(String),
    FollowUp(String),
}

/// The core agent — runs the ReAct loop (reason, act, observe).
pub struct Agent {
    pub model: Model,
    pub thinking_level: ThinkingLevel,
    pub tools: ToolRegistry,
    pub messages: Vec<Message>,
    pub system_prompt: String,
    pub cwd: PathBuf,
    pub max_turns: u32,
    pub role: Option<Role>,
    pub hooks: HookRunner,
    pub api_key: String,
    pub ui: Arc<dyn crate::ui::UserInterface>,

    event_tx: mpsc::Sender<AgentEvent>,
    command_rx: mpsc::Receiver<AgentCommand>,
}

/// Handle for controlling the agent from outside.
pub struct AgentHandle {
    pub event_rx: mpsc::Receiver<AgentEvent>,
    pub command_tx: mpsc::Sender<AgentCommand>,
}

impl Agent {
    pub fn new(model: Model, cwd: PathBuf) -> (Self, AgentHandle) {
        let (event_tx, event_rx) = mpsc::channel(256);
        let (command_tx, command_rx) = mpsc::channel(32);

        let agent = Self {
            model,
            thinking_level: ThinkingLevel::Medium,
            tools: ToolRegistry::new(),
            messages: Vec::new(),
            system_prompt: String::new(),
            cwd,
            max_turns: 50,
            role: None,
            hooks: HookRunner::new(),
            api_key: String::new(),
            ui: Arc::new(crate::ui::NullInterface),
            event_tx,
            command_rx,
        };

        let handle = AgentHandle {
            event_rx,
            command_tx,
        };

        (agent, handle)
    }

    /// Run the agent loop with an initial prompt.
    pub async fn run(&mut self, prompt: String) -> Result<()> {
        self.emit(AgentEvent::AgentStart {
            model: self.model.meta.id.clone(),
            timestamp: imp_llm::now(),
        })
        .await;

        self.messages.push(Message::user(&prompt));

        let mut turn: u32 = 0;
        let mut total_usage = Usage::default();
        let mut cancelled = false;

        loop {
            if turn >= self.max_turns {
                self.emit(AgentEvent::Error {
                    error: format!("Max turns exceeded ({})", self.max_turns),
                })
                .await;
                let cost = total_usage.cost(&self.model.meta.pricing);
                self.emit(AgentEvent::AgentEnd {
                    usage: total_usage,
                    cost,
                })
                .await;
                return Err(crate::error::Error::MaxTurns(self.max_turns));
            }

            // Check for commands between turns (non-blocking)
            while let Ok(cmd) = self.command_rx.try_recv() {
                match cmd {
                    AgentCommand::Cancel => {
                        cancelled = true;
                        break;
                    }
                    AgentCommand::Steer(msg) => {
                        self.messages.push(Message::user(&msg));
                    }
                    AgentCommand::FollowUp(_) => { /* queue for after loop */ }
                }
            }

            if cancelled {
                break;
            }

            self.emit(AgentEvent::TurnStart { index: turn }).await;

            let usage = crate::context::context_usage(&self.messages, &self.model);
            if usage.ratio >= 0.6 {
                crate::context::mask_observations(&mut self.messages, 10);
                self.hooks
                    .fire(&HookEvent::OnContextThreshold { ratio: usage.ratio })
                    .await;
            }

            if usage.ratio >= 0.8 {
                self.emit(AgentEvent::CompactionStart).await;
                match crate::compaction::compact(
                    &self.messages,
                    &self.model,
                    Default::default(),
                    &self.api_key,
                )
                .await
                {
                    Ok(result) => {
                        replace_with_compacted_messages(
                            &mut self.messages,
                            &result.summary,
                            &result.first_kept_id,
                        );
                        self.emit(AgentEvent::CompactionEnd {
                            summary: result.summary,
                        })
                        .await;
                    }
                    Err(error) => {
                        self.emit(AgentEvent::Error {
                            error: format!("Compaction failed: {error}"),
                        })
                        .await;
                    }
                }
            }

            // Build context and options for the LLM
            let context = Context {
                messages: self.messages.clone(),
            };

            let options = RequestOptions {
                thinking_level: self.thinking_level,
                max_tokens: Some(self.model.meta.max_output_tokens),
                temperature: None,
                system_prompt: self.system_prompt.clone(),
                tools: self.tools.definitions(),
                cache_options: Default::default(),
            };

            self.hooks.fire(&HookEvent::BeforeLlmCall).await;

            // Stream the LLM response
            let mut stream =
                self.model
                    .provider
                    .stream(&self.model, context, options, &self.api_key);

            let mut text_parts: Vec<String> = Vec::new();
            let mut thinking_parts: Vec<String> = Vec::new();
            let mut tool_calls: Vec<(String, String, serde_json::Value)> = Vec::new();
            let mut assistant_msg: Option<AssistantMessage> = None;

            while let Some(event_result) = stream.next().await {
                // Check for cancel during streaming
                if let Ok(AgentCommand::Cancel) = self.command_rx.try_recv() {
                    cancelled = true;
                    break;
                }

                match event_result {
                    Ok(event) => {
                        // Forward as delta
                        self.emit(AgentEvent::MessageDelta {
                            delta: event.clone(),
                        })
                        .await;

                        match event {
                            StreamEvent::TextDelta { text } => {
                                text_parts.push(text);
                            }
                            StreamEvent::ThinkingDelta { text } => {
                                thinking_parts.push(text);
                            }
                            StreamEvent::ToolCall {
                                id,
                                name,
                                arguments,
                            } => {
                                tool_calls.push((id, name, arguments));
                            }
                            StreamEvent::MessageEnd { message } => {
                                if let Some(ref usage) = message.usage {
                                    total_usage.add(usage);
                                }
                                assistant_msg = Some(message);
                            }
                            StreamEvent::MessageStart { .. } => {}
                            StreamEvent::Error { error } => {
                                self.emit(AgentEvent::Error {
                                    error: error.clone(),
                                })
                                .await;
                                // Build a minimal error message to push
                                let err_msg = AssistantMessage {
                                    content: vec![ContentBlock::Text { text: error }],
                                    usage: None,
                                    stop_reason: StopReason::Error("Stream error".to_string()),
                                    timestamp: imp_llm::now(),
                                };
                                self.messages.push(Message::Assistant(err_msg.clone()));
                                self.emit(AgentEvent::TurnEnd {
                                    index: turn,
                                    message: err_msg,
                                })
                                .await;
                                let cost = total_usage.cost(&self.model.meta.pricing);
                                self.emit(AgentEvent::AgentEnd {
                                    usage: total_usage,
                                    cost,
                                })
                                .await;
                                return Err(crate::error::Error::Llm(imp_llm::Error::Provider(
                                    "Stream error".to_string(),
                                )));
                            }
                        }
                    }
                    Err(e) => {
                        self.emit(AgentEvent::Error {
                            error: e.to_string(),
                        })
                        .await;
                        let cost = total_usage.cost(&self.model.meta.pricing);
                        self.emit(AgentEvent::AgentEnd {
                            usage: total_usage,
                            cost,
                        })
                        .await;
                        return Err(e.into());
                    }
                }
            }

            if cancelled {
                // Emit TurnEnd with whatever we have so far
                let partial = assistant_msg.unwrap_or_else(|| {
                    build_assistant_message(&text_parts, &thinking_parts, &tool_calls, None)
                });
                self.messages.push(Message::Assistant(partial.clone()));
                self.emit(AgentEvent::TurnEnd {
                    index: turn,
                    message: partial,
                })
                .await;
                break;
            }

            // Use the MessageEnd message if provided, otherwise build from accumulated parts
            let msg = assistant_msg.unwrap_or_else(|| {
                build_assistant_message(&text_parts, &thinking_parts, &tool_calls, None)
            });

            self.messages.push(Message::Assistant(msg.clone()));

            if tool_calls.is_empty() {
                // No tool calls — the model is done
                self.emit(AgentEvent::TurnEnd {
                    index: turn,
                    message: msg,
                })
                .await;
                break;
            }

            // Execute tool calls
            let results = self.execute_tools(tool_calls).await;

            // Push tool results onto messages
            for result in &results {
                self.messages.push(Message::ToolResult(result.clone()));
            }

            self.emit(AgentEvent::TurnEnd {
                index: turn,
                message: msg,
            })
            .await;

            turn += 1;
        }

        let cost = total_usage.cost(&self.model.meta.pricing);
        self.emit(AgentEvent::AgentEnd {
            usage: total_usage,
            cost,
        })
        .await;

        if cancelled {
            return Err(crate::error::Error::Cancelled);
        }

        Ok(())
    }

    async fn emit(&self, event: AgentEvent) {
        let _ = self.event_tx.send(event).await;
    }

    /// Execute tool calls from a single assistant message.
    async fn execute_tools(
        &self,
        calls: Vec<(String, String, serde_json::Value)>,
    ) -> Vec<imp_llm::ToolResultMessage> {
        let mut readonly = Vec::new();
        let mut mutable = Vec::new();

        for (index, (id, name, args)) in calls.into_iter().enumerate() {
            if self.tools.get(&name).is_some_and(|tool| tool.is_readonly()) {
                readonly.push((index, id, name, args));
            } else {
                mutable.push((index, id, name, args));
            }
        }

        let mut results = join_all(readonly.into_iter().map(
            |(index, id, name, args)| async move {
                let result = self.execute_one_tool(&id, &name, args).await;
                (index, result)
            },
        ))
        .await;

        for (index, id, name, args) in mutable {
            let result = self.execute_one_tool(&id, &name, args).await;
            results.push((index, result));
        }

        results.sort_by_key(|(index, _)| *index);
        results.into_iter().map(|(_, result)| result).collect()
    }

    async fn execute_one_tool(
        &self,
        call_id: &str,
        tool_name: &str,
        args: serde_json::Value,
    ) -> imp_llm::ToolResultMessage {
        self.emit(AgentEvent::ToolExecutionStart {
            tool_call_id: call_id.to_string(),
            tool_name: tool_name.to_string(),
            args: args.clone(),
        })
        .await;

        let before_results = self
            .hooks
            .fire(&HookEvent::BeforeToolCall {
                tool_name,
                args: &args,
            })
            .await;

        if let Some(blocking_result) = before_results.into_iter().find(|result| result.block) {
            let reason = blocking_result
                .reason
                .unwrap_or_else(|| format!("Tool call blocked by hook: {tool_name}"));
            let result =
                crate::tools::ToolOutput::error(reason).into_tool_result(call_id, tool_name);
            self.emit(AgentEvent::ToolExecutionEnd {
                tool_call_id: call_id.to_string(),
                result: result.clone(),
            })
            .await;
            return result;
        }

        let mut result = match self.tools.get(tool_name) {
            Some(tool) => {
                let (update_tx, _update_rx) = mpsc::channel(64);
                let ctx = crate::tools::ToolContext {
                    cwd: self.cwd.clone(),
                    cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    update_tx,
                    ui: self.ui.clone(),
                };
                match tool.execute(call_id, args.clone(), ctx).await {
                    Ok(output) => output.into_tool_result(call_id, tool_name),
                    Err(e) => crate::tools::ToolOutput::error(e.to_string())
                        .into_tool_result(call_id, tool_name),
                }
            }
            None => crate::tools::ToolOutput::error(format!("Unknown tool: {tool_name}"))
                .into_tool_result(call_id, tool_name),
        };

        let after_results = self
            .hooks
            .fire(&HookEvent::AfterToolCall {
                tool_name,
                result: &result,
            })
            .await;

        if let Some(modified_content) = after_results
            .into_iter()
            .filter_map(|hook_result| hook_result.modified_content)
            .next_back()
        {
            result.content = modified_content;
        }

        if !result.is_error && matches!(tool_name, "write" | "edit" | "multi_edit") {
            if let Some(path) = extract_file_path(self.cwd.as_path(), &args) {
                self.hooks
                    .fire(&HookEvent::AfterFileWrite {
                        file: path.as_path(),
                    })
                    .await;
            }
        }

        self.emit(AgentEvent::ToolExecutionEnd {
            tool_call_id: call_id.to_string(),
            result: result.clone(),
        })
        .await;

        result
    }
}

fn replace_with_compacted_messages(
    messages: &mut Vec<Message>,
    summary: &str,
    first_kept_id: &str,
) {
    let keep_from = messages
        .iter()
        .position(|message| message_matches_compaction_id(message, first_kept_id))
        .unwrap_or(messages.len());

    let mut compacted = vec![Message::user(summary)];
    compacted.extend(messages.drain(keep_from..));
    *messages = compacted;
}

fn message_matches_compaction_id(message: &Message, first_kept_id: &str) -> bool {
    match message {
        Message::ToolResult(result) => result.tool_call_id == first_kept_id,
        Message::Assistant(assistant) => {
            assistant.content.iter().any(
                |block| matches!(block, ContentBlock::ToolCall { id, .. } if id == first_kept_id),
            ) || format!("assistant_{}", assistant.timestamp) == first_kept_id
        }
        Message::User(user) => format!("user_{}", user.timestamp) == first_kept_id,
    }
}

/// Build an AssistantMessage from accumulated stream parts.
fn build_assistant_message(
    text_parts: &[String],
    thinking_parts: &[String],
    tool_calls: &[(String, String, serde_json::Value)],
    usage: Option<Usage>,
) -> AssistantMessage {
    let mut content = Vec::new();

    if !thinking_parts.is_empty() {
        content.push(ContentBlock::Thinking {
            text: thinking_parts.concat(),
        });
    }

    if !text_parts.is_empty() {
        content.push(ContentBlock::Text {
            text: text_parts.concat(),
        });
    }

    for (id, name, arguments) in tool_calls {
        content.push(ContentBlock::ToolCall {
            id: id.clone(),
            name: name.clone(),
            arguments: arguments.clone(),
        });
    }

    let stop_reason = if tool_calls.is_empty() {
        StopReason::EndTurn
    } else {
        StopReason::ToolUse
    };

    AssistantMessage {
        content,
        usage,
        stop_reason,
        timestamp: imp_llm::now(),
    }
}

fn extract_file_path(cwd: &Path, args: &serde_json::Value) -> Option<PathBuf> {
    let raw_path = args.get("path")?.as_str()?;
    if raw_path.is_empty() {
        return None;
    }

    let path = PathBuf::from(raw_path);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(cwd.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex as StdMutex,
    };
    use std::time::Duration;

    use async_trait::async_trait;
    use futures_core::Stream;
    use imp_llm::auth::{ApiKey, AuthStore};
    use imp_llm::model::{Capabilities, ModelMeta, ModelPricing};
    use imp_llm::provider::Provider;
    use tokio::sync::{Mutex, Notify};

    /// A mock provider that returns pre-programmed responses.
    /// Each call to `stream()` pops the next response from the queue.
    struct MockProvider {
        responses: Mutex<Vec<Vec<StreamEvent>>>,
    }

    impl MockProvider {
        fn new(responses: Vec<Vec<StreamEvent>>) -> Self {
            Self {
                responses: Mutex::new(responses),
            }
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn stream(
            &self,
            _model: &Model,
            _context: Context,
            _options: RequestOptions,
            _api_key: &str,
        ) -> Pin<Box<dyn Stream<Item = imp_llm::Result<StreamEvent>> + Send>> {
            // We need to get the next response synchronously. Use try_lock since
            // tests are single-threaded per agent run.
            let mut responses = self.responses.try_lock().expect("MockProvider lock");
            let events = if responses.is_empty() {
                vec![StreamEvent::Error {
                    error: "No more mock responses".to_string(),
                }]
            } else {
                responses.remove(0)
            };
            let stream = futures::stream::iter(events.into_iter().map(Ok));
            Box::pin(stream)
        }

        async fn resolve_auth(&self, _auth: &AuthStore) -> imp_llm::Result<ApiKey> {
            Ok("mock-key".to_string())
        }

        fn id(&self) -> &str {
            "mock"
        }

        fn models(&self) -> &[ModelMeta] {
            &[]
        }
    }

    fn test_model(provider: Arc<dyn Provider>) -> Model {
        test_model_with_context_window(provider, 200_000)
    }

    fn test_model_with_context_window(provider: Arc<dyn Provider>, context_window: u32) -> Model {
        Model {
            meta: ModelMeta {
                id: "test-model".to_string(),
                provider: "mock".to_string(),
                name: "Test Model".to_string(),
                context_window,
                max_output_tokens: 16_384,
                pricing: ModelPricing {
                    input_per_mtok: 3.0,
                    output_per_mtok: 15.0,
                    cache_read_per_mtok: 0.3,
                    cache_write_per_mtok: 3.75,
                },
                capabilities: Capabilities {
                    reasoning: true,
                    images: false,
                    tool_use: true,
                },
            },
            provider,
        }
    }

    fn text_response(text: &str, input_tokens: u32, output_tokens: u32) -> Vec<StreamEvent> {
        vec![
            StreamEvent::MessageStart {
                model: "test-model".to_string(),
            },
            StreamEvent::TextDelta {
                text: text.to_string(),
            },
            StreamEvent::MessageEnd {
                message: AssistantMessage {
                    content: vec![ContentBlock::Text {
                        text: text.to_string(),
                    }],
                    usage: Some(Usage {
                        input_tokens,
                        output_tokens,
                        cache_read_tokens: 0,
                        cache_write_tokens: 0,
                    }),
                    stop_reason: StopReason::EndTurn,
                    timestamp: 1000,
                },
            },
        ]
    }

    fn tool_call_response(
        call_id: &str,
        tool_name: &str,
        args: serde_json::Value,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Vec<StreamEvent> {
        vec![
            StreamEvent::MessageStart {
                model: "test-model".to_string(),
            },
            StreamEvent::ToolCall {
                id: call_id.to_string(),
                name: tool_name.to_string(),
                arguments: args.clone(),
            },
            StreamEvent::MessageEnd {
                message: AssistantMessage {
                    content: vec![ContentBlock::ToolCall {
                        id: call_id.to_string(),
                        name: tool_name.to_string(),
                        arguments: args,
                    }],
                    usage: Some(Usage {
                        input_tokens,
                        output_tokens,
                        cache_read_tokens: 0,
                        cache_write_tokens: 0,
                    }),
                    stop_reason: StopReason::ToolUse,
                    timestamp: 1000,
                },
            },
        ]
    }

    fn multi_tool_call_response(
        calls: &[(&str, &str, serde_json::Value)],
        input_tokens: u32,
        output_tokens: u32,
    ) -> Vec<StreamEvent> {
        let mut events = vec![StreamEvent::MessageStart {
            model: "test-model".to_string(),
        }];

        let mut content = Vec::new();
        for (id, name, args) in calls {
            events.push(StreamEvent::ToolCall {
                id: id.to_string(),
                name: name.to_string(),
                arguments: args.clone(),
            });
            content.push(ContentBlock::ToolCall {
                id: id.to_string(),
                name: name.to_string(),
                arguments: args.clone(),
            });
        }

        events.push(StreamEvent::MessageEnd {
            message: AssistantMessage {
                content,
                usage: Some(Usage {
                    input_tokens,
                    output_tokens,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0,
                }),
                stop_reason: StopReason::ToolUse,
                timestamp: 1000,
            },
        });

        events
    }

    fn make_assistant_tool_call(
        call_id: &str,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Message {
        Message::Assistant(AssistantMessage {
            content: vec![ContentBlock::ToolCall {
                id: call_id.to_string(),
                name: tool_name.to_string(),
                arguments: args,
            }],
            usage: None,
            stop_reason: StopReason::ToolUse,
            timestamp: imp_llm::now(),
        })
    }

    fn make_tool_result(call_id: &str, tool_name: &str, output: &str) -> Message {
        Message::ToolResult(imp_llm::ToolResultMessage {
            tool_call_id: call_id.to_string(),
            tool_name: tool_name.to_string(),
            content: vec![ContentBlock::Text {
                text: output.to_string(),
            }],
            is_error: false,
            details: serde_json::Value::Null,
            timestamp: imp_llm::now(),
        })
    }

    fn tool_result_text(message: &Message) -> Option<&str> {
        match message {
            Message::ToolResult(result) => result.content.iter().find_map(|block| match block {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            }),
            _ => None,
        }
    }

    /// A simple echo tool for testing.
    struct EchoTool;

    #[async_trait]
    impl crate::tools::Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn label(&self) -> &str {
            "Echo"
        }
        fn description(&self) -> &str {
            "Echoes back the input"
        }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" }
                },
                "required": ["text"]
            })
        }
        fn is_readonly(&self) -> bool {
            true
        }
        async fn execute(
            &self,
            _call_id: &str,
            params: serde_json::Value,
            _ctx: crate::tools::ToolContext,
        ) -> crate::error::Result<crate::tools::ToolOutput> {
            let text = params["text"].as_str().unwrap_or("no text");
            Ok(crate::tools::ToolOutput::text(format!("echo: {text}")))
        }
    }

    /// A mutable tool for testing write partitioning.
    #[allow(dead_code)]
    struct WriteTool;

    #[async_trait]
    impl crate::tools::Tool for WriteTool {
        fn name(&self) -> &str {
            "write"
        }
        fn label(&self) -> &str {
            "Write"
        }
        fn description(&self) -> &str {
            "Writes data"
        }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "data": { "type": "string" }
                },
                "required": ["data"]
            })
        }
        fn is_readonly(&self) -> bool {
            false
        }
        async fn execute(
            &self,
            _call_id: &str,
            params: serde_json::Value,
            _ctx: crate::tools::ToolContext,
        ) -> crate::error::Result<crate::tools::ToolOutput> {
            let data = params["data"].as_str().unwrap_or("no data");
            Ok(crate::tools::ToolOutput::text(format!("wrote: {data}")))
        }
    }

    struct ConcurrentReadonlyState {
        readonly_expected: usize,
        readonly_started: AtomicUsize,
        readonly_finished: AtomicUsize,
        mutable_observed_finished: AtomicUsize,
        log: StdMutex<Vec<String>>,
        notify: Notify,
    }

    impl ConcurrentReadonlyState {
        fn new(readonly_expected: usize) -> Self {
            Self {
                readonly_expected,
                readonly_started: AtomicUsize::new(0),
                readonly_finished: AtomicUsize::new(0),
                mutable_observed_finished: AtomicUsize::new(0),
                log: StdMutex::new(Vec::new()),
                notify: Notify::new(),
            }
        }

        fn record(&self, entry: impl Into<String>) {
            self.log
                .lock()
                .expect("concurrent log lock")
                .push(entry.into());
        }

        async fn wait_for_all_readonly_to_start(&self) {
            while self.readonly_started.load(Ordering::SeqCst) < self.readonly_expected {
                self.notify.notified().await;
            }
        }
    }

    struct CoordinatedReadonlyTool {
        name: &'static str,
        shared: Arc<ConcurrentReadonlyState>,
    }

    #[async_trait]
    impl crate::tools::Tool for CoordinatedReadonlyTool {
        fn name(&self) -> &str {
            self.name
        }
        fn label(&self) -> &str {
            self.name
        }
        fn description(&self) -> &str {
            "Read-only tool used to verify concurrent execution"
        }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" }
                },
                "required": ["text"]
            })
        }
        fn is_readonly(&self) -> bool {
            true
        }
        async fn execute(
            &self,
            _call_id: &str,
            params: serde_json::Value,
            _ctx: crate::tools::ToolContext,
        ) -> crate::error::Result<crate::tools::ToolOutput> {
            self.shared.record(format!("{}:start", self.name));
            self.shared.readonly_started.fetch_add(1, Ordering::SeqCst);
            self.shared.notify.notify_waiters();
            self.shared.wait_for_all_readonly_to_start().await;
            self.shared.record(format!("{}:end", self.name));
            self.shared.readonly_finished.fetch_add(1, Ordering::SeqCst);

            let text = params["text"].as_str().unwrap_or(self.name);
            Ok(crate::tools::ToolOutput::text(format!(
                "{}: {text}",
                self.name
            )))
        }
    }

    struct CoordinatedMutableTool {
        shared: Arc<ConcurrentReadonlyState>,
    }

    #[async_trait]
    impl crate::tools::Tool for CoordinatedMutableTool {
        fn name(&self) -> &str {
            "write_after_reads"
        }
        fn label(&self) -> &str {
            "Write After Reads"
        }
        fn description(&self) -> &str {
            "Mutable tool used to verify read-only tools finish first"
        }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "data": { "type": "string" }
                },
                "required": ["data"]
            })
        }
        fn is_readonly(&self) -> bool {
            false
        }
        async fn execute(
            &self,
            _call_id: &str,
            params: serde_json::Value,
            _ctx: crate::tools::ToolContext,
        ) -> crate::error::Result<crate::tools::ToolOutput> {
            let finished = self.shared.readonly_finished.load(Ordering::SeqCst);
            self.shared
                .mutable_observed_finished
                .store(finished, Ordering::SeqCst);
            self.shared.record("write_after_reads:start");

            let data = params["data"].as_str().unwrap_or("no data");
            Ok(crate::tools::ToolOutput::text(format!(
                "wrote after reads: {data}"
            )))
        }
    }

    /// Collect all events from the handle until the channel closes.
    async fn collect_events(mut handle: AgentHandle) -> Vec<AgentEvent> {
        let mut events = Vec::new();
        while let Some(event) = handle.event_rx.recv().await {
            events.push(event);
        }
        events
    }

    // ── Test 1: Simple text response ───────────────────────────────

    #[tokio::test]
    async fn agent_simple_text_response() {
        let provider = Arc::new(MockProvider::new(vec![text_response(
            "Hello, world!",
            100,
            20,
        )]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("Say hello".to_string()).await.unwrap();
        drop(agent); // close event channel

        let events = events_task.await.unwrap();

        // Verify event order: AgentStart → TurnStart → deltas → TurnEnd → AgentEnd
        assert!(matches!(events[0], AgentEvent::AgentStart { .. }));

        let turn_start = events
            .iter()
            .position(|e| matches!(e, AgentEvent::TurnStart { index: 0 }));
        assert!(turn_start.is_some());

        let turn_end = events
            .iter()
            .position(|e| matches!(e, AgentEvent::TurnEnd { index: 0, .. }));
        assert!(turn_end.is_some());
        assert!(turn_end.unwrap() > turn_start.unwrap());

        let agent_end = events
            .iter()
            .position(|e| matches!(e, AgentEvent::AgentEnd { .. }));
        assert!(agent_end.is_some());
        assert!(agent_end.unwrap() > turn_end.unwrap());

        // Verify usage
        if let AgentEvent::AgentEnd { usage, cost, .. } = &events[agent_end.unwrap()] {
            assert_eq!(usage.input_tokens, 100);
            assert_eq!(usage.output_tokens, 20);
            assert!(cost.total > 0.0);
        } else {
            panic!("Expected AgentEnd");
        }

        // Only one turn (no tool calls)
        let turn_starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AgentEvent::TurnStart { .. }))
            .collect();
        assert_eq!(turn_starts.len(), 1);
    }

    // ── Test 2: Single tool call → result → text response ──────────

    #[tokio::test]
    async fn agent_single_tool_call() {
        let provider = Arc::new(MockProvider::new(vec![
            // Turn 0: model calls echo tool
            tool_call_response(
                "call_1",
                "echo",
                serde_json::json!({"text": "hello"}),
                100,
                30,
            ),
            // Turn 1: model responds with text after seeing tool result
            text_response("The echo said: hello", 200, 25),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.tools.register(Arc::new(EchoTool));

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("Echo hello".to_string()).await.unwrap();
        drop(agent);

        let events = events_task.await.unwrap();

        // Should have 2 TurnStart events (turn 0 with tool, turn 1 with text)
        let turn_starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AgentEvent::TurnStart { .. }))
            .collect();
        assert_eq!(turn_starts.len(), 2);

        // Should have tool execution events
        let tool_starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AgentEvent::ToolExecutionStart { .. }))
            .collect();
        assert_eq!(tool_starts.len(), 1);

        let tool_ends: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AgentEvent::ToolExecutionEnd { .. }))
            .collect();
        assert_eq!(tool_ends.len(), 1);

        // Verify accumulated usage across turns (100 + 200 input, 30 + 25 output)
        if let Some(AgentEvent::AgentEnd { usage, .. }) = events
            .iter()
            .find(|e| matches!(e, AgentEvent::AgentEnd { .. }))
        {
            assert_eq!(usage.input_tokens, 300);
            assert_eq!(usage.output_tokens, 55);
        } else {
            panic!("Expected AgentEnd");
        }
    }

    // ── Test 3: Multiple tool calls → follow-up tool calls → done ──

    #[tokio::test]
    async fn agent_multiple_tool_calls() {
        let provider = Arc::new(MockProvider::new(vec![
            // Turn 0: model calls echo twice
            multi_tool_call_response(
                &[
                    ("call_1", "echo", serde_json::json!({"text": "first"})),
                    ("call_2", "echo", serde_json::json!({"text": "second"})),
                ],
                100,
                40,
            ),
            // Turn 1: model calls echo once more
            tool_call_response(
                "call_3",
                "echo",
                serde_json::json!({"text": "third"}),
                200,
                20,
            ),
            // Turn 2: model responds with final text
            text_response("All done!", 300, 10),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.tools.register(Arc::new(EchoTool));

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("Echo three things".to_string()).await.unwrap();
        drop(agent);

        let events = events_task.await.unwrap();

        // 3 turns
        let turn_starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AgentEvent::TurnStart { .. }))
            .collect();
        assert_eq!(turn_starts.len(), 3);

        // 3 tool executions total
        let tool_starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AgentEvent::ToolExecutionStart { .. }))
            .collect();
        assert_eq!(tool_starts.len(), 3);

        // Total usage: 100+200+300=600 input, 40+20+10=70 output
        if let Some(AgentEvent::AgentEnd { usage, .. }) = events
            .iter()
            .find(|e| matches!(e, AgentEvent::AgentEnd { .. }))
        {
            assert_eq!(usage.input_tokens, 600);
            assert_eq!(usage.output_tokens, 70);
        } else {
            panic!("Expected AgentEnd");
        }
    }

    // ── Test 4: Cancel command mid-run ─────────────────────────────

    #[tokio::test]
    async fn agent_cancel_mid_run() {
        let provider = Arc::new(MockProvider::new(vec![
            // Turn 0: tool call (agent will process this, then see Cancel before turn 1)
            tool_call_response(
                "call_1",
                "echo",
                serde_json::json!({"text": "hello"}),
                100,
                20,
            ),
            // Turn 1: this should never be reached
            text_response("Should not see this", 100, 20),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.tools.register(Arc::new(EchoTool));

        // Send cancel before the second turn
        handle.command_tx.send(AgentCommand::Cancel).await.unwrap();

        let events_task = tokio::spawn(collect_events(handle));
        let result = agent.run("Do something".to_string()).await;
        drop(agent);

        // Should return Cancelled error
        assert!(matches!(result, Err(crate::error::Error::Cancelled)));

        let events = events_task.await.unwrap();

        // Should have AgentEnd
        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::AgentEnd { .. })));

        // Should NOT have a second turn
        let turn_starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AgentEvent::TurnStart { .. }))
            .collect();
        assert!(turn_starts.len() <= 1);
    }

    // ── Test 5: Max turns exceeded ─────────────────────────────────

    #[tokio::test]
    async fn agent_max_turns_exceeded() {
        // Each turn will call a tool, forcing the loop to continue
        let responses: Vec<Vec<StreamEvent>> = (0..5)
            .map(|i| {
                tool_call_response(
                    &format!("call_{i}"),
                    "echo",
                    serde_json::json!({"text": format!("turn {i}")}),
                    50,
                    10,
                )
            })
            .collect();

        let provider = Arc::new(MockProvider::new(responses));
        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.tools.register(Arc::new(EchoTool));
        agent.max_turns = 3; // Will exceed after 3 turns (0, 1, 2)

        let events_task = tokio::spawn(collect_events(handle));
        let result = agent.run("Loop forever".to_string()).await;
        drop(agent);

        assert!(matches!(result, Err(crate::error::Error::MaxTurns(3))));

        let events = events_task.await.unwrap();

        // Should have error event about max turns
        let has_error = events
            .iter()
            .any(|e| matches!(e, AgentEvent::Error { error } if error.contains("Max turns")));
        assert!(has_error);

        // Should still have AgentEnd
        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::AgentEnd { .. })));

        // Verify usage accumulated for the 3 turns that did execute
        if let Some(AgentEvent::AgentEnd { usage, .. }) = events
            .iter()
            .find(|e| matches!(e, AgentEvent::AgentEnd { .. }))
        {
            assert_eq!(usage.input_tokens, 150); // 3 * 50
            assert_eq!(usage.output_tokens, 30); // 3 * 10
        }
    }

    // ── Test 6: Unknown tool → error result → model self-corrects ──

    #[tokio::test]
    async fn agent_unknown_tool_self_corrects() {
        let provider = Arc::new(MockProvider::new(vec![
            // Turn 0: model calls a tool that doesn't exist
            tool_call_response(
                "call_1",
                "nonexistent",
                serde_json::json!({"foo": "bar"}),
                100,
                20,
            ),
            // Turn 1: model self-corrects and responds with text
            text_response("Sorry, I used the wrong tool. Here's the answer.", 200, 30),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        // Deliberately NOT registering the "nonexistent" tool

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("Do something".to_string()).await.unwrap();
        drop(agent);

        let events = events_task.await.unwrap();

        // The tool execution should produce an error result
        let tool_end = events
            .iter()
            .find(|e| matches!(e, AgentEvent::ToolExecutionEnd { .. }));
        assert!(tool_end.is_some());
        if let Some(AgentEvent::ToolExecutionEnd { result, .. }) = tool_end {
            assert!(result.is_error);
            let text = result.content.iter().find_map(|c| {
                if let ContentBlock::Text { text } = c {
                    Some(text.as_str())
                } else {
                    None
                }
            });
            assert!(text.unwrap().contains("Unknown tool"));
        }

        // Model should have self-corrected in turn 1
        let turn_starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AgentEvent::TurnStart { .. }))
            .collect();
        assert_eq!(turn_starts.len(), 2);

        // Should complete successfully
        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::AgentEnd { .. })));
    }

    #[tokio::test]
    async fn agent_concurrent_readonly() {
        let shared = Arc::new(ConcurrentReadonlyState::new(3));
        let provider = Arc::new(MockProvider::new(vec![
            multi_tool_call_response(
                &[
                    ("call_ro_1", "echo_a", serde_json::json!({"text": "first"})),
                    (
                        "call_write",
                        "write_after_reads",
                        serde_json::json!({"data": "mutate"}),
                    ),
                    ("call_ro_2", "echo_b", serde_json::json!({"text": "second"})),
                    ("call_ro_3", "echo_c", serde_json::json!({"text": "third"})),
                ],
                100,
                40,
            ),
            text_response("All tools finished", 150, 20),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        drop(handle);

        agent.tools.register(Arc::new(CoordinatedReadonlyTool {
            name: "echo_a",
            shared: shared.clone(),
        }));
        agent.tools.register(Arc::new(CoordinatedReadonlyTool {
            name: "echo_b",
            shared: shared.clone(),
        }));
        agent.tools.register(Arc::new(CoordinatedReadonlyTool {
            name: "echo_c",
            shared: shared.clone(),
        }));
        agent.tools.register(Arc::new(CoordinatedMutableTool {
            shared: shared.clone(),
        }));

        tokio::time::timeout(
            Duration::from_millis(250),
            agent.run("Run all tools".to_string()),
        )
        .await
        .expect("read-only tools should not block each other")
        .expect("agent should complete successfully");

        let tool_result_ids: Vec<_> = agent
            .messages
            .iter()
            .filter_map(|message| match message {
                Message::ToolResult(result) => Some(result.tool_call_id.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            tool_result_ids,
            vec!["call_ro_1", "call_write", "call_ro_2", "call_ro_3"]
        );

        assert_eq!(shared.readonly_started.load(Ordering::SeqCst), 3);
        assert_eq!(shared.readonly_finished.load(Ordering::SeqCst), 3);
        assert_eq!(shared.mutable_observed_finished.load(Ordering::SeqCst), 3);

        let log = shared.log.lock().expect("concurrent log lock").clone();
        assert_eq!(
            log.last().map(String::as_str),
            Some("write_after_reads:start")
        );
    }

    // ── Event ordering validation ──────────────────────────────────

    #[tokio::test]
    async fn agent_event_ordering() {
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_1",
                "echo",
                serde_json::json!({"text": "hello"}),
                50,
                10,
            ),
            text_response("Done", 50, 10),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.tools.register(Arc::new(EchoTool));

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("test".to_string()).await.unwrap();
        drop(agent);

        let events = events_task.await.unwrap();

        // Extract event types in order
        let types: Vec<&str> = events
            .iter()
            .map(|e| match e {
                AgentEvent::AgentStart { .. } => "AgentStart",
                AgentEvent::AgentEnd { .. } => "AgentEnd",
                AgentEvent::TurnStart { .. } => "TurnStart",
                AgentEvent::TurnEnd { .. } => "TurnEnd",
                AgentEvent::MessageDelta { .. } => "MessageDelta",
                AgentEvent::ToolExecutionStart { .. } => "ToolExecStart",
                AgentEvent::ToolExecutionEnd { .. } => "ToolExecEnd",
                AgentEvent::Error { .. } => "Error",
                _ => "Other",
            })
            .collect();

        // Must start with AgentStart
        assert_eq!(types[0], "AgentStart");

        // Must end with AgentEnd
        assert_eq!(types[types.len() - 1], "AgentEnd");

        // TurnStart must come before TurnEnd for each turn
        let mut turn_start_indices: Vec<usize> = Vec::new();
        let mut turn_end_indices: Vec<usize> = Vec::new();
        for (i, t) in types.iter().enumerate() {
            if *t == "TurnStart" {
                turn_start_indices.push(i);
            }
            if *t == "TurnEnd" {
                turn_end_indices.push(i);
            }
        }
        assert_eq!(turn_start_indices.len(), 2);
        assert_eq!(turn_end_indices.len(), 2);
        for i in 0..turn_start_indices.len() {
            assert!(turn_start_indices[i] < turn_end_indices[i]);
        }

        // ToolExecStart must come before ToolExecEnd
        let tool_start = types.iter().position(|t| *t == "ToolExecStart");
        let tool_end = types.iter().position(|t| *t == "ToolExecEnd");
        assert!(tool_start.is_some());
        assert!(tool_end.is_some());
        assert!(tool_start.unwrap() < tool_end.unwrap());
    }

    #[tokio::test]
    async fn agent_fires_hooks() {
        let provider = Arc::new(MockProvider::new(vec![text_response("hooked", 100, 20)]));
        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        drop(handle);

        let hook_calls = Arc::new(AtomicUsize::new(0));
        let hook_calls_for_callback = hook_calls.clone();
        agent.hooks.register(crate::hooks::HookDefinition {
            event: "before_llm_call".to_string(),
            match_pattern: None,
            action: crate::hooks::HookAction::Callback(Arc::new(move |_event| {
                hook_calls_for_callback.fetch_add(1, Ordering::SeqCst);
                crate::hooks::HookResult::default()
            })),
            blocking: true,
            threshold: None,
        });

        agent.run("Run once".to_string()).await.unwrap();

        assert_eq!(hook_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn agent_context_masking() {
        let provider = Arc::new(MockProvider::new(vec![text_response("done", 100, 20)]));

        let mut seeded_messages = Vec::new();
        for index in 0..12 {
            let call_id = format!("call_{index}");
            seeded_messages.push(make_assistant_tool_call(
                &call_id,
                "read",
                serde_json::json!({"path": format!("src/file_{index}.rs")}),
            ));
            seeded_messages.push(make_tool_result(
                &call_id,
                "read",
                &format!("{}", "x".repeat(400)),
            ));
        }

        let mut usage_messages = seeded_messages.clone();
        usage_messages.push(Message::user("trigger masking"));
        let provisional_model = test_model(provider.clone());
        let usage = crate::context::context_usage(&usage_messages, &provisional_model);
        let context_window = ((usage.used as f64) / 0.7).ceil() as u32;

        let model = test_model_with_context_window(provider, context_window.max(1));
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        drop(handle);
        agent.messages = seeded_messages;

        agent.run("trigger masking".to_string()).await.unwrap();

        let masked = tool_result_text(&agent.messages[1]).expect("first tool result text");
        assert!(masked.starts_with("[Output omitted"));

        let recent_index = (10 * 2) + 1;
        let recent =
            tool_result_text(&agent.messages[recent_index]).expect("recent tool result text");
        let expected_recent = "x".repeat(400);
        assert_eq!(recent, expected_recent.as_str());
    }

    // ── Usage/cost accumulation ────────────────────────────────────

    #[tokio::test]
    async fn agent_usage_cost_accumulation() {
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_1",
                "echo",
                serde_json::json!({"text": "a"}),
                1_000_000, // 1M input tokens
                500_000,   // 500k output tokens
            ),
            text_response("done", 1_000_000, 500_000),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.tools.register(Arc::new(EchoTool));

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("test".to_string()).await.unwrap();
        drop(agent);

        let events = events_task.await.unwrap();

        if let Some(AgentEvent::AgentEnd { usage, cost }) = events
            .iter()
            .find(|e| matches!(e, AgentEvent::AgentEnd { .. }))
        {
            // 2M input, 1M output
            assert_eq!(usage.input_tokens, 2_000_000);
            assert_eq!(usage.output_tokens, 1_000_000);

            // Cost: 2M * $3/Mtok input = $6, 1M * $15/Mtok output = $15, total = $21
            assert!((cost.input - 6.0).abs() < 1e-10);
            assert!((cost.output - 15.0).abs() < 1e-10);
            assert!((cost.total - 21.0).abs() < 1e-10);
        } else {
            panic!("Expected AgentEnd");
        }
    }
}

// ── Integration tests: full ReAct cycle with real tools ─────────────

#[cfg(test)]
mod integration {
    use super::*;
    use std::path::PathBuf;
    use std::pin::Pin;
    use std::sync::Arc;

    use async_trait::async_trait;
    use futures_core::Stream;
    use imp_llm::auth::{ApiKey, AuthStore};
    use imp_llm::model::{Capabilities, ModelMeta, ModelPricing};
    use imp_llm::provider::Provider;
    use tokio::sync::Mutex;

    use crate::tools::{
        bash::BashTool, edit::EditTool, grep::GrepTool, read::ReadTool, write::WriteTool,
    };

    // ── Shared test helpers (duplicated from unit tests to keep modules independent) ──

    struct MockProvider {
        responses: Mutex<Vec<Vec<StreamEvent>>>,
    }

    impl MockProvider {
        fn new(responses: Vec<Vec<StreamEvent>>) -> Self {
            Self {
                responses: Mutex::new(responses),
            }
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn stream(
            &self,
            _model: &Model,
            _context: Context,
            _options: RequestOptions,
            _api_key: &str,
        ) -> Pin<Box<dyn Stream<Item = imp_llm::Result<StreamEvent>> + Send>> {
            let mut responses = self.responses.try_lock().expect("MockProvider lock");
            let events = if responses.is_empty() {
                vec![StreamEvent::Error {
                    error: "No more mock responses".to_string(),
                }]
            } else {
                responses.remove(0)
            };
            Box::pin(futures::stream::iter(events.into_iter().map(Ok)))
        }

        async fn resolve_auth(&self, _auth: &AuthStore) -> imp_llm::Result<ApiKey> {
            Ok("mock-key".to_string())
        }

        fn id(&self) -> &str {
            "mock"
        }

        fn models(&self) -> &[ModelMeta] {
            &[]
        }
    }

    fn test_model(provider: Arc<dyn Provider>) -> Model {
        Model {
            meta: ModelMeta {
                id: "test-model".to_string(),
                provider: "mock".to_string(),
                name: "Test Model".to_string(),
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
                    images: false,
                    tool_use: true,
                },
            },
            provider,
        }
    }

    fn text_response(text: &str, input_tokens: u32, output_tokens: u32) -> Vec<StreamEvent> {
        vec![
            StreamEvent::MessageStart {
                model: "test-model".to_string(),
            },
            StreamEvent::TextDelta {
                text: text.to_string(),
            },
            StreamEvent::MessageEnd {
                message: AssistantMessage {
                    content: vec![ContentBlock::Text {
                        text: text.to_string(),
                    }],
                    usage: Some(Usage {
                        input_tokens,
                        output_tokens,
                        cache_read_tokens: 0,
                        cache_write_tokens: 0,
                    }),
                    stop_reason: StopReason::EndTurn,
                    timestamp: 1000,
                },
            },
        ]
    }

    fn tool_call_response(
        call_id: &str,
        tool_name: &str,
        args: serde_json::Value,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Vec<StreamEvent> {
        vec![
            StreamEvent::MessageStart {
                model: "test-model".to_string(),
            },
            StreamEvent::ToolCall {
                id: call_id.to_string(),
                name: tool_name.to_string(),
                arguments: args.clone(),
            },
            StreamEvent::MessageEnd {
                message: AssistantMessage {
                    content: vec![ContentBlock::ToolCall {
                        id: call_id.to_string(),
                        name: tool_name.to_string(),
                        arguments: args,
                    }],
                    usage: Some(Usage {
                        input_tokens,
                        output_tokens,
                        cache_read_tokens: 0,
                        cache_write_tokens: 0,
                    }),
                    stop_reason: StopReason::ToolUse,
                    timestamp: 1000,
                },
            },
        ]
    }

    /// Create an agent pre-loaded with all native filesystem and shell tools.
    fn create_agent_with_tools(
        provider: Arc<dyn Provider>,
        cwd: PathBuf,
    ) -> (Agent, AgentHandle) {
        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, cwd);
        agent.tools.register(Arc::new(WriteTool));
        agent.tools.register(Arc::new(ReadTool));
        agent.tools.register(Arc::new(EditTool));
        agent.tools.register(Arc::new(GrepTool));
        agent.tools.register(Arc::new(BashTool));
        (agent, handle)
    }

    // ── Test 1: Write then read a file ─────────────────────────────

    #[tokio::test]
    async fn agent_reads_and_writes_file() {
        let tmp = tempfile::tempdir().unwrap();
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_write",
                "write",
                serde_json::json!({"path": "test.txt", "content": "hello world"}),
                100,
                20,
            ),
            tool_call_response(
                "call_read",
                "read",
                serde_json::json!({"path": "test.txt"}),
                100,
                20,
            ),
            text_response("The file contains: hello world", 100, 20),
        ]));

        let (mut agent, handle) = create_agent_with_tools(provider, tmp.path().to_path_buf());
        drop(handle);

        agent
            .run("Write and read a file".to_string())
            .await
            .unwrap();

        // File should exist on disk with correct content
        let on_disk = std::fs::read_to_string(tmp.path().join("test.txt")).unwrap();
        assert_eq!(on_disk, "hello world");

        // Read tool result should contain the file content
        let read_result = agent
            .messages
            .iter()
            .find_map(|m| match m {
                Message::ToolResult(r) if r.tool_call_id == "call_read" => Some(r),
                _ => None,
            })
            .expect("should have a read tool result");
        let read_text = read_result
            .content
            .iter()
            .find_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap();
        assert!(
            read_text.contains("hello world"),
            "read result should contain file content, got: {read_text}"
        );

        // 3 assistant messages = 3 turns (write, read, final text)
        let assistant_count = agent
            .messages
            .iter()
            .filter(|m| matches!(m, Message::Assistant(_)))
            .count();
        assert_eq!(assistant_count, 3);
    }

    // ── Test 2: Edit tool modifies a file ──────────────────────────

    #[tokio::test]
    async fn agent_edit_tool_modifies_file() {
        let tmp = tempfile::tempdir().unwrap();
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_write",
                "write",
                serde_json::json!({
                    "path": "src/main.rs",
                    "content": "fn main() {\n    println!(\"old\");\n}"
                }),
                100,
                20,
            ),
            tool_call_response(
                "call_edit",
                "edit",
                serde_json::json!({
                    "path": "src/main.rs",
                    "oldText": "old",
                    "newText": "new"
                }),
                100,
                20,
            ),
            tool_call_response(
                "call_read",
                "read",
                serde_json::json!({"path": "src/main.rs"}),
                100,
                20,
            ),
            text_response("Done", 100, 20),
        ]));

        let (mut agent, handle) = create_agent_with_tools(provider, tmp.path().to_path_buf());
        drop(handle);

        agent.run("Edit a file".to_string()).await.unwrap();

        // File should contain "new" not "old"
        let on_disk = std::fs::read_to_string(tmp.path().join("src/main.rs")).unwrap();
        assert!(on_disk.contains("new"), "file should contain 'new'");
        assert!(!on_disk.contains("old"), "file should not contain 'old'");

        // Edit tool result should include a diff
        let edit_result = agent
            .messages
            .iter()
            .find_map(|m| match m {
                Message::ToolResult(r) if r.tool_call_id == "call_edit" => Some(r),
                _ => None,
            })
            .expect("should have an edit tool result");
        let edit_text = edit_result
            .content
            .iter()
            .find_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap();
        assert!(
            edit_text.contains("---") || edit_text.contains("+++"),
            "edit result should include a diff, got: {edit_text}"
        );
    }

    // ── Test 3: Grep finds a pattern ───────────────────────────────

    #[tokio::test]
    async fn agent_grep_finds_pattern() {
        let tmp = tempfile::tempdir().unwrap();
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_write",
                "write",
                serde_json::json!({
                    "path": "search_me.txt",
                    "content": "line one\nunique_pattern_xyz here\nline three"
                }),
                100,
                20,
            ),
            tool_call_response(
                "call_grep",
                "grep",
                serde_json::json!({"pattern": "unique_pattern_xyz", "path": "."}),
                100,
                20,
            ),
            text_response("Found it!", 100, 20),
        ]));

        let (mut agent, handle) = create_agent_with_tools(provider, tmp.path().to_path_buf());
        drop(handle);

        agent
            .run("Search for a pattern".to_string())
            .await
            .unwrap();

        // Grep result should contain the file path and matching line
        let grep_result = agent
            .messages
            .iter()
            .find_map(|m| match m {
                Message::ToolResult(r) if r.tool_call_id == "call_grep" => Some(r),
                _ => None,
            })
            .expect("should have a grep tool result");
        let grep_text = grep_result
            .content
            .iter()
            .find_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap();
        assert!(
            grep_text.contains("search_me.txt"),
            "grep should show file path, got: {grep_text}"
        );
        assert!(
            grep_text.contains("unique_pattern_xyz"),
            "grep should show matching text, got: {grep_text}"
        );
    }

    // ── Test 4: Bash runs a command ────────────────────────────────

    #[tokio::test]
    async fn agent_bash_runs_command() {
        let tmp = tempfile::tempdir().unwrap();
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_bash",
                "bash",
                serde_json::json!({"command": "echo hello && echo world"}),
                100,
                20,
            ),
            text_response("Done", 100, 20),
        ]));

        let (mut agent, handle) = create_agent_with_tools(provider, tmp.path().to_path_buf());
        drop(handle);

        agent.run("Run a command".to_string()).await.unwrap();

        // Bash result should contain the command output
        let bash_result = agent
            .messages
            .iter()
            .find_map(|m| match m {
                Message::ToolResult(r) if r.tool_call_id == "call_bash" => Some(r),
                _ => None,
            })
            .expect("should have a bash tool result");
        let bash_text = bash_result
            .content
            .iter()
            .find_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap();
        assert!(
            bash_text.contains("hello"),
            "bash output should contain 'hello', got: {bash_text}"
        );
        assert!(
            bash_text.contains("world"),
            "bash output should contain 'world', got: {bash_text}"
        );

        // Details should include exit_code: 0
        assert_eq!(bash_result.details["exit_code"], 0);
    }

    // ── Test 5: Tool error → agent self-corrects ───────────────────

    #[tokio::test]
    async fn agent_handles_tool_error_gracefully() {
        let tmp = tempfile::tempdir().unwrap();
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_read",
                "read",
                serde_json::json!({"path": "nonexistent.txt"}),
                100,
                20,
            ),
            text_response("File not found, let me try something else", 100, 20),
        ]));

        let (mut agent, handle) = create_agent_with_tools(provider, tmp.path().to_path_buf());
        drop(handle);

        agent.run("Read a file".to_string()).await.unwrap();

        // Read tool result should have is_error=true
        let read_result = agent
            .messages
            .iter()
            .find_map(|m| match m {
                Message::ToolResult(r) if r.tool_call_id == "call_read" => Some(r),
                _ => None,
            })
            .expect("should have a read tool result");
        assert!(
            read_result.is_error,
            "reading nonexistent file should produce an error result"
        );

        // Agent should continue to turn 1 and self-correct with text
        let assistant_count = agent
            .messages
            .iter()
            .filter(|m| matches!(m, Message::Assistant(_)))
            .count();
        assert_eq!(
            assistant_count, 2,
            "agent should have 2 turns: error + recovery"
        );

        // Agent completed successfully (no Err return)
    }
}
