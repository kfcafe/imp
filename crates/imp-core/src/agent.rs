use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use futures::{future::join_all, StreamExt};
use imp_llm::{
    AssistantMessage, ContentBlock, Context, Cost, Message, Model, RequestOptions, StopReason,
    StreamEvent, ThinkingLevel, Usage,
};
use tokio::sync::mpsc;

use imp_llm::provider::RetryPolicy;

use crate::config::{AgentMode, ContextConfig};
use crate::error::Result;
use crate::guardrails::{self, GuardrailConfig, GuardrailLevel, GuardrailProfile};
use crate::hooks::{HookEvent, HookRunner};
use crate::roles::Role;
use crate::tools::ToolRegistry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingStage {
    LlmRequestStart,
    FirstStreamEvent,
    FirstTextDelta,
    FirstToolCall,
    MessageEnd,
}

impl TimingStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LlmRequestStart => "llm_request_start",
            Self::FirstStreamEvent => "first_stream_event",
            Self::FirstTextDelta => "first_text_delta",
            Self::FirstToolCall => "first_tool_call",
            Self::MessageEnd => "message_end",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimingEvent {
    pub turn: u32,
    pub stage: TimingStage,
    pub since_turn_start_ms: u64,
    pub since_llm_request_start_ms: u64,
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
    ToolOutputDelta {
        tool_call_id: String,
        text: String,
    },
    ToolExecutionEnd {
        tool_call_id: String,
        result: imp_llm::ToolResultMessage,
    },
    Timing {
        timing: TimingEvent,
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
    /// Active agent mode — controls which tools are permitted.
    pub mode: AgentMode,
    /// Engineering guardrails config.
    pub guardrail_config: GuardrailConfig,
    /// Resolved guardrail profile (None = disabled).
    pub guardrail_profile: Option<GuardrailProfile>,
    /// In-session file content cache, shared across tool calls.
    pub file_cache: Arc<crate::tools::FileCache>,
    /// Tracks which files have been read; used for staleness and unread-edit warnings.
    pub file_tracker: Arc<std::sync::Mutex<crate::tools::FileTracker>>,
    /// Max lines the read tool may return before truncating. 0 means unlimited.
    pub read_max_lines: usize,
    /// Cache options for LLM requests.
    pub cache_options: imp_llm::CacheOptions,

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
            mode: AgentMode::Full,
            guardrail_config: GuardrailConfig::default(),
            guardrail_profile: None,
            file_cache: Arc::new(crate::tools::FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(crate::tools::FileTracker::new())),
            read_max_lines: 500,
            cache_options: imp_llm::CacheOptions {
                cache_system_prompt: true,
                cache_tools: true,
                cache_recent_turns: 2,
                extended_ttl: false,
                global_scope: false,
            },

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
        self.hooks
            .fire(&HookEvent::OnAgentStart { prompt: &prompt })
            .await;

        self.messages.push(Message::user(&prompt));

        let mut turn: u32 = 0;
        let mut total_usage = Usage::default();
        let mut cancelled = false;
        let mut queued_follow_ups: std::collections::VecDeque<String> =
            std::collections::VecDeque::new();

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
                    AgentCommand::FollowUp(msg) => queued_follow_ups.push_back(msg),
                }
            }

            if cancelled {
                break;
            }

            self.emit(AgentEvent::TurnStart { index: turn }).await;
            let turn_started_at = Instant::now();

            let mut usage = crate::context::context_usage(&self.messages, &self.model);
            if usage.ratio >= self.context_config.observation_mask_threshold {
                crate::context::mask_observations(
                    &mut self.messages,
                    self.context_config.mask_window,
                );
                self.hooks
                    .fire(&HookEvent::OnContextThreshold { ratio: usage.ratio })
                    .await;
                // Masking can materially reduce context size, so any subsequent
                // logic must use fresh usage rather than the pre-masking snapshot.
                usage = crate::context::context_usage(&self.messages, &self.model);
            }

            // Context management is observation-mask only. Full conversation
            // compaction has been removed because the rewrite-based behavior
            // was too error-prone to keep in the runtime.

            // Build context and options for the LLM
            let context = Context {
                messages: self.messages.clone(),
            };

            let options = RequestOptions {
                thinking_level: self.thinking_level,
                // Let providers choose their own sensible default output budget.
                // Anthropic in particular should not default to the model's absolute
                // max output size for every request.
                max_tokens: None,
                temperature: None,
                system_prompt: self.system_prompt.clone(),
                tools: self.tools.definitions(),
                cache_options: self.cache_options.clone(),
                effort: None,
            };

            self.hooks.fire(&HookEvent::BeforeLlmCall).await;

            // Stream the LLM response with retry on transient startup failures.
            let llm_request_started_at = Instant::now();
            self.emit_timing(
                turn,
                TimingStage::LlmRequestStart,
                turn_started_at,
                llm_request_started_at,
            )
            .await;
            let model = clone_model(&self.model);
            let context = context.clone();
            let options = options.clone();
            let api_key = self.api_key.clone();
            let mut stream = crate::retry::stream_with_retry(
                move || {
                    model
                        .provider
                        .stream(&model, context.clone(), options.clone(), &api_key)
                },
                self.retry_policy.clone(),
            );

            let mut ordered_content: Vec<ContentBlock> = Vec::new();
            let mut tool_calls: Vec<(String, String, serde_json::Value)> = Vec::new();
            let mut assistant_msg: Option<AssistantMessage> = None;
            let mut saw_first_stream_event = false;
            let mut saw_first_text_delta = false;
            let mut saw_first_tool_call = false;

            while let Some(event_result) = stream.next().await {
                // Check for cancel during event processing
                if let Ok(cmd) = self.command_rx.try_recv() {
                    match cmd {
                        AgentCommand::Cancel => {
                            cancelled = true;
                            break;
                        }
                        AgentCommand::Steer(msg) => {
                            self.messages.push(Message::user(&msg));
                        }
                        AgentCommand::FollowUp(msg) => queued_follow_ups.push_back(msg),
                    }
                }

                match event_result {
                    Ok(event) => {
                        if !saw_first_stream_event {
                            saw_first_stream_event = true;
                            self.emit_timing(
                                turn,
                                TimingStage::FirstStreamEvent,
                                turn_started_at,
                                llm_request_started_at,
                            )
                            .await;
                        }
                        // Forward as delta
                        self.emit(AgentEvent::MessageDelta {
                            delta: event.clone(),
                        })
                        .await;

                        match event {
                            StreamEvent::TextDelta { text } => {
                                if !saw_first_text_delta {
                                    saw_first_text_delta = true;
                                    self.emit_timing(
                                        turn,
                                        TimingStage::FirstTextDelta,
                                        turn_started_at,
                                        llm_request_started_at,
                                    )
                                    .await;
                                }
                                push_stream_text_block(&mut ordered_content, text);
                            }
                            StreamEvent::ThinkingDelta { text } => {
                                push_stream_thinking_block(&mut ordered_content, text);
                            }
                            StreamEvent::ToolCall {
                                id,
                                name,
                                arguments,
                            } => {
                                if !saw_first_tool_call {
                                    saw_first_tool_call = true;
                                    self.emit_timing(
                                        turn,
                                        TimingStage::FirstToolCall,
                                        turn_started_at,
                                        llm_request_started_at,
                                    )
                                    .await;
                                }
                                ordered_content.push(ContentBlock::ToolCall {
                                    id: id.clone(),
                                    name: name.clone(),
                                    arguments: arguments.clone(),
                                });
                                tool_calls.push((id, name, arguments));
                            }
                            StreamEvent::MessageEnd { message } => {
                                self.emit_timing(
                                    turn,
                                    TimingStage::MessageEnd,
                                    turn_started_at,
                                    llm_request_started_at,
                                )
                                .await;
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
                    build_assistant_message(&ordered_content, &tool_calls, None)
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
            let msg = assistant_msg
                .unwrap_or_else(|| build_assistant_message(&ordered_content, &tool_calls, None));

            self.messages.push(Message::Assistant(msg.clone()));

            if tool_calls.is_empty() {
                // No tool calls — the model is done unless a queued follow-up exists.
                self.emit(AgentEvent::TurnEnd {
                    index: turn,
                    message: msg,
                })
                .await;
                if let Some(follow_up) = queued_follow_ups.pop_front() {
                    self.messages.push(Message::user(&follow_up));
                    turn += 1;
                    continue;
                }
                break;
            }

            // Execute tool calls
            let results = self.execute_tools(tool_calls).await;

            for result in &results {
                self.messages.push(Message::ToolResult(result.clone()));
            }

            self.emit(AgentEvent::TurnEnd {
                index: turn,
                message: msg,
            })
            .await;

            if let Some(follow_up) = queued_follow_ups.pop_front() {
                self.messages.push(Message::user(&follow_up));
            }

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
        // Fire corresponding hooks for lifecycle events
        match &event {
            AgentEvent::AgentEnd { .. } => {
                self.hooks
                    .fire(&HookEvent::OnAgentEnd {
                        messages: &self.messages,
                    })
                    .await;
            }
            AgentEvent::TurnEnd { index, message } => {
                self.hooks
                    .fire(&HookEvent::OnTurnEnd {
                        index: *index,
                        message,
                    })
                    .await;
            }
            _ => {}
        }
        let _ = self.event_tx.send(event).await;
    }

    async fn emit_timing(
        &self,
        turn: u32,
        stage: TimingStage,
        turn_started_at: Instant,
        llm_request_started_at: Instant,
    ) {
        let now = Instant::now();
        let timing = TimingEvent {
            turn,
            stage,
            since_turn_start_ms: now.duration_since(turn_started_at).as_millis() as u64,
            since_llm_request_start_ms: now.duration_since(llm_request_started_at).as_millis()
                as u64,
        };
        let _ = self.event_tx.send(AgentEvent::Timing { timing }).await;
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

        if tool_name == "bash" {
            if let Some(command) = args.get("command").and_then(|v| v.as_str()) {
                if let Some(hint) = mana_bash_equivalent_hint(command) {
                    let result =
                        crate::tools::ToolOutput::error(hint).into_tool_result(call_id, tool_name);
                    self.emit(AgentEvent::ToolExecutionEnd {
                        tool_call_id: call_id.to_string(),
                        result: result.clone(),
                    })
                    .await;
                    return result;
                }
            }
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
                let (update_tx, mut update_rx) = mpsc::channel(64);
                let ctx = crate::tools::ToolContext {
                    cwd: self.cwd.clone(),
                    cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    update_tx,
                    ui: self.ui.clone(),
                    file_cache: self.file_cache.clone(),
                    file_tracker: self.file_tracker.clone(),
                    mode: self.mode,
                    read_max_lines: self.read_max_lines,
                };

                // Forward tool output deltas to event stream
                let event_tx = self.event_tx.clone();
                let delta_call_id = call_id.to_string();
                let forwarder = tokio::spawn(async move {
                    while let Some(update) = update_rx.recv().await {
                        for block in &update.content {
                            if let imp_llm::ContentBlock::Text { text } = block {
                                let _ = event_tx
                                    .send(AgentEvent::ToolOutputDelta {
                                        tool_call_id: delta_call_id.clone(),
                                        text: text.clone(),
                                    })
                                    .await;
                            }
                        }
                    }
                });

                let exec_result = match tool.execute(call_id, args.clone(), ctx).await {
                    Ok(output) => output.into_tool_result(call_id, tool_name),
                    Err(e) => crate::tools::ToolOutput::error(e.to_string())
                        .into_tool_result(call_id, tool_name),
                };
                forwarder.abort();
                exec_result
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

                // Run guardrail after-write checks when enabled
                if let Some(profile) = self.guardrail_profile {
                    if self.guardrail_config.should_check_path(&path) {
                        let check_results = guardrails::run_after_write_checks(
                            &self.guardrail_config,
                            profile,
                            &self.cwd,
                        )
                        .await;

                        if !check_results.is_empty() {
                            let level = self.guardrail_config.effective_level();
                            let msg = guardrails::format_check_results(&check_results, level);
                            if !msg.is_empty() && msg != "Guardrail checks passed." {
                                // Append guardrail output to the tool result
                                result.content.push(imp_llm::ContentBlock::Text {
                                    text: format!("\n\n{msg}"),
                                });
                                if level == GuardrailLevel::Enforce
                                    && check_results.iter().any(|r| !r.success)
                                {
                                    result.is_error = true;
                                }
                            }
                        }
                    }
                }
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

fn push_stream_text_block(content: &mut Vec<ContentBlock>, text: String) {
    if text.is_empty() {
        return;
    }

    if let Some(ContentBlock::Text { text: existing }) = content.last_mut() {
        existing.push_str(&text);
    } else {
        content.push(ContentBlock::Text { text });
    }
}

fn push_stream_thinking_block(content: &mut Vec<ContentBlock>, text: String) {
    if text.is_empty() {
        return;
    }

    if let Some(ContentBlock::Thinking { text: existing }) = content.last_mut() {
        existing.push_str(&text);
    } else {
        content.push(ContentBlock::Thinking { text });
    }
}

/// Build an AssistantMessage from accumulated stream parts while preserving
/// the original block order emitted by the model.
fn build_assistant_message(
    content: &[ContentBlock],
    tool_calls: &[(String, String, serde_json::Value)],
    usage: Option<Usage>,
) -> AssistantMessage {
    let stop_reason = if tool_calls.is_empty() {
        StopReason::EndTurn
    } else {
        StopReason::ToolUse
    };

    AssistantMessage {
        content: content.to_vec(),
        usage,
        stop_reason,
        timestamp: imp_llm::now(),
    }
}

fn clone_model(model: &Model) -> Model {
    Model {
        meta: model.meta.clone(),
        provider: Arc::clone(&model.provider),
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

fn mana_bash_equivalent_hint(command: &str) -> Option<&'static str> {
    let trimmed = command.trim();
    let Some(rest) = trimmed.strip_prefix("mana") else {
        return None;
    };
    if rest.chars().next().is_some_and(|c| !c.is_whitespace()) {
        return None;
    }

    let action = rest.split_whitespace().next().unwrap_or("");
    match action {
        "status" | "list" | "ls" | "show" | "read" | "create" | "close" | "update" | "run"
        | "run_state" | "evaluate" | "agents" | "logs" | "next" | "claim" | "release" | "tree" => {
            Some("Use the native mana tool instead of `bash` for this mana command.")
        }
        _ => None,
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
        responses: Mutex<Vec<Vec<imp_llm::Result<StreamEvent>>>>,
    }

    impl MockProvider {
        fn new(responses: Vec<Vec<StreamEvent>>) -> Self {
            Self {
                responses: Mutex::new(
                    responses
                        .into_iter()
                        .map(|events| events.into_iter().map(Ok).collect())
                        .collect(),
                ),
            }
        }

        fn new_results(responses: Vec<Vec<imp_llm::Result<StreamEvent>>>) -> Self {
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
                vec![Ok(StreamEvent::Error {
                    error: "No more mock responses".to_string(),
                })]
            } else {
                responses.remove(0)
            };
            let stream = futures::stream::iter(events);
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

    #[tokio::test]
    async fn agent_emits_timing_events_in_order() {
        let provider = Arc::new(MockProvider::new(vec![text_response("timed", 10, 5)]));
        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("time this".to_string()).await.unwrap();
        drop(agent);

        let events = events_task.await.unwrap();
        let timings: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                AgentEvent::Timing { timing } => Some(*timing),
                _ => None,
            })
            .collect();

        assert!(timings.len() >= 4);
        assert_eq!(timings[0].stage, TimingStage::LlmRequestStart);
        assert_eq!(timings[1].stage, TimingStage::FirstStreamEvent);
        assert_eq!(timings[2].stage, TimingStage::FirstTextDelta);
        assert!(timings
            .iter()
            .any(|timing| timing.stage == TimingStage::MessageEnd));

        for timing in timings {
            assert_eq!(timing.turn, 0);
            assert!(timing.since_turn_start_ms >= timing.since_llm_request_start_ms);
        }
    }

    #[tokio::test]
    async fn agent_streams_message_delta_before_message_end() {
        let provider = Arc::new(MockProvider::new_results(vec![vec![
            Ok(StreamEvent::MessageStart {
                model: "test-model".to_string(),
            }),
            Ok(StreamEvent::TextDelta {
                text: "streaming".to_string(),
            }),
            Ok(StreamEvent::MessageEnd {
                message: AssistantMessage {
                    content: vec![ContentBlock::Text {
                        text: "streaming".to_string(),
                    }],
                    usage: Some(Usage {
                        input_tokens: 10,
                        output_tokens: 5,
                        cache_read_tokens: 0,
                        cache_write_tokens: 0,
                    }),
                    stop_reason: StopReason::EndTurn,
                    timestamp: 1000,
                },
            }),
        ]]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("Say hi".to_string()).await.unwrap();
        drop(agent);

        let events = events_task.await.unwrap();
        let text_delta_idx = events.iter().position(|event| {
            matches!(
                event,
                AgentEvent::MessageDelta {
                    delta: StreamEvent::TextDelta { text }
                } if text == "streaming"
            )
        });
        let turn_end_idx = events
            .iter()
            .position(|event| matches!(event, AgentEvent::TurnEnd { .. }));

        assert!(text_delta_idx.is_some());
        assert!(turn_end_idx.is_some());
        assert!(text_delta_idx.unwrap() < turn_end_idx.unwrap());
    }

    #[tokio::test]
    async fn agent_retries_before_first_meaningful_event_but_not_after() {
        let provider = Arc::new(MockProvider::new_results(vec![
            vec![
                Ok(StreamEvent::MessageStart {
                    model: "test-model".to_string(),
                }),
                Err(imp_llm::Error::Stream("startup failure".into())),
            ],
            vec![
                Ok(StreamEvent::MessageStart {
                    model: "test-model".to_string(),
                }),
                Ok(StreamEvent::TextDelta {
                    text: "recovered".to_string(),
                }),
                Ok(StreamEvent::MessageEnd {
                    message: AssistantMessage {
                        content: vec![ContentBlock::Text {
                            text: "recovered".to_string(),
                        }],
                        usage: Some(Usage {
                            input_tokens: 10,
                            output_tokens: 5,
                            cache_read_tokens: 0,
                            cache_write_tokens: 0,
                        }),
                        stop_reason: StopReason::EndTurn,
                        timestamp: 1000,
                    },
                }),
            ],
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("Recover".to_string()).await.unwrap();
        drop(agent);

        let events = events_task.await.unwrap();
        let text_delta = events.iter().position(|e| {
            matches!(
                e,
                AgentEvent::MessageDelta {
                    delta: StreamEvent::TextDelta { text }
                } if text == "recovered"
            )
        });
        let turn_end = events
            .iter()
            .position(|e| matches!(e, AgentEvent::TurnEnd { .. }));

        assert!(text_delta.is_some());
        assert!(turn_end.is_some());
        assert!(text_delta.unwrap() < turn_end.unwrap());
    }

    #[tokio::test]
    async fn agent_surfaces_error_after_partial_stream_without_retrying() {
        let provider = Arc::new(MockProvider::new_results(vec![vec![
            Ok(StreamEvent::TextDelta {
                text: "partial".to_string(),
            }),
            Err(imp_llm::Error::Stream("mid-stream failure".into())),
        ]]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));

        let events_task = tokio::spawn(collect_events(handle));
        let result = agent.run("Fail midway".to_string()).await;
        drop(agent);

        assert!(result.is_err());

        let events = events_task.await.unwrap();
        let text_delta = events.iter().position(|e| {
            matches!(
                e,
                AgentEvent::MessageDelta {
                    delta: StreamEvent::TextDelta { text }
                } if text == "partial"
            )
        });
        let error_idx = events.iter().position(
            |e| matches!(e, AgentEvent::Error { error } if error.contains("mid-stream failure")),
        );

        assert!(text_delta.is_some());
        assert!(error_idx.is_some());
        assert!(text_delta.unwrap() < error_idx.unwrap());
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
    async fn agent_follow_up_runs_after_current_work_finishes() {
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_1",
                "echo",
                serde_json::json!({"text": "hello"}),
                100,
                20,
            ),
            text_response("Handled follow-up", 120, 25),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.tools.register(Arc::new(EchoTool));

        handle
            .command_tx
            .send(AgentCommand::FollowUp("What next?".into()))
            .await
            .unwrap();

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("Do the first thing".to_string()).await.unwrap();
        drop(agent);

        let events = events_task.await.unwrap();
        let turn_starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AgentEvent::TurnStart { .. }))
            .collect();
        assert_eq!(turn_starts.len(), 2);
    }

    #[tokio::test]
    async fn agent_follow_up_preserves_order_with_multiple_messages() {
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_1",
                "echo",
                serde_json::json!({"text": "hello"}),
                100,
                20,
            ),
            text_response("First follow-up handled", 120, 25),
            text_response("Second follow-up handled", 130, 30),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.tools.register(Arc::new(EchoTool));

        handle
            .command_tx
            .send(AgentCommand::FollowUp("follow up one".into()))
            .await
            .unwrap();
        handle
            .command_tx
            .send(AgentCommand::FollowUp("follow up two".into()))
            .await
            .unwrap();

        agent.run("Do the first thing".to_string()).await.unwrap();

        let user_texts: Vec<String> = agent
            .messages
            .iter()
            .filter_map(|message| match message {
                Message::User(user) => user.content.iter().find_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                }),
                _ => None,
            })
            .collect();

        assert_eq!(
            user_texts,
            vec![
                "Do the first thing".to_string(),
                "follow up one".to_string(),
                "follow up two".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn agent_cancel_still_wins_over_follow_up_queue() {
        let provider = Arc::new(MockProvider::new(vec![tool_call_response(
            "call_1",
            "echo",
            serde_json::json!({"text": "hello"}),
            100,
            20,
        )]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.tools.register(Arc::new(EchoTool));

        handle
            .command_tx
            .send(AgentCommand::FollowUp("queued later".into()))
            .await
            .unwrap();
        handle.command_tx.send(AgentCommand::Cancel).await.unwrap();

        let result = agent.run("Do something".to_string()).await;
        assert!(matches!(result, Err(crate::error::Error::Cancelled)));
    }

    #[test]
    fn mana_bash_equivalent_hint_handles_release_and_tree() {
        assert!(mana_bash_equivalent_hint("mana release 1").is_some());
        assert!(mana_bash_equivalent_hint("mana tree").is_some());
    }

    #[test]
    fn mana_bash_equivalent_hint_ignores_non_mana_prefixes() {
        assert!(mana_bash_equivalent_hint("manatee status").is_none());
        assert!(mana_bash_equivalent_hint("./mana status").is_none());
    }

    #[tokio::test]
    async fn agent_blocks_bash_mana_when_native_action_exists() {
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_1",
                "bash",
                serde_json::json!({"command": "mana status", "timeout": 5}),
                100,
                20,
            ),
            text_response("Recovered after native-mana hint", 120, 25),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.tools.register(Arc::new(crate::tools::bash::BashTool));

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("Check mana state".to_string()).await.unwrap();
        drop(agent);

        let events = events_task.await.unwrap();
        let tool_end = events.iter().find_map(|e| match e {
            AgentEvent::ToolExecutionEnd { result, .. } => Some(result),
            _ => None,
        });
        let tool_end = tool_end.expect("expected ToolExecutionEnd");
        assert!(tool_end.is_error);
        let text = tool_end
            .content
            .iter()
            .find_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap_or("");
        assert!(text.contains("Use the native mana tool"));
    }

    #[tokio::test]
    async fn agent_allows_non_mana_bash_commands() {
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_1",
                "bash",
                serde_json::json!({"command": "printf 'ok'", "timeout": 5}),
                100,
                20,
            ),
            text_response("done", 120, 25),
        ]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.tools.register(Arc::new(crate::tools::bash::BashTool));

        agent.run("Run a shell command".to_string()).await.unwrap();

        let tool_result = agent
            .messages
            .iter()
            .find_map(|message| match message {
                Message::ToolResult(result) => Some(result),
                _ => None,
            })
            .expect("expected tool result");
        assert!(!tool_result.is_error);
    }

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

    #[tokio::test]
    async fn agent_masks_observations_when_context_is_tight() {
        let provider = Arc::new(MockProvider::new(vec![text_response("done", 100, 20)]));

        let mut seeded_messages = Vec::new();
        for index in 0..12 {
            let call_id = format!("call_{index}");
            seeded_messages.push(make_assistant_tool_call(
                &call_id,
                "read",
                serde_json::json!({"path": format!("src/file_{index}.rs")}),
            ));
            seeded_messages.push(make_tool_result(&call_id, "read", &"x".repeat(400)));
        }

        let mut usage_messages = seeded_messages.clone();
        usage_messages.push(Message::user("trigger masking"));
        let provisional_model = test_model(provider.clone());
        let usage_before = crate::context::context_usage(&usage_messages, &provisional_model);

        let mut masked_messages = usage_messages.clone();
        crate::context::mask_observations(&mut masked_messages, 10);
        let usage_after = crate::context::context_usage(&masked_messages, &provisional_model);

        assert!(usage_before.used > usage_after.used);

        // Pick a window where masking definitely triggers.
        let context_window = ((usage_before.used as f64) / 0.7).ceil() as u32;

        let model = test_model_with_context_window(provider, context_window.max(1));
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        let events_task = tokio::spawn(collect_events(handle));
        agent.messages = seeded_messages;

        agent.run("trigger masking".to_string()).await.unwrap();
        drop(agent);

        let events = events_task.await.unwrap();

        assert!(
            events
                .iter()
                .any(|e| matches!(e, AgentEvent::TurnStart { index: 0 })),
            "agent should still run normally"
        );
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
            personality: None,
            task: None,
            role: None,
            mode: &mode,
            memory: None,
            user_profile: None,
            cwd: None,
            learning_enabled: false,
            guardrail_profile: None,
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
            personality: None,
            task: None,
            role: None,
            mode: &AgentMode::Full,
            memory: None,
            user_profile: None,
            cwd: None,
            learning_enabled: false,
            guardrail_profile: None,
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
            personality: None,
            task: None,
            role: None,
            mode: &AgentMode::Orchestrator,
            memory: None,
            user_profile: None,
            cwd: None,
            learning_enabled: false,
            guardrail_profile: None,
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
            personality: None,
            task: None,
            role: None,
            mode: &AgentMode::Worker,
            memory: None,
            user_profile: None,
            cwd: None,
            learning_enabled: false,
            guardrail_profile: None,
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
            personality: None,
            task: None,
            role: None,
            mode: &AgentMode::Reviewer,
            memory: None,
            user_profile: None,
            cwd: None,
            learning_enabled: false,
            guardrail_profile: None,
        });
        assert!(
            reviewer_result.text.contains("reviewer") || reviewer_result.text.contains("read"),
            "reviewer prompt should contain mode instructions"
        );
    }
}
