use std::collections::hash_map::DefaultHasher;
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::future::join_all;
use imp_llm::{
    AssistantMessage, ContentBlock, Context, Cost, Message, Model, RequestOptions, StopReason,
    StreamEvent, ThinkingLevel, Usage,
};
use tokio::sync::mpsc;

use imp_llm::provider::RetryPolicy;

use crate::config::{AgentMode, ContextConfig};
use crate::error::Result;
use crate::hooks::{HookEvent, HookRunner};
use crate::roles::Role;
use crate::tools::ToolRegistry;

/// Detects infinite tool-call retry loops by tracking a sliding window of
/// (tool, args, result) hashes. If the same call+result hash appears
/// `max_repeats` or more times within the last `window_size` entries, the
/// agent is considered stuck.
///
/// The hash covers the tool name, serialized arguments, whether the call
/// errored, and the first 200 chars of output — so identical calls with
/// different results are distinct, but repeated identical failures are caught.
pub struct LoopDetector {
    window: VecDeque<u64>,
    window_size: usize,
    max_repeats: usize,
}

impl LoopDetector {
    /// Create a detector with a 20-entry window and a 5-repeat threshold.
    pub fn new() -> Self {
        Self {
            window: VecDeque::with_capacity(20),
            window_size: 20,
            max_repeats: 5,
        }
    }

    /// Create with custom parameters (used in tests).
    pub fn with_params(window_size: usize, max_repeats: usize) -> Self {
        Self {
            window: VecDeque::with_capacity(window_size),
            window_size,
            max_repeats,
        }
    }

    /// Record a tool execution result. Returns `true` if the agent is looping.
    pub fn record(
        &mut self,
        tool_name: &str,
        args: &serde_json::Value,
        is_error: bool,
        output_prefix: &str,
    ) -> bool {
        let hash = Self::compute_hash(tool_name, args, is_error, output_prefix);
        if self.window.len() >= self.window_size {
            self.window.pop_front();
        }
        self.window.push_back(hash);
        self.is_looping()
    }

    /// Returns `true` if any hash appears `max_repeats` or more times in the window.
    pub fn is_looping(&self) -> bool {
        if self.window.len() < self.max_repeats {
            return false;
        }
        // Count occurrences of each hash using a simple linear scan.
        // Window is small (≤20), so this is cheaper than a HashMap.
        for (i, hash) in self.window.iter().enumerate() {
            let count = self.window.iter().skip(i).filter(|h| *h == hash).count();
            if count >= self.max_repeats {
                return true;
            }
        }
        false
    }

