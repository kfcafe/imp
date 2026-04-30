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

use crate::config::{AgentMode, Config, ContextConfig, ContinuePolicy};
use crate::error::Result;
use crate::guardrails::{self, GuardrailConfig, GuardrailLevel, GuardrailProfile};
use crate::hooks::{HookBackgroundEvent, HookEvent, HookRunner};
use crate::mana_review::{ManaReviewState, TurnManaReview, TurnManaReviewAccumulator};
use crate::roles::Role;
use crate::tools::{LuaToolLoader, ToolRegistry};
use crate::ui::NotifyLevel;

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
    TurnAssessment {
        index: u32,
        assessment: NextActionAssessment,
    },
    TurnEnd {
        index: u32,
        message: AssistantMessage,
        mana_review: TurnManaReview,
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
    Warning {
        message: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum NextAction {
    Continue {
        prompt: String,
        reason: ContinueReason,
    },
    Stop {
        reason: NextActionStopReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContinueReason {
    ExternalizationNeeded,
    HighConfidenceVisibleNextStep,
    ExecutionDebt,
}

impl ContinueReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::ExternalizationNeeded => "externalization_needed",
            Self::HighConfidenceVisibleNextStep => "high_confidence_visible_next_step",
            Self::ExecutionDebt => "execution_debt",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NextActionStopReason {
    NoAutomaticFollowUp,
    NoProgress,
    RepeatedAction,
    UserBlocker,
    ExecutionBlocked,
    DecompositionCompleted,
    WorkCompleted,
}

impl NextActionStopReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::NoAutomaticFollowUp => "no_automatic_follow_up",
            Self::NoProgress => "no_progress",
            Self::RepeatedAction => "repeated_action",
            Self::UserBlocker => "user_blocker",
            Self::ExecutionBlocked => "execution_blocked",
            Self::DecompositionCompleted => "decomposition_completed",
            Self::WorkCompleted => "work_completed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeEvidence {
    repeated_action: bool,
    execution_stop_reason: Option<NextActionStopReason>,
    work_completed: bool,
    execution_debt: bool,
    execution_evidence: bool,
    planning_only_progress: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManaEvidence {
    stop_reason: Option<NextActionStopReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextFallbackEvidence {
    planner_stop_reason: Option<NextActionStopReason>,
    execution_stop_reason: Option<NextActionStopReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContinueRecommendation {
    prompt: String,
    reason: ContinueReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextActionAssessment {
    pub runtime: NextActionRuntimeEvidence,
    pub mana: NextActionManaEvidence,
    pub text_fallback: NextActionTextFallbackEvidence,
    pub continue_recommendation: Option<NextActionContinueRecommendation>,
    pub chosen_action: NextActionDebugView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextActionRuntimeEvidence {
    pub repeated_action: bool,
    pub execution_stop_reason: Option<String>,
    pub work_completed: bool,
    pub execution_debt: bool,
    pub execution_evidence: bool,
    pub planning_only_progress: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextActionManaEvidence {
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextActionTextFallbackEvidence {
    pub planner_stop_reason: Option<String>,
    pub execution_stop_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextActionContinueRecommendation {
    pub prompt: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextActionDebugView {
    Continue { prompt: String, reason: String },
    Stop { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PostTurnAssessment {
    runtime: RuntimeEvidence,
    mana: ManaEvidence,
    text_fallback: TextFallbackEvidence,
    continue_recommendation: Option<ContinueRecommendation>,
}

impl PostTurnAssessment {
    fn into_next_action(self) -> NextAction {
        if self.runtime.repeated_action {
            return NextAction::Stop {
                reason: NextActionStopReason::RepeatedAction,
            };
        }

        if let Some(reason) = self.runtime.execution_stop_reason {
            return NextAction::Stop { reason };
        }

        if self.runtime.work_completed {
            return NextAction::Stop {
                reason: NextActionStopReason::WorkCompleted,
            };
        }

        if let Some(reason) = self.mana.stop_reason {
            return NextAction::Stop { reason };
        }

        if let Some(reason) = self.text_fallback.planner_stop_reason {
            return NextAction::Stop { reason };
        }

        if let Some(reason) = self.text_fallback.execution_stop_reason {
            return NextAction::Stop { reason };
        }

        if let Some(continue_recommendation) = self.continue_recommendation {
            return NextAction::Continue {
                prompt: continue_recommendation.prompt,
                reason: continue_recommendation.reason,
            };
        }

        if self.runtime.planning_only_progress {
            return NextAction::Stop {
                reason: NextActionStopReason::NoProgress,
            };
        }

        NextAction::Stop {
            reason: NextActionStopReason::NoAutomaticFollowUp,
        }
    }

    fn debug_view(&self) -> NextActionAssessment {
        let chosen_action = match self.clone().into_next_action() {
            NextAction::Continue { prompt, reason } => NextActionDebugView::Continue {
                prompt,
                reason: reason.as_str().to_string(),
            },
            NextAction::Stop { reason } => NextActionDebugView::Stop {
                reason: reason.as_str().to_string(),
            },
        };

        NextActionAssessment {
            runtime: NextActionRuntimeEvidence {
                repeated_action: self.runtime.repeated_action,
                execution_stop_reason: self
                    .runtime
                    .execution_stop_reason
                    .map(|reason| reason.as_str().to_string()),
                work_completed: self.runtime.work_completed,
                execution_debt: self.runtime.execution_debt,
                execution_evidence: self.runtime.execution_evidence,
                planning_only_progress: self.runtime.planning_only_progress,
            },
            mana: NextActionManaEvidence {
                stop_reason: self
                    .mana
                    .stop_reason
                    .map(|reason| reason.as_str().to_string()),
            },
            text_fallback: NextActionTextFallbackEvidence {
                planner_stop_reason: self
                    .text_fallback
                    .planner_stop_reason
                    .map(|reason| reason.as_str().to_string()),
                execution_stop_reason: self
                    .text_fallback
                    .execution_stop_reason
                    .map(|reason| reason.as_str().to_string()),
            },
            continue_recommendation: self.continue_recommendation.clone().map(|recommendation| {
                NextActionContinueRecommendation {
                    prompt: recommendation.prompt,
                    reason: recommendation.reason.as_str().to_string(),
                }
            }),
            chosen_action,
        }
    }
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
    pub max_tokens: Option<u32>,
    pub role: Option<Role>,
    pub hooks: HookRunner,
    pub api_key: String,
    /// Optional auth store for automatic OAuth token refresh before LLM calls.
    /// Optional auth store for automatic OAuth token refresh before LLM calls.
    pub auth_store: Option<std::sync::Arc<tokio::sync::Mutex<imp_llm::auth::AuthStore>>>,
    pub ui: Arc<dyn crate::ui::UserInterface>,
    /// Context management thresholds (wired from Config via AgentBuilder).
    pub context_config: ContextConfig,
    /// Retry policy for transient LLM stream failures.
    pub retry_policy: RetryPolicy,
    /// Active agent mode — controls which tools are permitted.
    pub mode: AgentMode,
    /// Whether a mana skill is available in discovered resources.
    pub has_mana_skill: bool,
    /// Whether a mana-basics skill is available in discovered resources.
    pub has_mana_basics_skill: bool,
    /// Whether a mana-delegation skill is available in discovered resources.
    pub has_mana_delegation_skill: bool,
    /// Engineering guardrails config.
    pub guardrail_config: GuardrailConfig,
    /// Resolved guardrail profile (None = disabled).
    pub guardrail_profile: Option<GuardrailProfile>,
    /// Cloneable Lua extension tool loader inherited from the session/builder.
    pub lua_tool_loader: Option<LuaToolLoader>,
    /// In-session file content cache, shared across tool calls.
    pub file_cache: Arc<crate::tools::FileCache>,
    /// Shared checkpoint/file-history state, used to capture destructive edit restore points.
    pub checkpoint_state: Arc<crate::tools::CheckpointState>,
    /// Tracks which files have been read; used for staleness and unread-edit warnings.
    pub file_tracker: Arc<std::sync::Mutex<crate::tools::FileTracker>>,
    /// Session-local anchors emitted by read and consumed by anchored edit mode.
    pub anchor_store: Arc<crate::tools::AnchorStore>,
    /// Max lines the read tool may return before truncating. 0 means unlimited.
    pub read_max_lines: usize,
    /// Cache options for LLM requests.
    pub cache_options: imp_llm::CacheOptions,
    /// Tracks identical consecutive tool calls to detect loops.
    last_tool_call: std::sync::Arc<std::sync::Mutex<Option<RepeatedToolCallState>>>,
    /// Prevent repeated self-nudges for mana externalization in a single run.
    queued_mana_externalization_nudge: bool,
    /// Policy for imp-local visible auto-continuation after high-confidence turns.
    pub continue_policy: ContinuePolicy,
    /// Prevent repeated confidence-based auto-continue nudges in a single run.
    queued_confidence_continue_nudge: bool,
    /// Prevent repeated execution-debt stop-gate follow-ups in a single run.
    queued_execution_debt_follow_up: bool,
    /// Runtime-side turn-scoped between-turn mana review accumulator.
    turn_mana_review: Arc<std::sync::Mutex<TurnManaReviewAccumulator>>,
    /// Resolved runtime config for tool-specific policy checks.
    pub config: Arc<Config>,

    event_tx: mpsc::Sender<AgentEvent>,
    command_tx: mpsc::Sender<AgentCommand>,
    command_rx: mpsc::Receiver<AgentCommand>,
    cancel_token: Arc<std::sync::atomic::AtomicBool>,
}

/// Handle for controlling the agent from outside.
pub struct AgentHandle {
    pub event_rx: mpsc::Receiver<AgentEvent>,
    pub command_tx: mpsc::Sender<AgentCommand>,
    pub cancel_token: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Debug, Clone)]
struct RepeatedToolCallState {
    tool_name: String,
    args_json: String,
    consecutive: usize,
}

#[derive(Debug, Clone)]
enum RepeatedToolCallCheck {
    Ok,
    Warn(String),
    Block(imp_llm::ToolResultMessage),
}

impl Agent {
    pub fn new(model: Model, cwd: PathBuf) -> (Self, AgentHandle) {
        let (event_tx, event_rx) = mpsc::channel(256);
        let (command_tx, command_rx) = mpsc::channel(32);
        let cancel_token = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut hooks = HookRunner::new();
        let background_event_tx = event_tx.clone();
        hooks.set_background_reporter(Arc::new(move |event: HookBackgroundEvent| {
            let background_event_tx = background_event_tx.clone();
            tokio::spawn(async move {
                let _ = background_event_tx
                    .send(AgentEvent::Warning {
                        message: event.to_string(),
                    })
                    .await;
            });
        }));

        let agent = Self {
            model,
            thinking_level: ThinkingLevel::Medium,
            tools: ToolRegistry::new(),
            messages: Vec::new(),
            system_prompt: String::new(),
            cwd,
            max_turns: 50,
            max_tokens: None,
            role: None,
            hooks,
            api_key: String::new(),
            ui: Arc::new(crate::ui::NullInterface),
            context_config: ContextConfig::default(),
            retry_policy: RetryPolicy::default(),
            mode: AgentMode::Full,
            has_mana_skill: false,
            has_mana_basics_skill: false,
            has_mana_delegation_skill: false,
            guardrail_config: GuardrailConfig::default(),
            guardrail_profile: None,
            file_cache: Arc::new(crate::tools::FileCache::new()),
            checkpoint_state: Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(crate::tools::FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            read_max_lines: 500,
            auth_store: None,
            cache_options: imp_llm::CacheOptions {
                cache_system_prompt: true,
                cache_tools: true,
                cache_recent_turns: 2,
                extended_ttl: false,
                global_scope: false,
            },
            last_tool_call: Arc::new(std::sync::Mutex::new(None)),
            queued_mana_externalization_nudge: false,
            continue_policy: ContinuePolicy::Disabled,
            queued_confidence_continue_nudge: false,
            queued_execution_debt_follow_up: false,
            turn_mana_review: Arc::new(std::sync::Mutex::new(TurnManaReviewAccumulator::default())),
            config: Arc::new(Config::default()),
            lua_tool_loader: None,

            event_tx,
            command_tx: command_tx.clone(),
            command_rx,
            cancel_token: Arc::clone(&cancel_token),
        };

        let handle = AgentHandle {
            event_rx,
            command_tx,
            cancel_token,
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

        self.cancel_token
            .store(false, std::sync::atomic::Ordering::Relaxed);
        let mut turn: u32 = 0;
        let mut total_usage = Usage::default();
        let mut cancelled = false;
        let mut queued_follow_ups: std::collections::VecDeque<String> =
            std::collections::VecDeque::new();
        let mut queued_pre_turn_follow_ups: std::collections::VecDeque<String> =
            std::collections::VecDeque::new();

        if let Some(nudge) = mana_skill_follow_up_hint(
            &prompt,
            self.mode,
            !self.tools.is_empty(),
            self.has_mana_skill,
            self.has_mana_basics_skill,
            self.has_mana_delegation_skill,
        ) {
            queued_pre_turn_follow_ups.push_back(nudge.to_string());
        }

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

            if turn > 0 {
                if let Some(follow_up) = queued_pre_turn_follow_ups.pop_front() {
                    self.messages.push(Message::user(&follow_up));
                }
            }

            // Check for commands between turns (non-blocking)
            while let Ok(cmd) = self.command_rx.try_recv() {
                match cmd {
                    AgentCommand::Cancel => {
                        self.cancel_token
                            .store(true, std::sync::atomic::Ordering::Relaxed);
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
            if let Ok(mut review) = self.turn_mana_review.lock() {
                review.begin_turn(turn);
            }
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
                // Use configured output cap when present; otherwise let providers
                // choose their own sensible default output budget.
                max_tokens: self.max_tokens,
                temperature: None,
                system_prompt: self.system_prompt.clone(),
                tools: self.tools.definitions(),
                cache_options: self.cache_options.clone(),
                effort: None,
            };

            self.hooks.fire(&HookEvent::BeforeLlmCall).await;

            // Pre-flight OAuth token refresh: if we have an auth store and the
            // token is expired, refresh it before making the API call. This
            // avoids wasting a round-trip on a guaranteed 401.
            if let Some(ref auth_store) = self.auth_store {
                let mut store = auth_store.lock().await;
                if store.is_oauth_expired("anthropic") {
                    match store.resolve_with_refresh("anthropic").await {
                        Ok(new_key) => {
                            self.api_key = new_key;
                        }
                        Err(e) => {
                            let message = format!(
                                "OAuth token refresh failed before request: {e}. Continuing with existing credentials."
                            );
                            let _ = self.ui.notify(&message, NotifyLevel::Warning).await;
                        }
                    }
                }
            }

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
            let mut saw_provider_message_end = false;
            let cancel_token = Arc::clone(&self.cancel_token);
            cancel_token.store(false, std::sync::atomic::Ordering::Relaxed);

            while let Some(event_result) = stream.next().await {
                // Check for cancel during event processing
                while let Ok(cmd) = self.command_rx.try_recv() {
                    match cmd {
                        AgentCommand::Cancel => {
                            cancel_token.store(true, std::sync::atomic::Ordering::Relaxed);
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
                                saw_provider_message_end = true;
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
                                    error: format!(
                                        "Provider stream failed after partial output: {error}"
                                    ),
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
                                let mana_review = self.finish_turn_mana_review(turn);
                                self.emit(AgentEvent::TurnEnd {
                                    index: turn,
                                    message: err_msg,
                                    mana_review,
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
                        let error = match &e {
                            imp_llm::Error::Stream(message) => {
                                format!("Provider stream failed after partial output: {message}")
                            }
                            _ => e.to_string(),
                        };
                        self.emit(AgentEvent::Error {
                            error: error.clone(),
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
                let mana_review = self.finish_turn_mana_review(turn);
                self.emit(AgentEvent::TurnEnd {
                    index: turn,
                    message: partial,
                    mana_review,
                })
                .await;
                break;
            }

            // Use the MessageEnd message if provided; otherwise the provider
            // stream ended without a terminal completion event and should be
            // treated as an error rather than silently synthesized as success.
            let msg = match assistant_msg {
                Some(message) => message,
                None if !saw_provider_message_end => {
                    let error = format!(
                        "Provider stream ended unexpectedly before completing the message (missing terminal completion event after {} content block(s) and {} tool call(s))",
                        ordered_content.len(),
                        tool_calls.len()
                    );
                    self.emit(AgentEvent::Error {
                        error: error.clone(),
                    })
                    .await;
                    let cost = total_usage.cost(&self.model.meta.pricing);
                    self.emit(AgentEvent::AgentEnd {
                        usage: total_usage,
                        cost,
                    })
                    .await;
                    return Err(crate::error::Error::Llm(imp_llm::Error::Stream(error)));
                }
                None => build_assistant_message(&ordered_content, &tool_calls, None),
            };

            self.messages.push(Message::Assistant(msg.clone()));

            if tool_calls.is_empty() {
                // No tool calls — the model is done unless a queued follow-up exists.
                let mana_review = self.finish_turn_mana_review(turn);
                self.emit(AgentEvent::TurnEnd {
                    index: turn,
                    message: msg.clone(),
                    mana_review: mana_review.clone(),
                })
                .await;

                let assessment = self.assess_post_turn(&msg, &[], false, &mana_review);
                self.emit(AgentEvent::TurnAssessment {
                    index: turn,
                    assessment: assessment.debug_view(),
                })
                .await;
                let next_action = assessment.into_next_action();
                self.enqueue_next_action(&mut queued_follow_ups, next_action);

                if let Some(follow_up) = queued_follow_ups.pop_front() {
                    self.messages.push(Message::user(&follow_up));
                    turn += 1;
                    continue;
                }
                break;
            }

            // Execute tool calls
            let results = self.execute_tools(tool_calls, cancel_token).await;

            for result in &results {
                self.messages.push(Message::ToolResult(result.clone()));
            }

            let mana_review = self.finish_turn_mana_review(turn);
            self.emit(AgentEvent::TurnEnd {
                index: turn,
                message: msg.clone(),
                mana_review: mana_review.clone(),
            })
            .await;

            let assessment = self.assess_post_turn(&msg, &results, true, &mana_review);
            self.emit(AgentEvent::TurnAssessment {
                index: turn,
                assessment: assessment.debug_view(),
            })
            .await;
            let next_action = assessment.into_next_action();
            let should_stop_after_tool_turn = matches!(
                next_action,
                NextAction::Stop {
                    reason: NextActionStopReason::RepeatedAction,
                }
            );
            self.enqueue_next_action(&mut queued_follow_ups, next_action);

            if let Some(follow_up) = queued_follow_ups.pop_front() {
                self.messages.push(Message::user(&follow_up));
            }

            if should_stop_after_tool_turn {
                break;
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

    fn assess_post_turn(
        &self,
        message: &AssistantMessage,
        tool_results: &[imp_llm::ToolResultMessage],
        _used_tools: bool,
        mana_review: &TurnManaReview,
    ) -> PostTurnAssessment {
        let repeated_action = tool_results_indicate_repeated_action(tool_results);
        let runtime_execution_stop_reason =
            tool_results_indicate_execution_blocker(tool_results, self.mode);
        let work_completed = tool_results_indicate_work_completed(tool_results, self.mode);
        let execution_debt = tool_results_indicate_execution_debt(tool_results, self.mode);
        let execution_evidence = tool_results_indicate_execution_evidence(tool_results, self.mode);
        let planning_only_progress = execution_debt && !execution_evidence;
        let mana_stop_reason = mana_review_stop_reason(mana_review, self.mode);
        let planner_text_stop_reason = planner_stop_reason(message, self.mode);
        let execution_text_stop_reason = execution_stop_reason(message, self.mode);

        let continue_recommendation = if should_queue_mana_externalization_follow_up(
            message,
            self.mode,
            self.has_mana_skill,
            self.queued_mana_externalization_nudge,
        ) {
            Some(ContinueRecommendation {
                prompt: mana_externalization_follow_up_text().to_string(),
                reason: ContinueReason::ExternalizationNeeded,
            })
        } else if !matches!(self.mode, AgentMode::Planner)
            && should_queue_execution_debt_follow_up(
                execution_debt,
                execution_evidence,
                self.queued_execution_debt_follow_up,
                !assistant_message_text(message).trim().is_empty(),
            )
        {
            Some(ContinueRecommendation {
                prompt: execution_debt_follow_up_text().to_string(),
                reason: ContinueReason::ExecutionDebt,
            })
        } else if should_queue_confidence_continue_follow_up(
            message,
            self.mode,
            self.continue_policy,
            self.queued_confidence_continue_nudge,
        ) {
            Some(ContinueRecommendation {
                prompt: confidence_continue_follow_up_text().to_string(),
                reason: ContinueReason::HighConfidenceVisibleNextStep,
            })
        } else {
            None
        };

        PostTurnAssessment {
            runtime: RuntimeEvidence {
                repeated_action,
                execution_stop_reason: runtime_execution_stop_reason,
                work_completed,
                execution_debt,
                execution_evidence,
                planning_only_progress,
            },
            mana: ManaEvidence {
                stop_reason: mana_stop_reason,
            },
            text_fallback: TextFallbackEvidence {
                planner_stop_reason: planner_text_stop_reason,
                execution_stop_reason: execution_text_stop_reason,
            },
            continue_recommendation,
        }
    }

    fn enqueue_next_action(
        &mut self,
        queued_follow_ups: &mut std::collections::VecDeque<String>,
        next_action: NextAction,
    ) {
        match next_action {
            NextAction::Continue { prompt, reason } => {
                match reason {
                    ContinueReason::ExternalizationNeeded => {
                        self.queued_mana_externalization_nudge = true;
                    }
                    ContinueReason::HighConfidenceVisibleNextStep => {
                        self.queued_confidence_continue_nudge = true;
                    }
                    ContinueReason::ExecutionDebt => {
                        self.queued_execution_debt_follow_up = true;
                    }
                }
                queued_follow_ups.push_back(prompt);
            }
            NextAction::Stop { .. } => {}
        }
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
            AgentEvent::TurnEnd { index, message, .. } => {
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
        cancel_token: Arc<std::sync::atomic::AtomicBool>,
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

        let mut results = join_all(readonly.into_iter().map(|(index, id, name, args)| {
            let cancel_token = Arc::clone(&cancel_token);
            async move {
                let result = self.execute_one_tool(&id, &name, args, cancel_token).await;
                (index, result)
            }
        }))
        .await;

        for (index, id, name, args) in mutable {
            let result = self
                .execute_one_tool(&id, &name, args, Arc::clone(&cancel_token))
                .await;
            results.push((index, result));
        }

        results.sort_by_key(|(index, _)| *index);
        results.into_iter().map(|(_, result)| result).collect()
    }

    fn repeated_tool_call_check(
        &self,
        call_id: &str,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> RepeatedToolCallCheck {
        let args_json = serde_json::to_string(args).unwrap_or_else(|_| "<unserializable>".into());
        let mut state = match self.last_tool_call.lock() {
            Ok(s) => s,
            Err(_) => return RepeatedToolCallCheck::Ok,
        };

        let consecutive = match state.as_mut() {
            Some(prev) if prev.tool_name == tool_name && prev.args_json == args_json => {
                prev.consecutive += 1;
                prev.consecutive
            }
            _ => {
                *state = Some(RepeatedToolCallState {
                    tool_name: tool_name.to_string(),
                    args_json,
                    consecutive: 1,
                });
                1
            }
        };

        if consecutive == 3 {
            return RepeatedToolCallCheck::Warn(format!(
                "Warning: identical tool call repeated 3 times in a row for '{tool_name}'. The result may not have changed. Consider using the information you already have or trying a different action."
            ));
        }

        if consecutive >= 4 {
            return RepeatedToolCallCheck::Block(
                crate::tools::ToolOutput::error(format!(
                    "Blocked: identical tool call repeated {consecutive} times in a row for '{tool_name}'. The result likely has not changed. Use the information you already have or try a different action."
                ))
                .into_tool_result(call_id, tool_name),
            );
        }

        RepeatedToolCallCheck::Ok
    }

    async fn execute_one_tool(
        &self,
        call_id: &str,
        tool_name: &str,
        args: serde_json::Value,
        cancel_token: Arc<std::sync::atomic::AtomicBool>,
    ) -> imp_llm::ToolResultMessage {
        let repeat_check = self.repeated_tool_call_check(call_id, tool_name, &args);
        if let RepeatedToolCallCheck::Block(loop_result) = repeat_check {
            self.emit(AgentEvent::ToolExecutionStart {
                tool_call_id: call_id.to_string(),
                tool_name: tool_name.to_string(),
                args: args.clone(),
            })
            .await;
            self.emit(AgentEvent::ToolExecutionEnd {
                tool_call_id: call_id.to_string(),
                result: loop_result.clone(),
            })
            .await;
            return loop_result;
        }

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
                    cancelled: Arc::clone(&cancel_token),
                    update_tx,
                    command_tx: self.command_tx.clone(),
                    ui: self.ui.clone(),
                    file_cache: self.file_cache.clone(),
                    checkpoint_state: self.checkpoint_state.clone(),
                    file_tracker: self.file_tracker.clone(),
                    anchor_store: self.anchor_store.clone(),
                    lua_tool_loader: self.lua_tool_loader.clone(),
                    mode: self.mode,
                    read_max_lines: self.read_max_lines,
                    turn_mana_review: self.turn_mana_review.clone(),
                    config: self.config.clone(),
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
                forwarder.await.ok();
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

        if let RepeatedToolCallCheck::Warn(warning) = repeat_check {
            result.content.push(imp_llm::ContentBlock::Text {
                text: format!("\n\n{warning}"),
            });
        }

        result
    }

    fn finish_turn_mana_review(&self, turn: u32) -> TurnManaReview {
        match self.turn_mana_review.lock() {
            Ok(review) => {
                let review = review.finalize();
                if review.turn_index == turn {
                    review
                } else {
                    TurnManaReview::no_change(turn)
                }
            }
            Err(_) => TurnManaReview::no_change(turn),
        }
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

fn assistant_message_text(message: &AssistantMessage) -> String {
    message
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn assistant_message_contains_mana_tool_call(message: &AssistantMessage) -> bool {
    message.content.iter().any(|block| match block {
        ContentBlock::ToolCall { name, .. } => name == "mana",
        _ => false,
    })
}

fn should_queue_execution_debt_follow_up(
    execution_debt: bool,
    execution_evidence: bool,
    already_queued: bool,
    assistant_finalized: bool,
) -> bool {
    execution_debt && !execution_evidence && !already_queued && assistant_finalized
}

fn should_queue_mana_externalization_follow_up(
    message: &AssistantMessage,
    mode: AgentMode,
    has_mana_skill: bool,
    already_queued: bool,
) -> bool {
    if already_queued || !has_mana_skill {
        return false;
    }

    if !matches!(
        mode,
        AgentMode::Full | AgentMode::Planner | AgentMode::Orchestrator
    ) {
        return false;
    }

    if assistant_message_contains_mana_tool_call(message) {
        return false;
    }

    let text = assistant_message_text(message);
    if text.trim().is_empty() {
        return false;
    }

    let lower = text.to_ascii_lowercase();
    let planning_signal = [
        "plan",
        "phase",
        "rollout",
        "decompose",
        "break",
        "split",
        "architecture",
        "migration",
        "follow-up",
        "next step",
        "next steps",
        "dependency",
        "dependencies",
        "verify",
        "acceptance",
    ]
    .iter()
    .any(|needle| lower.contains(needle));

    planning_signal
}

fn mana_externalization_follow_up_text() -> &'static str {
    "Before you continue: externalize the durable plan or decomposition you just described into mana now. Create or update the relevant unit(s) with native mana actions, prefer root scope for cross-project work, and avoid extra chat restatement when the mana tool/UI already makes the delta obvious."
}

fn should_queue_confidence_continue_follow_up(
    message: &AssistantMessage,
    mode: AgentMode,
    continue_policy: ContinuePolicy,
    already_queued: bool,
) -> bool {
    if already_queued || matches!(continue_policy, ContinuePolicy::Disabled) {
        return false;
    }

    if !matches!(
        mode,
        AgentMode::Full | AgentMode::Planner | AgentMode::Orchestrator
    ) {
        return false;
    }

    if !assistant_message_contains_mana_tool_call(message) {
        return false;
    }

    let text = assistant_message_text(message);
    if text.trim().is_empty() {
        return false;
    }

    let lower = text.to_ascii_lowercase();
    let positive_signal = [
        "done",
        "completed",
        "finished",
        "updated",
        "created",
        "next",
        "continue",
        "proceed",
        "follow-up",
        "follow up",
    ]
    .iter()
    .filter(|needle| lower.contains(**needle))
    .count();

    let blocker_signal = [
        "blocked",
        "unclear",
        "need your input",
        "which should",
        "approval",
    ]
    .iter()
    .any(|needle| lower.contains(needle));

    if blocker_signal {
        return false;
    }

    let threshold = match continue_policy {
        ContinuePolicy::Disabled => return false,
        ContinuePolicy::Conservative => 3,
        ContinuePolicy::Balanced => 2,
        ContinuePolicy::Aggressive => 1,
    };

    positive_signal >= threshold
}

fn confidence_continue_follow_up_text() -> &'static str {
    "Confidence is high and the mana delta is already visible. Continue to the next small, well-bounded step now using native mana-backed workflow, unless a consequential decision or blocker appears. Do not re-summarize the same visible mana change in chat unless new context needs to be called out."
}

fn execution_debt_follow_up_text() -> &'static str {
    "You have recorded or planned work, but the requested outcome is not satisfied yet. Continue working until the user's requested outcome is satisfied, or until concrete evidence shows it cannot be completed. Do not stop merely because you recorded a plan, updated mana, or completed one intermediate step."
}

fn tool_results_indicate_repeated_action(tool_results: &[imp_llm::ToolResultMessage]) -> bool {
    tool_results.iter().any(|result| {
        result.is_error
            && result.content.iter().any(|block| match block {
                ContentBlock::Text { text } => {
                    text.contains("Blocked: identical tool call repeated")
                }
                _ => false,
            })
    })
}

fn tool_results_indicate_execution_blocker(
    tool_results: &[imp_llm::ToolResultMessage],
    mode: AgentMode,
) -> Option<NextActionStopReason> {
    if !matches!(
        mode,
        AgentMode::Full | AgentMode::Orchestrator | AgentMode::Worker
    ) {
        return None;
    }

    let saw_edit_like_success = tool_results.iter().any(|result| {
        !result.is_error && matches!(result.tool_name.as_str(), "write" | "edit" | "multi_edit")
    });

    for result in tool_results {
        let action = result.details.get("action").and_then(|v| v.as_str());

        if action == Some("verify")
            && result.details.get("passed").and_then(|v| v.as_bool()) == Some(false)
        {
            return Some(NextActionStopReason::ExecutionBlocked);
        }

        if result.tool_name == "ask_user" && !result.is_error {
            return Some(NextActionStopReason::UserBlocker);
        }

        if result.tool_name == "bash" || result.tool_name == "shell" {
            let exit_code = result.details.get("exit_code").and_then(|v| v.as_i64());
            let timed_out = result.details.get("timed_out").and_then(|v| v.as_bool()) == Some(true);
            let cancelled = result.details.get("cancelled").and_then(|v| v.as_bool()) == Some(true);
            let command = result
                .details
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let looks_like_check = command.contains("check")
                || command.contains("test")
                || command.contains("verify")
                || command.contains("pytest")
                || command.contains("cargo test")
                || command.contains("cargo check");

            if looks_like_check
                && (timed_out || cancelled || exit_code.is_some_and(|code| code != 0))
            {
                return Some(NextActionStopReason::ExecutionBlocked);
            }

            if saw_edit_like_success
                && (timed_out || cancelled || exit_code.is_some_and(|code| code != 0))
            {
                return Some(NextActionStopReason::ExecutionBlocked);
            }
        }
    }

    None
}

fn tool_results_indicate_execution_debt(
    tool_results: &[imp_llm::ToolResultMessage],
    mode: AgentMode,
) -> bool {
    if !matches!(
        mode,
        AgentMode::Full | AgentMode::Orchestrator | AgentMode::Worker
    ) {
        return false;
    }

    tool_results.iter().any(|result| {
        !result.is_error
            && result.tool_name == "mana"
            && matches!(
                result.details.get("action").and_then(|v| v.as_str()),
                Some("create" | "update" | "notes_append" | "decision_add" | "dep_add" | "claim")
            )
    })
}

fn tool_results_indicate_execution_evidence(
    tool_results: &[imp_llm::ToolResultMessage],
    mode: AgentMode,
) -> bool {
    if !matches!(
        mode,
        AgentMode::Full | AgentMode::Orchestrator | AgentMode::Worker
    ) {
        return false;
    }

    tool_results.iter().any(|result| {
        if result.is_error {
            return false;
        }

        match result.tool_name.as_str() {
            "write" | "edit" | "multi_edit" | "openrouter_secret_run" => true,
            "bash" | "shell" => true,
            "mana" => matches!(
                result.details.get("action").and_then(|v| v.as_str()),
                Some("run" | "verify" | "close" | "fail")
            ),
            _ => false,
        }
    })
}

fn tool_results_indicate_work_completed(
    tool_results: &[imp_llm::ToolResultMessage],
    mode: AgentMode,
) -> bool {
    if !matches!(
        mode,
        AgentMode::Full | AgentMode::Orchestrator | AgentMode::Worker
    ) {
        return false;
    }

    let mut saw_edit_like_success = false;
    let mut saw_successful_check = false;

    for result in tool_results {
        if result.is_error {
            continue;
        }

        if matches!(result.tool_name.as_str(), "write" | "edit" | "multi_edit") {
            saw_edit_like_success = true;
        }

        let action = result.details.get("action").and_then(|v| v.as_str());
        let has_closed_unit = result
            .details
            .get("unit")
            .and_then(|unit| unit.get("status"))
            .and_then(|v| v.as_str())
            == Some("closed");

        if let Some(command) = result.details.get("command").and_then(|v| v.as_str()) {
            let exit_code_ok = result.details.get("exit_code").and_then(|v| v.as_i64()) == Some(0);
            let command_lower = command.to_ascii_lowercase();
            let looks_like_check = command_lower.contains("check")
                || command_lower.contains("test")
                || command_lower.contains("verify")
                || command_lower.contains("pytest")
                || command_lower.contains("cargo test")
                || command_lower.contains("cargo check");
            if exit_code_ok && looks_like_check {
                saw_successful_check = true;
            }
        }

        match action {
            Some("close") => return true,
            Some("verify")
                if result.details.get("passed").and_then(|v| v.as_bool()) == Some(true) =>
            {
                return true;
            }
            _ if has_closed_unit => return true,
            _ => {}
        }
    }

    saw_edit_like_success && saw_successful_check
}

fn mana_review_stop_reason(
    mana_review: &TurnManaReview,
    mode: AgentMode,
) -> Option<NextActionStopReason> {
    match mana_review.state {
        ManaReviewState::NeedsDecision => Some(NextActionStopReason::UserBlocker),
        ManaReviewState::Changed if matches!(mode, AgentMode::Planner) => {
            if !mana_review.proposed_children.is_empty()
                || !mana_review.touched_units.is_empty()
                || !mana_review.material_field_changes.is_empty()
                || !mana_review.notes_appended.is_empty()
                || !mana_review.decision_events.is_empty()
            {
                Some(NextActionStopReason::DecompositionCompleted)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn planner_stop_reason(
    message: &AssistantMessage,
    mode: AgentMode,
) -> Option<NextActionStopReason> {
    if !matches!(mode, AgentMode::Planner) {
        return None;
    }

    classify_stop_reason_from_text(message, true)
}

fn execution_stop_reason(
    message: &AssistantMessage,
    mode: AgentMode,
) -> Option<NextActionStopReason> {
    if !matches!(
        mode,
        AgentMode::Full | AgentMode::Orchestrator | AgentMode::Worker
    ) {
        return None;
    }

    match classify_stop_reason_from_text(message, false) {
        Some(
            reason @ (NextActionStopReason::UserBlocker | NextActionStopReason::WorkCompleted),
        ) => Some(reason),
        _ => None,
    }
}

fn classify_stop_reason_from_text(
    message: &AssistantMessage,
    planner_mode: bool,
) -> Option<NextActionStopReason> {
    let text = assistant_message_text(message);
    if text.trim().is_empty() {
        return None;
    }

    let lower = text.to_ascii_lowercase();

    let blocker_signal = [
        "blocked",
        "need your input",
        "which should",
        "waiting on you",
        "approval",
        "before i continue",
        "before continuing",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    if blocker_signal {
        return Some(NextActionStopReason::UserBlocker);
    }

    if planner_mode {
        let decomposition_complete_signal = [
            "externalized into mana",
            "created the units",
            "created child units",
            "decomposition is complete",
            "plan is complete",
            "ready for handoff",
        ]
        .iter()
        .any(|needle| lower.contains(needle));
        if decomposition_complete_signal {
            return Some(NextActionStopReason::DecompositionCompleted);
        }
    } else {
        let work_complete_signal = [
            "all done",
            "done",
            "completed",
            "finished",
            "implemented",
            "fixed",
            "handled",
        ]
        .iter()
        .any(|needle| lower.contains(needle));
        if work_complete_signal {
            return Some(NextActionStopReason::WorkCompleted);
        }
    }

    None
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
    let rest = trimmed.strip_prefix("mana")?;
    if rest.chars().next().is_some_and(|c| !c.is_whitespace()) {
        return None;
    }

    let action = rest.split_whitespace().next().unwrap_or("");
    match action {
        "status" | "list" | "ls" | "show" | "read" | "create" | "close" | "update" | "run"
        | "run_state" | "evaluate" | "agents" | "logs" | "next" | "claim" | "release" | "tree" => {
            Some("Use the native mana tool instead of `bash` for this mana command. For orchestration, the native tool supports canonical target selection (`id`, `targets`, or all ready work) plus background run tracking.")
        }
        _ => None,
    }
}

fn mana_skill_follow_up_hint(
    prompt: &str,
    mode: AgentMode,
    tools_available: bool,
    has_mana_skill: bool,
    has_mana_basics_skill: bool,
    _has_mana_delegation_skill: bool,
) -> Option<&'static str> {
    if !tools_available {
        return None;
    }

    let lower = prompt.to_ascii_lowercase();

    let orchestration_signal = [
        "spawn",
        "delegate",
        "decompose",
        "decomposition",
        "split this",
        "break this up",
        "break it up",
        "parallel",
        "spawn workers",
        "spawn worker",
        "worker spawn",
        "orchestrate",
        "orchestration",
        "create a unit",
        "create units",
        "mana run",
    ]
    .iter()
    .any(|needle| lower.contains(needle));

    let mana_signal = [
        " mana ",
        "mana status",
        "mana list",
        "mana show",
        "mana update",
        "mana create",
        "mana run",
        "unit",
        "units",
    ]
    .iter()
    .any(|needle| lower.contains(needle));

    match mode {
        AgentMode::Full | AgentMode::Orchestrator | AgentMode::Planner
            if orchestration_signal || mana_signal =>
        {
            if has_mana_skill {
                Some("Before you continue: load `mana` with `read` and follow it for unit design, decomposition, retries, and worker handoff.")
            } else {
                None
            }
        }
        AgentMode::Worker | AgentMode::Auditor if mana_signal => {
            if has_mana_basics_skill {
                Some("Before you continue: load `mana-basics` with `read` and follow the allowed native mana workflow for this mode.")
            } else if has_mana_skill {
                Some("Before you continue: load `mana` with `read` and follow the allowed native mana workflow for this mode.")
            } else {
                None
            }
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

    #[test]
    fn agent_queues_mana_hint_for_planner_requests() {
        let provider = Arc::new(MockProvider::new(vec![
            text_response("Loaded mana skill", 100, 20),
            text_response("Done", 120, 25),
        ]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.has_mana_skill = true;
        agent.mode = AgentMode::Planner;

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            agent
                .run("Please split this into units for workers".to_string())
                .await
                .unwrap();
        });

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

        assert_eq!(user_texts.len(), 1);
        assert_eq!(user_texts[0], "Please split this into units for workers");
    }

    #[tokio::test]
    async fn agent_queues_mana_externalization_follow_up_after_planning_turn() {
        let provider = Arc::new(MockProvider::new(vec![
            text_response("Here is the plan: split this into phases, add dependencies, and define verify steps.", 100, 20),
            text_response("Externalized into mana.", 120, 25),
        ]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.has_mana_skill = true;
        agent.mode = AgentMode::Planner;

        agent.run("Plan the rollout".to_string()).await.unwrap();

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

        assert_eq!(user_texts.len(), 2);
        assert_eq!(user_texts[0], "Plan the rollout");
        assert!(user_texts[1].contains("externalize the durable plan"));
    }

    #[tokio::test]
    async fn turn_assessment_debug_view_reports_execution_blocker() {
        let (agent, _handle) = Agent::new(
            test_model(Arc::new(MockProvider::new(vec![]))),
            PathBuf::from("/tmp"),
        );
        let assessment = agent.assess_post_turn(
            &AssistantMessage {
                content: vec![ContentBlock::Text {
                    text: "Verify failed.".to_string(),
                }],
                usage: None,
                stop_reason: StopReason::EndTurn,
                timestamp: 0,
            },
            &[imp_llm::ToolResultMessage {
                tool_call_id: "call_verify".to_string(),
                tool_name: "mana".to_string(),
                content: vec![ContentBlock::Text {
                    text: "Verify failed".to_string(),
                }],
                is_error: true,
                details: serde_json::json!({
                    "action": "verify",
                    "passed": false,
                    "exit_code": 1
                }),
                timestamp: 0,
            }],
            true,
            &TurnManaReview::no_change(0),
        );

        let debug = assessment.debug_view();
        assert_eq!(
            debug.runtime.execution_stop_reason.as_deref(),
            Some("execution_blocked")
        );
        assert_eq!(
            debug.chosen_action,
            NextActionDebugView::Stop {
                reason: "execution_blocked".to_string(),
            }
        );
    }

    #[test]
    fn turn_assessment_debug_view_reports_continue_recommendation() {
        let assessment = PostTurnAssessment {
            runtime: RuntimeEvidence {
                repeated_action: false,
                execution_stop_reason: None,
                work_completed: false,
                execution_debt: false,
                execution_evidence: false,
                planning_only_progress: false,
            },
            mana: ManaEvidence { stop_reason: None },
            text_fallback: TextFallbackEvidence {
                planner_stop_reason: None,
                execution_stop_reason: None,
            },
            continue_recommendation: Some(ContinueRecommendation {
                prompt: "continue".to_string(),
                reason: ContinueReason::HighConfidenceVisibleNextStep,
            }),
        };

        let debug = assessment.debug_view();
        let recommendation = debug
            .continue_recommendation
            .expect("continue recommendation present");
        assert_eq!(recommendation.reason, "high_confidence_visible_next_step");
        assert!(matches!(
            debug.chosen_action,
            NextActionDebugView::Continue { .. }
        ));
    }

    #[tokio::test]
    async fn emits_turn_assessment_event_for_execution_blocker() {
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_check",
                "bash",
                serde_json::json!({"command": "cargo check -p definitely_missing_crate", "timeout": 1}),
                100,
                20,
            ),
            text_response("The check failed.", 120, 20),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.mode = AgentMode::Full;
        agent.tools.register(Arc::new(crate::tools::bash::BashTool));

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("Run the check".to_string()).await.unwrap();
        drop(agent);
        let events = events_task.await.unwrap();

        let assessment = events.iter().find_map(|event| match event {
            AgentEvent::TurnAssessment { assessment, .. } => Some(assessment),
            _ => None,
        });

        let assessment = assessment.expect("turn assessment emitted");
        assert_eq!(
            assessment.runtime.execution_stop_reason.as_deref(),
            Some("execution_blocked")
        );
        assert_eq!(
            assessment.chosen_action,
            NextActionDebugView::Stop {
                reason: "execution_blocked".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn emits_turn_assessment_event_for_continue_recommendation() {
        let provider = Arc::new(MockProvider::new(vec![
            vec![
                StreamEvent::MessageStart {
                    model: "test-model".to_string(),
                },
                StreamEvent::ToolCall {
                    id: "call_1".to_string(),
                    name: "mana".to_string(),
                    arguments: serde_json::json!({"action": "update", "id": "1", "notes": "done"}),
                },
                StreamEvent::TextDelta {
                    text: "Done. Updated mana and next step is ready to continue.".to_string(),
                },
                StreamEvent::MessageEnd {
                    message: AssistantMessage {
                        content: vec![
                            ContentBlock::ToolCall {
                                id: "call_1".to_string(),
                                name: "mana".to_string(),
                                arguments: serde_json::json!({"action": "update", "id": "1", "notes": "done"}),
                            },
                            ContentBlock::Text {
                                text: "Done. Updated mana and next step is ready to continue."
                                    .to_string(),
                            },
                        ],
                        usage: Some(Usage {
                            input_tokens: 100,
                            output_tokens: 20,
                            cache_read_tokens: 0,
                            cache_write_tokens: 0,
                        }),
                        stop_reason: StopReason::ToolUse,
                        timestamp: 1000,
                    },
                },
            ],
            text_response("Stopped after visible mana turn.", 120, 25),
        ]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.mode = AgentMode::Planner;
        agent.continue_policy = ContinuePolicy::Balanced;
        agent
            .tools
            .register(Arc::new(crate::tools::mana::ManaTool::default()));

        let events_task = tokio::spawn(collect_events(handle));
        agent.run("Do the next thing".to_string()).await.unwrap();
        drop(agent);
        let events = events_task.await.unwrap();

        let assessment = events.iter().find_map(|event| match event {
            AgentEvent::TurnAssessment { assessment, .. } => Some(assessment),
            _ => None,
        });

        let assessment = assessment.expect("turn assessment emitted");
        let recommendation = assessment
            .continue_recommendation
            .as_ref()
            .expect("continue recommendation present");
        assert_eq!(recommendation.reason, "high_confidence_visible_next_step");
        assert!(matches!(
            assessment.chosen_action,
            NextActionDebugView::Continue { .. }
        ));
    }

    #[test]
    fn post_turn_assessment_prefers_execution_blocker_over_completion() {
        let assessment = PostTurnAssessment {
            runtime: RuntimeEvidence {
                repeated_action: false,
                execution_stop_reason: Some(NextActionStopReason::ExecutionBlocked),
                work_completed: true,
                execution_debt: false,
                execution_evidence: false,
                planning_only_progress: false,
            },
            mana: ManaEvidence {
                stop_reason: Some(NextActionStopReason::DecompositionCompleted),
            },
            text_fallback: TextFallbackEvidence {
                planner_stop_reason: Some(NextActionStopReason::DecompositionCompleted),
                execution_stop_reason: Some(NextActionStopReason::WorkCompleted),
            },
            continue_recommendation: Some(ContinueRecommendation {
                prompt: "continue".to_string(),
                reason: ContinueReason::HighConfidenceVisibleNextStep,
            }),
        };

        assert_eq!(
            assessment.into_next_action(),
            NextAction::Stop {
                reason: NextActionStopReason::ExecutionBlocked,
            }
        );
    }

    #[test]
    fn post_turn_assessment_emits_continue_when_no_stop_reason_exists() {
        let assessment = PostTurnAssessment {
            runtime: RuntimeEvidence {
                repeated_action: false,
                execution_stop_reason: None,
                work_completed: false,
                execution_debt: false,
                execution_evidence: false,
                planning_only_progress: false,
            },
            mana: ManaEvidence { stop_reason: None },
            text_fallback: TextFallbackEvidence {
                planner_stop_reason: None,
                execution_stop_reason: None,
            },
            continue_recommendation: Some(ContinueRecommendation {
                prompt: "continue".to_string(),
                reason: ContinueReason::HighConfidenceVisibleNextStep,
            }),
        };

        assert_eq!(
            assessment.into_next_action(),
            NextAction::Continue {
                prompt: "continue".to_string(),
                reason: ContinueReason::HighConfidenceVisibleNextStep,
            }
        );
    }

    #[test]
    fn execution_debt_follow_up_is_preferred_before_stopping_for_planning_only_progress() {
        let assessment = PostTurnAssessment {
            runtime: RuntimeEvidence {
                repeated_action: false,
                execution_stop_reason: None,
                work_completed: false,
                execution_debt: true,
                execution_evidence: false,
                planning_only_progress: false,
            },
            mana: ManaEvidence { stop_reason: None },
            text_fallback: TextFallbackEvidence {
                planner_stop_reason: None,
                execution_stop_reason: None,
            },
            continue_recommendation: Some(ContinueRecommendation {
                prompt: execution_debt_follow_up_text().to_string(),
                reason: ContinueReason::ExecutionDebt,
            }),
        };

        assert_eq!(
            assessment.into_next_action(),
            NextAction::Continue {
                prompt: execution_debt_follow_up_text().to_string(),
                reason: ContinueReason::ExecutionDebt,
            }
        );
    }

    #[test]
    fn mana_planning_without_execution_creates_execution_debt_follow_up() {
        let result = imp_llm::ToolResultMessage {
            tool_call_id: "call_mana".to_string(),
            tool_name: "mana".to_string(),
            content: vec![ContentBlock::Text {
                text: "Created task".to_string(),
            }],
            is_error: false,
            details: serde_json::json!({ "action": "create" }),
            timestamp: 0,
        };

        assert!(tool_results_indicate_execution_debt(
            std::slice::from_ref(&result),
            AgentMode::Full
        ));
        assert!(!tool_results_indicate_execution_evidence(
            std::slice::from_ref(&result),
            AgentMode::Full
        ));
        assert!(should_queue_execution_debt_follow_up(
            true, false, false, true
        ));
    }

    #[test]
    fn mutating_tool_call_satisfies_execution_evidence() {
        let result = imp_llm::ToolResultMessage {
            tool_call_id: "call_edit".to_string(),
            tool_name: "edit".to_string(),
            content: vec![ContentBlock::Text {
                text: "diff".to_string(),
            }],
            is_error: false,
            details: serde_json::json!({ "path": "src/lib.rs" }),
            timestamp: 0,
        };

        assert!(tool_results_indicate_execution_evidence(
            &[result],
            AgentMode::Full
        ));
        assert!(!should_queue_execution_debt_follow_up(
            true, true, false, true
        ));
    }

    #[test]
    fn tool_results_indicate_execution_blocker_detects_failed_verify() {
        let result = imp_llm::ToolResultMessage {
            tool_call_id: "call_verify".to_string(),
            tool_name: "mana".to_string(),
            content: vec![ContentBlock::Text {
                text: "Verify failed".to_string(),
            }],
            is_error: true,
            details: serde_json::json!({
                "action": "verify",
                "passed": false,
                "exit_code": 1
            }),
            timestamp: 0,
        };

        assert_eq!(
            tool_results_indicate_execution_blocker(&[result], AgentMode::Full),
            Some(NextActionStopReason::ExecutionBlocked)
        );
    }

    #[test]
    fn tool_results_indicate_execution_blocker_detects_ask_tool_as_user_blocker() {
        let result = imp_llm::ToolResultMessage {
            tool_call_id: "call_ask".to_string(),
            tool_name: "ask_user".to_string(),
            content: vec![ContentBlock::Text {
                text: "blue".to_string(),
            }],
            is_error: false,
            details: serde_json::Value::Null,
            timestamp: 0,
        };

        assert_eq!(
            tool_results_indicate_execution_blocker(&[result], AgentMode::Full),
            Some(NextActionStopReason::UserBlocker)
        );
    }

    #[test]
    fn tool_results_indicate_work_completed_detects_edit_plus_successful_check() {
        let edit_result = imp_llm::ToolResultMessage {
            tool_call_id: "call_edit".to_string(),
            tool_name: "edit".to_string(),
            content: vec![ContentBlock::Text {
                text: "diff output".to_string(),
            }],
            is_error: false,
            details: serde_json::json!({
                "path": "/tmp/file.rs"
            }),
            timestamp: 0,
        };
        let check_result = imp_llm::ToolResultMessage {
            tool_call_id: "call_check".to_string(),
            tool_name: "bash".to_string(),
            content: vec![ContentBlock::Text {
                text: "ok".to_string(),
            }],
            is_error: false,
            details: serde_json::json!({
                "exit_code": 0,
                "command": "cargo check -p imp-core"
            }),
            timestamp: 0,
        };

        assert!(tool_results_indicate_work_completed(
            &[edit_result, check_result],
            AgentMode::Full
        ));
    }

    #[test]
    fn tool_results_indicate_work_completed_detects_closed_unit_details() {
        let result = imp_llm::ToolResultMessage {
            tool_call_id: "call_close".to_string(),
            tool_name: "mana".to_string(),
            content: vec![ContentBlock::Text {
                text: "Closed unit 1".to_string(),
            }],
            is_error: false,
            details: serde_json::json!({
                "action": "close",
                "unit": {
                    "id": "1",
                    "title": "Test unit",
                    "status": "closed"
                }
            }),
            timestamp: 0,
        };

        assert!(tool_results_indicate_work_completed(
            &[result],
            AgentMode::Full
        ));
    }

    #[test]
    fn mana_review_needs_decision_maps_to_user_blocker() {
        let review = TurnManaReview {
            turn_index: 0,
            state: ManaReviewState::NeedsDecision,
            scope: crate::mana_review::ManaReviewScope::default(),
            anchor_unit: None,
            touched_units: Vec::new(),
            proposed_children: Vec::new(),
            material_field_changes: Vec::new(),
            notes_appended: Vec::new(),
            decision_events: Vec::new(),
            unresolved_consequential_choices: Vec::new(),
            next_question: Some("Which path should we take?".to_string()),
        };

        assert_eq!(
            mana_review_stop_reason(&review, AgentMode::Planner),
            Some(NextActionStopReason::UserBlocker)
        );
    }

    #[test]
    fn mana_review_changed_with_planner_children_maps_to_decomposition_completed() {
        let review = TurnManaReview {
            turn_index: 0,
            state: ManaReviewState::Changed,
            scope: crate::mana_review::ManaReviewScope::default(),
            anchor_unit: None,
            touched_units: Vec::new(),
            proposed_children: vec![crate::mana_review::TurnManaProposedChild {
                unit: crate::mana_review::ManaUnitRef::new(
                    "28.6.1",
                    "child",
                    Some("job".to_string()),
                ),
                parent: crate::mana_review::ManaUnitRef::new(
                    "28.6",
                    "parent",
                    Some("epic".to_string()),
                ),
                child_kind: crate::mana_review::ManaReviewUnitKind::Job,
                child_origin: crate::mana_review::ManaUnitOrigin::CreatedInTurn,
            }],
            material_field_changes: Vec::new(),
            notes_appended: Vec::new(),
            decision_events: Vec::new(),
            unresolved_consequential_choices: Vec::new(),
            next_question: None,
        };

        assert_eq!(
            mana_review_stop_reason(&review, AgentMode::Planner),
            Some(NextActionStopReason::DecompositionCompleted)
        );
    }

    #[tokio::test]
    async fn planner_stops_after_decomposition_is_externalized() {
        let provider = Arc::new(MockProvider::new(vec![text_response(
            "Externalized into mana. Plan is complete and ready for handoff.",
            100,
            20,
        )]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.mode = AgentMode::Planner;
        agent.has_mana_skill = true;

        agent.run("Plan the rollout".to_string()).await.unwrap();

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

        assert_eq!(user_texts, vec!["Plan the rollout".to_string()]);
    }

    #[tokio::test]
    async fn planner_stops_for_user_blocker_instead_of_auto_follow_up() {
        let provider = Arc::new(MockProvider::new(vec![text_response(
            "Blocked: I need your input on which auth direction we should choose before continuing.",
            100,
            20,
        )]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.mode = AgentMode::Planner;
        agent.has_mana_skill = true;

        agent.run("Plan the rollout".to_string()).await.unwrap();

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

        assert_eq!(user_texts, vec!["Plan the rollout".to_string()]);
    }

    #[tokio::test]
    async fn agent_queues_confidence_continue_follow_up_after_visible_mana_turn() {
        let provider = Arc::new(MockProvider::new(vec![
            vec![
                StreamEvent::MessageStart {
                    model: "test-model".to_string(),
                },
                StreamEvent::ToolCall {
                    id: "call_1".to_string(),
                    name: "mana".to_string(),
                    arguments: serde_json::json!({"action": "update", "id": "1", "notes": "done"}),
                },
                StreamEvent::TextDelta {
                    text: "Done. Updated mana and next step is ready to continue.".to_string(),
                },
                StreamEvent::MessageEnd {
                    message: AssistantMessage {
                        content: vec![
                            ContentBlock::ToolCall {
                                id: "call_1".to_string(),
                                name: "mana".to_string(),
                                arguments: serde_json::json!({"action": "update", "id": "1", "notes": "done"}),
                            },
                            ContentBlock::Text {
                                text: "Done. Updated mana and next step is ready to continue."
                                    .to_string(),
                            },
                        ],
                        usage: Some(Usage {
                            input_tokens: 100,
                            output_tokens: 20,
                            cache_read_tokens: 0,
                            cache_write_tokens: 0,
                        }),
                        stop_reason: StopReason::ToolUse,
                        timestamp: 1000,
                    },
                },
            ],
            text_response("Continuing.", 120, 25),
        ]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.mode = AgentMode::Planner;
        agent.continue_policy = ContinuePolicy::Balanced;
        agent
            .tools
            .register(Arc::new(crate::tools::mana::ManaTool::default()));

        agent.run("Do the next thing".to_string()).await.unwrap();

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

        assert_eq!(user_texts.len(), 2);
        assert!(user_texts[1].contains("Confidence is high"));
    }

    #[tokio::test]
    async fn agent_does_not_queue_confidence_continue_when_policy_disabled() {
        let provider = Arc::new(MockProvider::new(vec![
            vec![
                StreamEvent::MessageStart {
                    model: "test-model".to_string(),
                },
                StreamEvent::ToolCall {
                    id: "call_1".to_string(),
                    name: "mana".to_string(),
                    arguments: serde_json::json!({"action": "update", "id": "1", "notes": "done"}),
                },
                StreamEvent::TextDelta {
                    text: "Done. Updated mana and next step is ready to continue.".to_string(),
                },
                StreamEvent::MessageEnd {
                    message: AssistantMessage {
                        content: vec![
                            ContentBlock::ToolCall {
                                id: "call_1".to_string(),
                                name: "mana".to_string(),
                                arguments: serde_json::json!({"action": "update", "id": "1", "notes": "done"}),
                            },
                            ContentBlock::Text {
                                text: "Done. Updated mana and next step is ready to continue."
                                    .to_string(),
                            },
                        ],
                        usage: Some(Usage {
                            input_tokens: 100,
                            output_tokens: 20,
                            cache_read_tokens: 0,
                            cache_write_tokens: 0,
                        }),
                        stop_reason: StopReason::ToolUse,
                        timestamp: 1000,
                    },
                },
            ],
            text_response("Stopped after visible mana turn.", 120, 25),
        ]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.mode = AgentMode::Planner;
        agent.continue_policy = ContinuePolicy::Disabled;
        agent
            .tools
            .register(Arc::new(crate::tools::mana::ManaTool::default()));

        agent.run("Do the next thing".to_string()).await.unwrap();

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

        assert_eq!(user_texts, vec!["Do the next thing".to_string()]);
    }

    #[tokio::test]
    async fn agent_does_not_queue_externalization_follow_up_after_mana_tool_turn() {
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_1",
                "mana",
                serde_json::json!({"action": "status"}),
                100,
                20,
            ),
            text_response("Done after mana", 120, 25),
        ]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.has_mana_skill = true;
        agent.mode = AgentMode::Planner;
        agent
            .tools
            .register(Arc::new(crate::tools::mana::ManaTool::default()));

        agent.run("Plan the rollout".to_string()).await.unwrap();

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

        assert_eq!(user_texts, vec!["Plan the rollout".to_string()]);
    }

    #[tokio::test]
    async fn agent_queues_mana_basics_hint_for_worker_mana_requests() {
        let provider = Arc::new(MockProvider::new(vec![
            text_response("Loaded basics skill", 100, 20),
            text_response("Done", 120, 25),
        ]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.has_mana_basics_skill = true;
        agent.mode = AgentMode::Worker;

        agent
            .run("Check mana status and logs for my unit".to_string())
            .await
            .unwrap();

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

        assert_eq!(user_texts.len(), 1);
        assert_eq!(user_texts[0], "Check mana status and logs for my unit");
    }

    #[tokio::test]
    async fn agent_does_not_queue_mana_hint_without_matching_signal() {
        let provider = Arc::new(MockProvider::new(vec![text_response("No nudge", 100, 20)]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.has_mana_skill = true;
        agent.mode = AgentMode::Planner;

        agent
            .run("Explain how this parser works".to_string())
            .await
            .unwrap();

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
            vec!["Explain how this parser works".to_string()]
        );
    }

    #[tokio::test]
    async fn agent_does_not_queue_mana_basics_hint_when_no_tools_available() {
        let provider = Arc::new(MockProvider::new(vec![text_response(
            "Loaded basics skill",
            100,
            20,
        )]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.has_mana_basics_skill = true;
        agent.mode = AgentMode::Worker;
        agent.tools.retain(|_| false);

        agent
            .run("Check mana status and logs for my unit".to_string())
            .await
            .unwrap();

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
            vec!["Check mana status and logs for my unit".to_string()]
        );
    }

    #[tokio::test]
    async fn single_text_turn_with_max_turns_one_and_no_tools_exits_cleanly() {
        let provider = Arc::new(MockProvider::new(vec![text_response("SMOKE_OK", 50, 10)]));
        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.max_turns = 1;
        agent.mode = AgentMode::Worker;
        agent.has_mana_basics_skill = true;
        agent.tools.retain(|_| false);

        let events_task = tokio::spawn(collect_events(handle));
        let result = agent.run("Check mana status and finish".to_string()).await;
        drop(agent);

        assert!(result.is_ok());

        let events = events_task.await.unwrap();
        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::AgentEnd { .. })));
        assert!(!events.iter().any(|e| matches!(
            e,
            AgentEvent::Error { error } if error.contains("Max turns exceeded")
        )));
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
        let error_idx = events.iter().position(|e| {
            matches!(
                e,
                AgentEvent::Error { error }
                if error.contains("Provider stream failed after partial output")
                    && error.contains("mid-stream failure")
            )
        });

        assert!(text_delta.is_some());
        assert!(error_idx.is_some());
        assert!(text_delta.unwrap() < error_idx.unwrap());
    }

    #[tokio::test]
    async fn agent_treats_silent_eof_without_message_end_as_error() {
        let provider = Arc::new(MockProvider::new_results(vec![vec![Ok(
            StreamEvent::TextDelta {
                text: "partial".to_string(),
            },
        )]]));

        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));

        let events_task = tokio::spawn(collect_events(handle));
        let result = agent.run("Fail with silent eof".to_string()).await;
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
        let error_idx = events.iter().position(|e| {
            matches!(
                e,
                AgentEvent::Error { error }
                if error.contains("missing terminal completion event")
            )
        });
        let turn_end_idx = events
            .iter()
            .position(|e| matches!(e, AgentEvent::TurnEnd { .. }));

        assert!(text_delta.is_some());
        assert!(error_idx.is_some());
        assert!(turn_end_idx.is_none());
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
    async fn execution_stops_after_failed_verify_tool_result_without_blocked_text() {
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_verify",
                "mana",
                serde_json::json!({"action": "verify", "id": "1"}),
                100,
                20,
            ),
            text_response("Verify failed.", 120, 20),
        ]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.mode = AgentMode::Full;
        agent
            .tools
            .register(Arc::new(crate::tools::mana::ManaTool::default()));

        agent.run("Verify the unit".to_string()).await.unwrap();

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

        assert_eq!(user_texts, vec!["Verify the unit".to_string()]);
    }

    #[tokio::test]
    async fn execution_stops_after_mana_close_tool_result_without_done_text() {
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_close",
                "mana",
                serde_json::json!({"action": "close", "id": "1"}),
                100,
                20,
            ),
            text_response("Unit closed.", 120, 20),
        ]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.mode = AgentMode::Full;
        agent
            .tools
            .register(Arc::new(crate::tools::mana::ManaTool::default()));

        agent.run("Close the unit".to_string()).await.unwrap();

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

        assert_eq!(user_texts, vec!["Close the unit".to_string()]);
    }

    #[tokio::test]
    async fn execution_stops_after_work_completed_text() {
        let provider = Arc::new(MockProvider::new(vec![text_response(
            "All done! Implemented the change and finished the task.",
            100,
            20,
        )]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.mode = AgentMode::Full;

        agent.run("Implement the change".to_string()).await.unwrap();

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

        assert_eq!(user_texts, vec!["Implement the change".to_string()]);
    }

    #[tokio::test]
    async fn execution_stops_for_user_blocker_text() {
        let provider = Arc::new(MockProvider::new(vec![text_response(
            "Blocked: I need your input on which path to take before continuing.",
            100,
            20,
        )]));

        let model = test_model(provider);
        let (mut agent, _handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.mode = AgentMode::Full;

        agent.run("Implement the change".to_string()).await.unwrap();

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

        assert_eq!(user_texts, vec!["Implement the change".to_string()]);
    }

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

    #[tokio::test]
    async fn single_text_turn_with_max_turns_one_exits_cleanly() {
        let provider = Arc::new(MockProvider::new(vec![text_response("SMOKE_OK", 50, 10)]));
        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, PathBuf::from("/tmp"));
        agent.max_turns = 1;

        let events_task = tokio::spawn(collect_events(handle));
        let result = agent.run("Reply once and stop".to_string()).await;
        drop(agent);

        assert!(result.is_ok());

        let events = events_task.await.unwrap();
        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::AgentEnd { .. })));
        assert!(!events.iter().any(|e| matches!(
            e,
            AgentEvent::Error { error } if error.contains("Max turns exceeded")
        )));
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
                AgentEvent::Warning { .. } => "Warning",
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
            seeded_messages.push(make_tool_result(&call_id, "read", &"x".repeat(400)));
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
                    events.into_iter().map(imp_llm::Result::Ok),
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

    use crate::tools::{bash::BashTool, edit::EditTool, read::ReadTool, write::WriteTool};

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

    /// Create an agent pre-loaded with the reduced default tool set used by tests.
    fn create_agent_with_tools(provider: Arc<dyn Provider>, cwd: PathBuf) -> (Agent, AgentHandle) {
        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, cwd);
        agent.tools.register(Arc::new(WriteTool));
        agent.tools.register(Arc::new(ReadTool));
        agent.tools.register(Arc::new(EditTool));
        agent.tools.register(Arc::new(BashTool));
        (agent, handle)
    }

    /// Create an agent with reduced tools only (used for synthetic A/B tests).
    fn create_agent_with_reduced_tools(
        provider: Arc<dyn Provider>,
        cwd: PathBuf,
    ) -> (Agent, AgentHandle) {
        let model = test_model(provider);
        let (mut agent, handle) = Agent::new(model, cwd);
        agent.tools.register(Arc::new(WriteTool));
        agent.tools.register(Arc::new(ReadTool));
        agent.tools.register(Arc::new(EditTool));
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

    // ── Test 3: Bash search finds a pattern (synthetic A/B baseline) ──────

    #[tokio::test]
    async fn agent_bash_search_finds_pattern() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("search_me.txt"),
            "line one\nunique_pattern_xyz here\nline three\n",
        )
        .unwrap();
        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_bash",
                "bash",
                serde_json::json!({"command": "grep --no-color -rn 'unique_pattern_xyz' ."}),
                100,
                20,
            ),
            text_response("Found it!", 100, 20),
        ]));

        let (mut agent, handle) =
            create_agent_with_reduced_tools(provider, tmp.path().to_path_buf());
        drop(handle);

        agent.run("Search for a pattern".to_string()).await.unwrap();

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
            !bash_text.trim().is_empty(),
            "bash grep output should not be empty"
        );
    }

    // ── Test 3b: repeated identical tool calls warn and then block ────────

    #[tokio::test]
    async fn agent_repeated_tool_calls_warn_then_block() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("repeat.txt"), "same content\n").unwrap();

        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(
                "call_1",
                "read",
                serde_json::json!({"path": "repeat.txt"}),
                100,
                20,
            ),
            tool_call_response(
                "call_2",
                "read",
                serde_json::json!({"path": "repeat.txt"}),
                100,
                20,
            ),
            tool_call_response(
                "call_3",
                "read",
                serde_json::json!({"path": "repeat.txt"}),
                100,
                20,
            ),
            tool_call_response(
                "call_4",
                "read",
                serde_json::json!({"path": "repeat.txt"}),
                100,
                20,
            ),
            text_response("Done", 100, 20),
        ]));

        let (mut agent, handle) =
            create_agent_with_reduced_tools(provider, tmp.path().to_path_buf());
        drop(handle);

        agent
            .run("Read the same file repeatedly".to_string())
            .await
            .unwrap();

        let third = agent
            .messages
            .iter()
            .find_map(|m| match m {
                Message::ToolResult(r) if r.tool_call_id == "call_3" => Some(r),
                _ => None,
            })
            .expect("third tool result");
        let fourth = agent
            .messages
            .iter()
            .find_map(|m| match m {
                Message::ToolResult(r) if r.tool_call_id == "call_4" => Some(r),
                _ => None,
            })
            .expect("fourth tool result");

        let third_text = third
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        let fourth_text = fourth
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(third_text.contains("Warning: identical tool call repeated 3 times"));
        assert!(fourth.is_error);
        assert!(fourth_text.contains("Blocked: identical tool call repeated 4 times"));
        assert_eq!(
            agent
                .messages
                .iter()
                .filter(|message| matches!(message, Message::User(_)))
                .count(),
            1,
            "agent should stop after repeated-action block rather than enqueueing more follow-ups"
        );
    }

    #[test]
    fn tool_results_indicate_repeated_action_detects_blocked_repeat_message() {
        let result = imp_llm::ToolResultMessage {
            tool_call_id: "call_repeat".to_string(),
            tool_name: "read".to_string(),
            content: vec![ContentBlock::Text {
                text: "Blocked: identical tool call repeated 4 times in a row for 'read'."
                    .to_string(),
            }],
            is_error: true,
            details: serde_json::Value::Null,
            timestamp: 0,
        };

        assert!(tool_results_indicate_repeated_action(&[result]));
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
        // Full mode keeps custom tools available
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
            project_memory_status: None,
            personality: None,
            soul: None,
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
            project_memory_status: None,
            personality: None,
            soul: None,
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
            !full_result.text.contains("You are a worker agent."),
            "Full mode should not include worker mode instructions"
        );

        // Orchestrator mode — should include mode instructions
        let orch_result = assemble(&AssembleParams {
            tools: &registry,
            agents_md: &[],
            skills: &[],
            facts: &[],
            project_memory_status: None,
            personality: None,
            soul: None,
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
            project_memory_status: None,
            personality: None,
            soul: None,
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
            project_memory_status: None,
            personality: None,
            soul: None,
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