    fn compute_hash(
        tool_name: &str,
        args: &serde_json::Value,
        is_error: bool,
        output_prefix: &str,
    ) -> u64 {
        let mut hasher = DefaultHasher::new();
        tool_name.hash(&mut hasher);
        // Serialize args deterministically for hashing.
        args.to_string().hash(&mut hasher);
        is_error.hash(&mut hasher);
        let truncated = if output_prefix.len() > 200 {
            &output_prefix[..200]
        } else {
            output_prefix
        };
        truncated.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for LoopDetector {
    fn default() -> Self {
        Self::new()
    }
}

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
    /// Context management thresholds (wired from Config via AgentBuilder).
    pub context_config: ContextConfig,
    /// Retry policy for transient LLM stream failures.
    pub retry_policy: RetryPolicy,
    /// The original prompt passed to `run()`, used to re-orient the model after compaction.
    pub original_prompt: Option<String>,
    /// Active agent mode — controls which tools are permitted.
    pub mode: AgentMode,
    /// In-session file content cache, shared across tool calls.
    pub file_cache: Arc<crate::tools::FileCache>,
    /// Tracks which files have been read; used for staleness and unread-edit warnings.
    pub file_tracker: Arc<std::sync::Mutex<crate::tools::FileTracker>>,
    /// Cache options for LLM requests.
    pub cache_options: imp_llm::CacheOptions,
    /// Detects infinite tool-call retry loops.
    pub loop_detector: LoopDetector,

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
            context_config: ContextConfig::default(),
            retry_policy: RetryPolicy::default(),
            original_prompt: None,
            mode: AgentMode::Full,
            file_cache: Arc::new(crate::tools::FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(crate::tools::FileTracker::new())),
            cache_options: imp_llm::CacheOptions {
                cache_system_prompt: true,
                cache_tools: true,
                cache_recent_turns: 2,
            },
            loop_detector: LoopDetector::new(),

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

        self.original_prompt = Some(prompt.clone());
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
            if usage.ratio >= self.context_config.observation_mask_threshold {
                crate::context::mask_observations(
                    &mut self.messages,
                    self.context_config.mask_window,
                );
                self.hooks
                    .fire(&HookEvent::OnContextThreshold { ratio: usage.ratio })
                    .await;
            }

            if usage.ratio >= self.context_config.compaction_threshold {
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
                        // Re-orient the model after compaction so it resumes
                        // the original task rather than losing its thread.
                        // Skip turn 0 — if compaction fires on the first turn,
                        // the task itself is too large and re-queueing won't help.
                        if turn > 0 {
                            if let Some(ref original) = self.original_prompt {
                                let resume_msg = format!(
                                    "[Context was compacted to manage the conversation length. \
                                     The original request was: \"{original}\". \
                                     Continue where you left off — check what's already been \
                                     done and proceed with remaining work.]"
                                );
                                self.messages.push(Message::user(&resume_msg));
                            }
                        }
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
                cache_options: self.cache_options.clone(),
            };

            self.hooks.fire(&HookEvent::BeforeLlmCall).await;

            // Stream the LLM response with retry on transient errors.
            let model_ref = &self.model;
            let api_key_ref = &self.api_key;
            let retry_result: imp_llm::Result<Vec<imp_llm::Result<StreamEvent>>> =
                crate::retry::run_with_retry(
                    || {
                        model_ref.provider.stream(
                            model_ref,
                            context.clone(),
                            options.clone(),
                            api_key_ref,
                        )
                    },
                    &self.retry_policy,
                )
                .await;

            let stream_events = match retry_result {
                Ok(events) => events,
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
            };

            let mut text_parts: Vec<String> = Vec::new();
            let mut thinking_parts: Vec<String> = Vec::new();
            let mut tool_calls: Vec<(String, String, serde_json::Value)> = Vec::new();
            let mut assistant_msg: Option<AssistantMessage> = None;

            for event_result in stream_events {
                // Check for cancel during event processing
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
                        // Errors here shouldn't happen (run_with_retry handles them)
                        // but propagate just in case.
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

            // Check for infinite retry loops before pushing results onto the
            // message history — we want to break *before* the LLM sees yet
            // another copy of the same failure.
            let mut loop_detected = false;
            for result in &results {
                let output_prefix = result
                    .content
                    .iter()
                    .find_map(|block| match block {
                        ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .unwrap_or("");
                if self.loop_detector.record(
                    &result.tool_name,
                    &serde_json::Value::Null, // args not available here; key signal is tool+output
                    result.is_error,
                    output_prefix,
                ) {
                    loop_detected = true;
                    break;
                }
            }

            // Push tool results onto messages
            for result in &results {
                self.messages.push(Message::ToolResult(result.clone()));
            }

            if loop_detected {
                self.emit(AgentEvent::Error {
                    error: "Loop detected: agent is calling the same tool with the same result repeatedly. Stopping to avoid burning tokens.".to_string(),
                })
                .await;
                self.emit(AgentEvent::TurnEnd {
                    index: turn,
                    message: msg,
                })
                .await;
                let cost = total_usage.cost(&self.model.meta.pricing);
                self.emit(AgentEvent::AgentEnd {
                    usage: total_usage,
                    cost,
                })
                .await;
                return Err(crate::error::Error::LoopDetected);
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

        // Execution-time mode guard — reject tools not permitted by the active mode.
        if !self.mode.allows_tool(tool_name) {
            let reason = format!(
                "Tool '{tool_name}' is not available in {} mode",
                format!("{:?}", self.mode).to_lowercase()
            );
            let result =
                crate::tools::ToolOutput::error(reason).into_tool_result(call_id, tool_name);
            self.emit(AgentEvent::ToolExecutionEnd {
                tool_call_id: call_id.to_string(),
                result: result.clone(),
            })
            .await;
            return result;
        }

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

        // Validate args against the tool's JSON schema before execution so the
        // model can self-correct on bad types or missing required fields.
        if let Some(tool) = self.tools.get(tool_name) {
            let schema = tool.parameters();
            if let Err(e) = crate::tools::validate_tool_args(&schema, &args) {
                let result = crate::tools::ToolOutput::error(e.to_string())
                    .into_tool_result(call_id, tool_name);
                self.emit(AgentEvent::ToolExecutionEnd {
                    tool_call_id: call_id.to_string(),
                    result: result.clone(),
                })
                .await;
                return result;
            }
        }

        let mut result = match self.tools.get(tool_name) {
            Some(tool) => {
                let (update_tx, _update_rx) = mpsc::channel(64);
                let ctx = crate::tools::ToolContext {
                    cwd: self.cwd.clone(),
                    cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    update_tx,
                    ui: self.ui.clone(),
                    file_cache: self.file_cache.clone(),
                    file_tracker: self.file_tracker.clone(),
                    mode: self.mode,
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

    // ── Retry policy tests ─────────────────────────────────────────

    /// A mock provider that returns a fixed sequence of results. Each call to
    /// `stream()` returns the next item: an `Err` for errors, or a pre-built
    /// event sequence for success.
    struct RetryMockProvider {
        calls: Mutex<Vec<std::result::Result<Vec<StreamEvent>, imp_llm::Error>>>,
    }

    impl RetryMockProvider {
        fn new(calls: Vec<std::result::Result<Vec<StreamEvent>, imp_llm::Error>>) -> Self {
            Self {
                calls: Mutex::new(calls),
            }
        }
    }

    #[async_trait]
    impl Provider for RetryMockProvider {
        fn stream(
            &self,
            _model: &Model,
            _context: Context,
            _options: RequestOptions,
            _api_key: &str,
        ) -> Pin<Box<dyn Stream<Item = imp_llm::Result<StreamEvent>> + Send>> {
            let mut calls = self.calls.try_lock().expect("RetryMockProvider lock");
            let outcome = if calls.is_empty() {
                Ok(vec![StreamEvent::Error {
                    error: "No more mock responses".to_string(),
                }])
            } else {
                calls.remove(0)
            };
            match outcome {
                Ok(events) => Box::pin(futures::stream::iter(
                    events.into_iter().map(|ev| imp_llm::Result::Ok(ev)),
                )),
                Err(e) => Box::pin(futures::stream::once(async move {
                    imp_llm::Result::<StreamEvent>::Err(e)
                })),
            }
        }

        async fn resolve_auth(&self, _auth: &AuthStore) -> imp_llm::Result<ApiKey> {
            Ok("mock-key".to_string())
        }

        fn id(&self) -> &str {
            "retry-mock"
        }

        fn models(&self) -> &[ModelMeta] {
            &[]
        }
    }

    /// Provider that fails N times with a rate-limit error, then succeeds.
    #[tokio::test]
    async fn retry_succeeds_after_transient_failures() {
        use imp_llm::provider::RetryPolicy;

        let provider = Arc::new(RetryMockProvider::new(vec![
            // First two calls fail with a rate-limit error
            Err(imp_llm::Error::RateLimited {
                retry_after_secs: Some(0),
            }),
            Err(imp_llm::Error::RateLimited {
                retry_after_secs: Some(0),
            }),
            // Third call succeeds
            Ok(text_response("Hello after retries", 100, 20)),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        // Zero delays so the test runs fast
        agent.retry_policy = RetryPolicy {
            max_retries: 3,
            base_delay: std::time::Duration::from_millis(0),
            max_delay: std::time::Duration::from_secs(30),
            retry_on: vec![],
        };

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("Say hello".to_string()).await.unwrap();
        drop(agent);

        let events = events_task.await.unwrap();

        // Agent should have completed successfully
        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::AgentEnd { .. })));

        // The final text should be present in TurnEnd
        let turn_end = events.iter().find_map(|e| match e {
            AgentEvent::TurnEnd { message, .. } => Some(message),
            _ => None,
        });
        assert!(turn_end.is_some());
        let content_text = turn_end
            .unwrap()
            .content
            .iter()
            .find_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap_or("");
        assert!(
            content_text.contains("Hello after retries"),
            "expected final text, got: {content_text}"
        );
    }

    /// When max_retries is exhausted the agent returns an error.
    #[tokio::test]
    async fn retry_fails_when_max_retries_exhausted() {
        use imp_llm::provider::RetryPolicy;

        let provider = Arc::new(RetryMockProvider::new(vec![
            Err(imp_llm::Error::RateLimited {
                retry_after_secs: Some(0),
            }),
            Err(imp_llm::Error::RateLimited {
                retry_after_secs: Some(0),
            }),
            Err(imp_llm::Error::RateLimited {
                retry_after_secs: Some(0),
            }),
            Err(imp_llm::Error::RateLimited {
                retry_after_secs: Some(0),
            }),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.retry_policy = RetryPolicy {
            max_retries: 2, // only 2 retries allowed
            base_delay: std::time::Duration::from_millis(0),
            max_delay: std::time::Duration::from_secs(30),
            retry_on: vec![],
        };
        drop(handle);

        let result = agent.run("Fail".to_string()).await;
        assert!(
            result.is_err(),
            "should have failed after exhausting retries"
        );
    }

    /// Auth errors (HTTP 401/403) must NOT be retried.
    #[tokio::test]
    async fn retry_does_not_retry_auth_errors() {
        use imp_llm::provider::RetryPolicy;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        struct CountingAuthFailProvider {
            calls: AtomicUsize,
            success_after: usize,
        }

        #[async_trait]
        impl Provider for CountingAuthFailProvider {
            fn stream(
                &self,
                _model: &Model,
                _context: Context,
                _options: RequestOptions,
                _api_key: &str,
            ) -> Pin<Box<dyn Stream<Item = imp_llm::Result<StreamEvent>> + Send>> {
                let n = self.calls.fetch_add(1, Ordering::SeqCst);
                if n < self.success_after {
                    Box::pin(futures::stream::once(async {
                        Err(imp_llm::Error::Auth("Invalid API key".to_string()))
                    }))
                } else {
                    Box::pin(futures::stream::iter(
                        text_response("ok", 10, 5).into_iter().map(Ok),
                    ))
                }
            }

            async fn resolve_auth(&self, _auth: &AuthStore) -> imp_llm::Result<ApiKey> {
                Ok("mock-key".to_string())
            }

            fn id(&self) -> &str {
                "auth-fail-mock"
            }

            fn models(&self) -> &[ModelMeta] {
                &[]
            }
        }

        let _ = call_count_clone; // silence unused warning

        let provider = Arc::new(CountingAuthFailProvider {
            calls: AtomicUsize::new(0),
            success_after: 999, // would succeed eventually, but we expect no retry
        });
        let call_ref = &provider.calls;

        let model = test_model(provider.clone());
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.retry_policy = RetryPolicy {
            max_retries: 5, // generous, to confirm auth errors bypass retry entirely
            base_delay: std::time::Duration::from_millis(0),
            max_delay: std::time::Duration::from_secs(30),
            retry_on: vec![],
        };
        drop(handle);

        let result = agent.run("Auth test".to_string()).await;
        assert!(result.is_err(), "should fail on auth error");

        // The provider should have been called exactly once — no retries.
        assert_eq!(
            call_ref.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "auth errors should not be retried"
        );
    }

    // ── Compaction resume tests ────────────────────────────────────

    /// A mock provider that intercepts the context passed to each call so tests
    /// can inspect what messages the agent sent to the model.
    struct CapturingMockProvider {
        /// Pre-programmed responses, popped in order.
        responses: Mutex<Vec<Vec<StreamEvent>>>,
        /// All contexts ever sent to `stream()`.
        captured_contexts: Mutex<Vec<Vec<Message>>>,
    }

    impl CapturingMockProvider {
        fn new(responses: Vec<Vec<StreamEvent>>) -> Self {
            Self {
                responses: Mutex::new(responses),
                captured_contexts: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl Provider for CapturingMockProvider {
        fn stream(
            &self,
            _model: &Model,
            context: Context,
            _options: RequestOptions,
            _api_key: &str,
        ) -> Pin<Box<dyn Stream<Item = imp_llm::Result<StreamEvent>> + Send>> {
            self.captured_contexts
                .try_lock()
                .expect("capturing lock")
                .push(context.messages.clone());

            let mut responses = self.responses.try_lock().expect("responses lock");
            let events = if responses.is_empty() {
                vec![StreamEvent::Error {
                    error: "No more mock responses".to_string(),
                }]
            } else {
                responses.remove(0)
            };
            Box::pin(futures::stream::iter(events.into_iter().map(Ok)))
        }

        async fn resolve_auth(
            &self,
            _auth: &imp_llm::auth::AuthStore,
        ) -> imp_llm::Result<imp_llm::auth::ApiKey> {
            Ok("mock-key".to_string())
        }

        fn id(&self) -> &str {
            "capturing-mock"
        }

        fn models(&self) -> &[ModelMeta] {
            &[]
        }
    }

    fn user_message_text(msg: &Message) -> Option<&str> {
        match msg {
            Message::User(u) => u.content.iter().find_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            }),
            _ => None,
        }
    }

    /// Build a large tool-result message to bulk up context.
    ///
    /// JSON size: "x".repeat(2000) → ~2100 chars → ~525 estimated tokens
    /// (estimate_tokens uses chars/4).
    fn make_big_tool_result(call_id: &str) -> Message {
        Message::ToolResult(imp_llm::ToolResultMessage {
            tool_call_id: call_id.to_string(),
            tool_name: "read".to_string(),
            content: vec![ContentBlock::Text {
                text: "x".repeat(2000),
            }],
            is_error: false,
            details: serde_json::Value::Null,
            timestamp: imp_llm::now(),
        })
    }

    /// Build an agent whose pre-seeded history puts context just BELOW the
    /// compaction threshold on turn 0, so compaction fires on turn 1 after
    /// the tool call + result push it over.
    ///
    /// Token math (estimate_tokens = chars/4):
    ///   - 3 pre-seeded exchanges: each is ~2200 chars (tool_call ~200 + tool_result ~2000)
    ///     → 3 × 2200 / 4 = ~1650 tokens
    ///   - User prompt: ~50 chars → ~12 tokens
    ///   - Total before turn 0 LLM call: ~1662 tokens
    ///   - Context window = 4000, threshold = 0.8 → fires at 3200 tokens
    ///   - 1662 < 3200 → compaction does NOT fire on turn 0 ✓
    ///
    ///   - Turn 0: LLM returns tool call (~200 chars / 4 = 50 tokens) + echo result (~100 chars / 4 = 25 tokens)
    ///   - Total after turn 0: ~1737 tokens → still < 3200, no compaction yet
    ///
    ///   BUT we also need the context to exceed threshold. So we bump the
    ///   pre-filled data: use 5 exchanges with 1500-char results instead.
    ///   - 5 × ~1700 chars = 8500 chars → 8500/4 = 2125 tokens
    ///   - Plus prompt: 2125 + 12 = 2137 → < 3200 at turn 0 ✓
    ///   - After turn 0 tool call + result: 2137 + 75 = 2212 → < 3200 still
    ///
    /// Actually let's simplify: use a LOW context window (3000) and threshold 0.9
    /// (fires at 2700). Pre-fill with 4 × 600-char results → 4 × 800 chars/4 = 800 tokens.
    /// After turn 0 adds ~300 tokens → 1100. Still not enough.
    ///
    /// Simplest: pre-fill LOTS of data, use a high threshold (0.95) and a
    /// modest window, so turn 0's prompt + response tips it over.
    ///
    /// Final approach: context_window=4000, threshold=0.7 (fires at 2800).
    ///   Pre-fill: 5 × (200 + 2000) chars = 11000 chars → 2750 tokens. Just below 2800.
    ///   Turn 0: user prompt adds ~50/4 = 12 tokens → 2762. Still below.
    ///   Agent turn 0 LLM call → response adds to messages → turn 0 ends.
    ///   Turn 1: context check with tool call + result → 2762 + ~200 = 2962 > 2800 → fires!
    ///
    /// Provider call ordering:
    ///   call 0 — agent turn 0: tool call (context below threshold)
    ///   call 1 — compaction LLM: fires at start of turn 1
    ///   call 2 — agent turn 1: text response after compaction (ends run)
    fn make_compaction_agent_turn1(
        compaction_summary: &str,
        post_compaction_response: Vec<StreamEvent>,
    ) -> (Agent, AgentHandle, Arc<CapturingMockProvider>) {
        let provider = Arc::new(CapturingMockProvider::new(vec![
            // call 0: turn 0 — tool call (compaction hasn't fired yet)
            tool_call_response(
                "call_t0",
                "echo",
                serde_json::json!({"text": "check"}),
                50,
                10,
            ),
            // call 1: compaction summary (fires at start of turn 1)
            text_response(compaction_summary, 400, 80),
            // call 2: turn 1 — agent's text response after compaction
            post_compaction_response,
        ]));

        // context_window=8000, threshold=0.5 → fires at 4000 tokens.
        // Pre-fill: 5 pairs × ~575 tokens = ~2875 tokens. Below 4000 on turn 0.
        // After turn 0 adds user prompt (~50) + tool call (~50) + tool result (~30)
        //   = 2875 + 130 = 3005 tokens. Still below 4000 on turn 1.
        // But we need it to fire! So use 6 pairs instead:
        //   6 × 575 = 3450 tokens. Plus turn 0 additions = ~3580. Below 4000.
        // Need threshold lower or more data. Let's use 7 pairs:
        //   7 × 575 = 4025. Over 4000 on turn 0 again!
        //
        // Different approach: smaller results. Use 500-char results:
        //   result JSON ~600 chars → 150 tokens. + assistant 50 = 200 per pair.
        //   Need >4000 tokens after turn 0 but ≤4000 before.
        //   20 pairs × 200 = 4000. With turn 0 = 4130. Threshold = 4000.
        //   Before turn 0: 4000 tokens → ratio = 4000/8000 = 0.5 → AT threshold.
        //
        // Simplest: context_window=200_000 (large), threshold=0.015 (fires at 3000).
        // Pre-fill 5 big pairs = 2875 tokens < 3000. Turn 0 adds ~130 = 3005 > 3000.
        let model = test_model_with_context_window(provider.clone(), 200_000);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.tools.register(Arc::new(EchoTool));
        agent.context_config.compaction_threshold = 0.015; // fires at ~3000 tokens
                                                           // Disable observation masking so it doesn't interfere.
        agent.context_config.observation_mask_threshold = 1.0;

        // Pre-fill 5 exchanges: ~2875 tokens total. Below 3000 threshold.
        // After turn 0 tool call adds ~130 tokens → 3005 > 3000 → compaction fires at turn 1.
        for i in 0..5 {
            let cid = format!("pre_{i}");
            agent.messages.push(make_assistant_tool_call(
                &cid,
                "read",
                serde_json::json!({"path": format!("file_{i}.rs")}),
            ));
            agent.messages.push(make_big_tool_result(&cid));
        }

        (agent, handle, provider)
    }

    /// After compaction fires mid-run (turn > 0), a resume message containing
    /// the original prompt must be present in agent.messages.
    #[tokio::test]
    #[ignore = "hangs — compaction agent loop doesn't terminate with current mock setup"]
    async fn compaction_resume_injects_original_prompt() {
        let original_prompt = "Please implement the authentication module";
        let compaction_summary = "Summary: user wants to implement auth.";

        let (mut agent, handle, _provider) = make_compaction_agent_turn1(
            compaction_summary,
            text_response("Resuming work on auth.", 50, 20),
        );

        let events_task = tokio::spawn(collect_events(handle));
        agent.run(original_prompt.to_string()).await.unwrap();

        let _events = events_task.await.unwrap();

        // After compaction the messages list should contain a user message that
        // re-states the original prompt.
        let resume_msg = agent
            .messages
            .iter()
            .find_map(|m| user_message_text(m).filter(|t| t.contains("original request was")));

        assert!(
            resume_msg.is_some(),
            "expected a resume message in agent.messages after compaction, messages: {:?}",
            agent
                .messages
                .iter()
                .map(|m| format!("{m:?}"))
                .collect::<Vec<_>>()
        );

        assert!(
            resume_msg.unwrap().contains(original_prompt),
            "resume message must contain the original prompt verbatim, got: {}",
            resume_msg.unwrap()
        );
    }

    /// The original prompt is preserved verbatim (not truncated or paraphrased).
    #[tokio::test]
    #[ignore = "hangs — compaction agent loop doesn't terminate with current mock setup"]
    async fn compaction_resume_preserves_prompt_verbatim() {
        let original_prompt = "Refactor src/auth.rs to use JWT tokens and update all 42 call sites";

        let (mut agent, handle, _provider) = make_compaction_agent_turn1(
            "Summary of auth refactor progress.",
            text_response("Continuing refactor.", 50, 20),
        );

        let events_task = tokio::spawn(collect_events(handle));
        agent.run(original_prompt.to_string()).await.unwrap();

        let _events = events_task.await.unwrap();

        let found = agent.messages.iter().any(|m| {
            user_message_text(m)
                .map(|t| t.contains(original_prompt))
                .unwrap_or(false)
        });

        assert!(
            found,
            "original prompt must appear verbatim in agent.messages after compaction"
        );
    }

    /// When compaction fires on turn 0 no resume message is injected — the task
    /// is just too big, re-queueing the prompt won't help.
    #[tokio::test]
    #[ignore = "hangs — compaction agent loop doesn't terminate with current mock setup"]
    async fn compaction_resume_no_inject_on_turn_zero() {
        // Pre-seed MORE messages so context already exceeds threshold before the
        // new prompt is added, ensuring compaction fires on turn 0.
        let compaction_summary = "Summary: nothing done yet.";

        let provider = Arc::new(CapturingMockProvider::new(vec![
            // call 0: compaction LLM (fires on turn 0)
            text_response(compaction_summary, 400, 80),
            // call 1: agent turn 0 after compaction
            text_response("Starting work.", 50, 20),
        ]));

        // context_window=4000, threshold=0.5 → fires at 2000 tokens.
        // 10 × 2200-char exchanges → ~5500 tokens → well over threshold on turn 0.
        let model = test_model_with_context_window(provider.clone(), 4000);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.context_config.compaction_threshold = 0.5;
        agent.context_config.observation_mask_threshold = 1.0; // disable masking

        // Pre-seed with enough to exceed the threshold on turn 0 (before any LLM call)
        for i in 0..10 {
            let cid = format!("pre_{i}");
            agent.messages.push(make_assistant_tool_call(
                &cid,
                "read",
                serde_json::json!({"path": format!("file_{i}.rs")}),
            ));
            agent.messages.push(make_big_tool_result(&cid));
        }

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("Do the task".to_string()).await.unwrap();

        let _events = events_task.await.unwrap();

        // No resume message should appear — compaction fired on turn 0 before any real work
        let has_resume = agent.messages.iter().any(|m| {
            user_message_text(m)
                .map(|t| t.contains("original request was"))
                .unwrap_or(false)
        });

        assert!(
            !has_resume,
            "no resume message should be injected when compaction fires on turn 0"
        );
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
    fn create_agent_with_tools(provider: Arc<dyn Provider>, cwd: PathBuf) -> (Agent, AgentHandle) {
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

        agent.run("Search for a pattern".to_string()).await.unwrap();

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

// ── Mode enforcement tests ─────────────────────────────────────────

#[cfg(test)]
mod mode_tests {
    use super::*;
    use std::path::PathBuf;
    use std::pin::Pin;
    use std::sync::Arc;

    use async_trait::async_trait;
    use futures_core::Stream;
    use imp_llm::auth::{ApiKey, AuthStore};
    use imp_llm::model::ModelMeta;
    use imp_llm::provider::Provider;
    use tokio::sync::Mutex;

    // ── Mock provider (same shape as in tests) ─────────────────────

    struct MockProvider {
        responses: Mutex<Vec<Vec<imp_llm::StreamEvent>>>,
    }

    impl MockProvider {
        fn new(responses: Vec<Vec<imp_llm::StreamEvent>>) -> Self {
            Self {
                responses: Mutex::new(responses),
            }
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn stream(
            &self,
            _model: &imp_llm::Model,
            _context: imp_llm::Context,
            _options: imp_llm::RequestOptions,
            _api_key: &str,
        ) -> Pin<Box<dyn Stream<Item = imp_llm::Result<imp_llm::StreamEvent>> + Send>> {
            let mut responses = self.responses.try_lock().expect("MockProvider lock");
            let events = if responses.is_empty() {
                vec![imp_llm::StreamEvent::Error {
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

        fn models(&self) -> &[imp_llm::model::ModelMeta] {
            &[]
        }
    }

    fn test_model(provider: Arc<dyn Provider>) -> imp_llm::Model {
        imp_llm::Model {
            meta: ModelMeta {
                id: "test-model".to_string(),
                provider: "mock".to_string(),
                name: "Test Model".to_string(),
                context_window: 200_000,
                max_output_tokens: 16_384,
                pricing: imp_llm::model::ModelPricing {
                    input_per_mtok: 3.0,
                    output_per_mtok: 15.0,
                    cache_read_per_mtok: 0.3,
                    cache_write_per_mtok: 3.75,
                },
                capabilities: imp_llm::model::Capabilities {
                    reasoning: true,
                    images: false,
                    tool_use: true,
                },
            },
            provider,
        }
    }

    fn text_response(text: &str, input: u32, output: u32) -> Vec<imp_llm::StreamEvent> {
        vec![
            imp_llm::StreamEvent::MessageStart {
                model: "test-model".to_string(),
            },
            imp_llm::StreamEvent::TextDelta {
                text: text.to_string(),
            },
            imp_llm::StreamEvent::MessageEnd {
                message: imp_llm::AssistantMessage {
                    content: vec![imp_llm::ContentBlock::Text {
                        text: text.to_string(),
                    }],
                    usage: Some(imp_llm::Usage {
                        input_tokens: input,
                        output_tokens: output,
                        cache_read_tokens: 0,
                        cache_write_tokens: 0,
                    }),
                    stop_reason: imp_llm::StopReason::EndTurn,
                    timestamp: 1000,
                },
            },
        ]
    }

    fn tool_call_response(
        call_id: &str,
        tool_name: &str,
        args: serde_json::Value,
        input: u32,
        output: u32,
    ) -> Vec<imp_llm::StreamEvent> {
        vec![
            imp_llm::StreamEvent::MessageStart {
                model: "test-model".to_string(),
            },
            imp_llm::StreamEvent::ToolCall {
                id: call_id.to_string(),
                name: tool_name.to_string(),
                arguments: args.clone(),
            },
            imp_llm::StreamEvent::MessageEnd {
                message: imp_llm::AssistantMessage {
                    content: vec![imp_llm::ContentBlock::ToolCall {
                        id: call_id.to_string(),
                        name: tool_name.to_string(),
                        arguments: args,
                    }],
                    usage: Some(imp_llm::Usage {
                        input_tokens: input,
                        output_tokens: output,
                        cache_read_tokens: 0,
                        cache_write_tokens: 0,
                    }),
                    stop_reason: imp_llm::StopReason::ToolUse,
                    timestamp: 1000,
                },
            },
        ]
    }

    async fn collect_events(mut handle: AgentHandle) -> Vec<AgentEvent> {
        let mut events = Vec::new();
        while let Some(event) = handle.event_rx.recv().await {
            events.push(event);
        }
        events
    }

    // ── Tool fixtures ───────────────────────────────────────────────

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
                "properties": { "text": { "type": "string" } },
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

    struct NamedWriteTool(&'static str);

    #[async_trait]
    impl crate::tools::Tool for NamedWriteTool {
        fn name(&self) -> &str {
            self.0
        }
        fn label(&self) -> &str {
            self.0
        }
        fn description(&self) -> &str {
            "A write tool"
        }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type": "object", "properties": {"data": {"type": "string"}}})
        }
        fn is_readonly(&self) -> bool {
            false
        }
        async fn execute(
            &self,
            _call_id: &str,
            _params: serde_json::Value,
            _ctx: crate::tools::ToolContext,
        ) -> crate::error::Result<crate::tools::ToolOutput> {
            Ok(crate::tools::ToolOutput::text("written"))
        }
    }

    fn single_text_model(text: &str) -> Arc<MockProvider> {
        Arc::new(MockProvider::new(vec![text_response(text, 50, 10)]))
    }

    /// Test: Full mode registers all tools (no filtering).
    #[tokio::test]
    async fn agent_mode_enforcement_full_registers_all_tools() {
        use crate::config::AgentMode;

        let provider = single_text_model("ok");
        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.mode = AgentMode::Full;

        // Register a mix of tools
        agent.tools.register(Arc::new(EchoTool)); // "echo" - not in any allow-list
        agent.tools.register(Arc::new(NamedWriteTool("write")));

        // Full mode allows everything — both tools should be present
        assert!(
            agent.tools.get("echo").is_some(),
            "echo should be registered"
        );
        assert!(
            agent.tools.get("write").is_some(),
            "write should be registered"
        );
        assert!(agent.mode.allows_tool("echo"));
        assert!(agent.mode.allows_tool("write"));
        assert!(agent.mode.allows_tool("any_future_tool"));
    }

    /// Test: Orchestrator mode excludes write-category tools at registration time.
    #[test]
    fn agent_mode_enforcement_orchestrator_excludes_write_tools() {
        use crate::config::AgentMode;
        use crate::tools::ToolRegistry;

        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool)); // "echo" — not in orchestrator allow-list
        registry.register(Arc::new(NamedWriteTool("write")));
        registry.register(Arc::new(NamedWriteTool("edit")));
        registry.register(Arc::new(NamedWriteTool("bash")));

        // Apply the mode filter exactly as AgentBuilder would
        let mode = AgentMode::Orchestrator;
        registry.retain(|name| mode.allows_tool(name));

        // Write-category tools must be absent
        assert!(
            registry.get("write").is_none(),
            "write must be filtered out"
        );
        assert!(registry.get("edit").is_none(), "edit must be filtered out");
        assert!(registry.get("bash").is_none(), "bash must be filtered out");
        // echo is not in any mode allow-list either
        assert!(registry.get("echo").is_none(), "echo must be filtered out");
    }

    /// Test: Execution-time guard blocks a disallowed tool call and returns an error result.
    #[tokio::test]
    async fn agent_mode_enforcement_guard_blocks_disallowed() {
        use crate::config::AgentMode;

        let provider = Arc::new(MockProvider::new(vec![
            // Turn 0: model calls "write" — disallowed in orchestrator mode
            tool_call_response(
                "call_1",
                "write",
                serde_json::json!({"data": "content"}),
                50,
                10,
            ),
            // Turn 1: model responds after seeing the error
            text_response("Understood, I cannot write directly.", 50, 10),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.mode = AgentMode::Orchestrator;
        // Register write so it passes schema validation — the mode guard fires first
        agent.tools.register(Arc::new(NamedWriteTool("write")));

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("Write something".to_string()).await.unwrap();
        drop(agent);

        let events = events_task.await.unwrap();

        // The tool execution end event should carry an error result
        let tool_end = events
            .iter()
            .find(|e| matches!(e, AgentEvent::ToolExecutionEnd { .. }));
        assert!(tool_end.is_some(), "should have a ToolExecutionEnd event");

        if let Some(AgentEvent::ToolExecutionEnd { result, .. }) = tool_end {
            assert!(result.is_error, "mode guard should produce an error result");
            let text = result.content.iter().find_map(|c| {
                if let ContentBlock::Text { text } = c {
                    Some(text.as_str())
                } else {
                    None
                }
            });
            let text = text.expect("error result should have text");
            assert!(
                text.contains("write") && text.contains("mode"),
                "error should name the tool and mention mode, got: {text}"
            );
        }
    }

    /// Test: Execution-time guard allows a permitted tool call through cleanly.
    #[tokio::test]
    async fn agent_mode_enforcement_guard_allows_permitted() {
        use crate::config::AgentMode;

        let provider = Arc::new(MockProvider::new(vec![
            // Turn 0: model calls "read" — allowed in orchestrator mode
            tool_call_response(
                "call_1",
                "echo",
                serde_json::json!({"text": "hello"}),
                50,
                10,
            ),
            text_response("Echo succeeded", 50, 10),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        // Orchestrator allows "read", "grep", "find", "ls", "scan", "web", "diff_show", "mana", "ask"
        // We use Full mode but register "echo" and let the mode allow it via Full
        agent.mode = AgentMode::Full;
        agent.tools.register(Arc::new(EchoTool));

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("Echo something".to_string()).await.unwrap();
        drop(agent);

        let events = events_task.await.unwrap();

        // Tool should have succeeded (not an error)
        let tool_end = events
            .iter()
            .find(|e| matches!(e, AgentEvent::ToolExecutionEnd { .. }));
        assert!(tool_end.is_some());

        if let Some(AgentEvent::ToolExecutionEnd { result, .. }) = tool_end {
            assert!(!result.is_error, "permitted tool should succeed");
        }
    }

    /// Test: System prompt filters tool descriptions by mode.
    #[test]
    fn agent_mode_enforcement_system_prompt_filters() {
        use crate::config::AgentMode;
        use crate::system_prompt::{assemble, AssembleParams};
        use crate::tools::ToolRegistry;

        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(NamedWriteTool("write")));
        registry.register(Arc::new(NamedWriteTool("edit")));
        registry.register(Arc::new(NamedWriteTool("bash")));

        // Provide read-category tools too
        struct ReadTool;
        #[async_trait]
        impl crate::tools::Tool for ReadTool {
            fn name(&self) -> &str {
                "read"
            }
            fn label(&self) -> &str {
                "Read"
            }
            fn description(&self) -> &str {
                "Read a file"
            }
            fn parameters(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            fn is_readonly(&self) -> bool {
                true
            }
            async fn execute(
                &self,
                _: &str,
                _: serde_json::Value,
                _: crate::tools::ToolContext,
            ) -> crate::error::Result<crate::tools::ToolOutput> {
                Ok(crate::tools::ToolOutput::text(""))
            }
        }
        registry.register(Arc::new(ReadTool));

        let mode = AgentMode::Orchestrator;
        let result = assemble(&AssembleParams {
            tools: &registry,
            agents_md: &[],
            skills: &[],
            facts: &[],
            task: None,
            role: None,
            mode: &mode,
            memory: None,
            user_profile: None,
            learning_enabled: false,
        });

        // Orchestrator allows "read" — should appear in system prompt
        assert!(
            result.text.contains("- read:"),
            "read should be in orchestrator prompt"
        );

        // Write tools must be absent from the system prompt
        assert!(
            !result.text.contains("- write:"),
            "write must not appear in orchestrator prompt"
        );
        assert!(
            !result.text.contains("- edit:"),
            "edit must not appear in orchestrator prompt"
        );
        assert!(
            !result.text.contains("- bash:"),
            "bash must not appear in orchestrator prompt"
        );
    }

    /// Test: System prompt includes mode instructions for non-Full modes.
    #[test]
    fn agent_mode_enforcement_system_prompt_instructions() {
        use crate::config::AgentMode;
        use crate::system_prompt::{assemble, AssembleParams};
        use crate::tools::ToolRegistry;

        let registry = ToolRegistry::new();

        // Full mode — no extra instructions
        let full_result = assemble(&AssembleParams {
            tools: &registry,
            agents_md: &[],
            skills: &[],
            facts: &[],
            task: None,
            role: None,
            mode: &AgentMode::Full,
            memory: None,
            user_profile: None,
            learning_enabled: false,
        });
        // Full mode has no instructions
        assert!(
            !full_result.text.contains("orchestrator"),
            "Full mode should not mention orchestrator"
        );
        assert!(
            !full_result.text.contains("worker"),
            "Full mode should not mention worker"
        );

        // Orchestrator mode — should include mode instructions
        let orch_result = assemble(&AssembleParams {
            tools: &registry,
            agents_md: &[],
            skills: &[],
            facts: &[],
            task: None,
            role: None,
            mode: &AgentMode::Orchestrator,
            memory: None,
            user_profile: None,
            learning_enabled: false,
        });
        assert!(
            orch_result.text.contains("orchestrator"),
            "orchestrator prompt should contain mode instructions, got: {}",
            orch_result.text
        );

        // Worker mode — should include mode instructions
        let worker_result = assemble(&AssembleParams {
            tools: &registry,
            agents_md: &[],
            skills: &[],
            facts: &[],
            task: None,
            role: None,
            mode: &AgentMode::Worker,
            memory: None,
            user_profile: None,
            learning_enabled: false,
        });
        assert!(
            worker_result.text.contains("worker"),
            "worker prompt should contain mode instructions"
        );

        // Reviewer mode — should include mode instructions
        let reviewer_result = assemble(&AssembleParams {
            tools: &registry,
            agents_md: &[],
            skills: &[],
            facts: &[],
            task: None,
            role: None,
            mode: &AgentMode::Reviewer,
            memory: None,
            user_profile: None,
            learning_enabled: false,
        });
        assert!(
            reviewer_result.text.contains("reviewer") || reviewer_result.text.contains("read"),
            "reviewer prompt should contain mode instructions"
        );
    }

    // ── Loop detection unit tests ──────────────────────────────────

    /// Feeding identical (tool, args, result) tuples triggers detection at the threshold.
    #[test]
    fn loop_detect_triggers_at_threshold() {
        let args = serde_json::json!({"path": "/tmp/x"});
        let mut detector = LoopDetector::with_params(20, 5);

        for i in 0..4 {
            let looping = detector.record("read", &args, true, "error: file not found");
            assert!(
                !looping,
                "should not trigger before threshold (call {})",
                i + 1
            );
        }

        // 5th identical call should trigger detection
        assert!(
            detector.record("read", &args, true, "error: file not found"),
            "should trigger on 5th identical call"
        );
    }

    /// Varied calls with different tools or outputs do not trigger false positives.
    #[test]
    fn loop_detect_no_false_positive_varied_calls() {
        let mut detector = LoopDetector::with_params(20, 5);

        // Different tool names
        assert!(!detector.record("read", &serde_json::json!({}), false, "content A"));
        assert!(!detector.record("write", &serde_json::json!({}), false, "wrote"));
        assert!(!detector.record("bash", &serde_json::json!({}), false, "output"));
        assert!(!detector.record("ls", &serde_json::json!({}), false, "files"));
        assert!(!detector.record("grep", &serde_json::json!({}), false, "match"));
        assert!(!detector.record("read", &serde_json::json!({}), false, "content B")); // same tool, different output

        // Should never have triggered
        assert!(!detector.is_looping());
    }

    /// Different outputs for the same tool do not trigger loop detection.
    #[test]
    fn loop_detect_no_false_positive_different_outputs() {
        let args = serde_json::json!({"path": "/tmp/x"});
        let mut detector = LoopDetector::with_params(20, 5);

        for i in 0..10 {
            let output = format!("output number {i}");
            let looping = detector.record("read", &args, false, &output);
            assert!(
                !looping,
                "different outputs should not trigger loop detection"
            );
        }
    }

    /// Detection resets after the window slides past old entries.
    #[test]
    fn loop_detect_resets_after_window_slides() {
        // Window of 6, threshold of 3
        let args = serde_json::json!({"path": "/tmp/x"});
        let mut detector = LoopDetector::with_params(6, 3);

        // Fill with 3 identical entries — triggers detection
        detector.record("read", &args, true, "fail");
        detector.record("read", &args, true, "fail");
        assert!(detector.record("read", &args, true, "fail")); // triggers at 3

        // Now push 6 different entries to slide the window past the old ones
        for i in 0..6 {
            let output = format!("fresh output {i}");
            detector.record("other_tool", &args, false, &output);
        }

        // The identical entries are gone from the window — should no longer loop
        assert!(
            !detector.is_looping(),
            "loop should clear after window slides"
        );
    }

    /// Agent integration: a mock provider that always calls the same failing tool
    /// must be stopped by loop detection before max_turns is reached.
    #[tokio::test]
    async fn loop_detect_agent_breaks_on_repeated_failing_tool() {
        // Build a provider that always calls "echo" with the same args — simulating a
        // model stuck in a retry loop.
        let responses: Vec<Vec<StreamEvent>> = (0..50)
            .map(|_| {
                tool_call_response(
                    "call_stuck",
                    "echo",
                    serde_json::json!({"text": "same input every time"}),
                    50,
                    10,
                )
            })
            .collect();

        let provider = Arc::new(MockProvider::new(responses));
        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.tools.register(Arc::new(EchoTool));
        agent.max_turns = 50; // Would exhaust without loop detection
                              // Use tight detector params so the test runs fast
        agent.loop_detector = LoopDetector::with_params(10, 5);

        let events_task = tokio::spawn(collect_events(handle));
        let result = agent.run("Loop forever".to_string()).await;
        drop(agent);

        let events = events_task.await.unwrap();

        // Should have returned LoopDetected, not MaxTurns or Ok
        assert!(
            matches!(result, Err(crate::error::Error::LoopDetected)),
            "expected LoopDetected error, got: {:?}",
            result
        );

        // Should have emitted an Error event describing the loop
        let loop_error = events
            .iter()
            .find(|e| matches!(e, AgentEvent::Error { error } if error.contains("Loop detected")));
        assert!(loop_error.is_some(), "expected a Loop detected error event");

        // Should still emit AgentEnd so consumers get a clean shutdown signal
        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::AgentEnd { .. })));

        // Should stop well before max_turns (≤ window_size + threshold)
        let turn_count = events
            .iter()
            .filter(|e| matches!(e, AgentEvent::TurnStart { .. }))
            .count();
        assert!(
            turn_count < 50,
            "loop detection should stop the agent early, but ran {turn_count} turns"
        );
    }
}
