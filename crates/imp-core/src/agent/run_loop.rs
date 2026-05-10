use std::sync::Arc;
use std::time::Instant;

use futures::StreamExt;
use imp_llm::{
    AssistantMessage, ContentBlock, Context, Message, RequestOptions, StopReason, StreamEvent,
    Usage,
};

use crate::agent::{
    Agent, AgentCommand, AgentEvent, LoopDecision, RecoveryCheckpointKind, RunFinalStatus,
    StopReason as AgentStopReason, TimingStage, TurnPhase, TurnState,
};
use crate::error::Result;
use crate::hooks::HookEvent;
use crate::ui::NotifyLevel;

use super::{
    build_assistant_message, clone_model, mana_skill_follow_up_hint, push_stream_text_block,
    push_stream_thinking_block, record_mana_mutation_results,
};

impl Agent {
    pub(super) async fn reconcile_recovery_before_turn(
        &self,
        turn: u32,
    ) -> Option<super::RecoveryReconciliation> {
        let reconciliation = self
            .recovery_ledger
            .lock()
            .ok()
            .and_then(|ledger| ledger.reconcile_latest_finished_turn())?;

        // Only a previous turn can block the next turn. The current turn has no
        // side effects yet, and same-turn reconciliation happens after tool
        // execution checkpoints are recorded.
        if reconciliation.turn >= turn {
            return None;
        }

        if !reconciliation.is_safe_to_continue() {
            self.emit(AgentEvent::Error {
                error: format!(
                    "Recovery blocked before turn {turn}: {} incomplete non-retryable tool side effect(s)",
                    reconciliation.unsafe_incomplete_tools.len()
                ),
            })
            .await;
        }

        Some(reconciliation)
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
        let mut final_status: Option<RunFinalStatus> = None;
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
            let mut turn_state = TurnState::new(turn);
            turn_state.enter(TurnPhase::ReceiveCommands);

            if let Some(reconciliation) = self.reconcile_recovery_before_turn(turn).await {
                if !reconciliation.is_safe_to_continue() {
                    let unsafe_count = reconciliation.unsafe_incomplete_tools.len();
                    final_status = Some(RunFinalStatus::Blocked {
                        reason: AgentStopReason::ExecutionBlocked,
                        message: format!(
                            "recovery requires user review: {unsafe_count} incomplete non-retryable tool side effect(s)"
                        ),
                    });
                    break;
                }
            }

            if turn > 0 {
                if let Some(follow_up) = queued_pre_turn_follow_ups.pop_front() {
                    turn_state.record_continue(super::ContinueReason::QueuedUserFollowUp);
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

            turn_state.enter(TurnPhase::PreTurn);
            self.emit(AgentEvent::TurnStart { index: turn }).await;
            if let Ok(mut review) = self.turn_mana_review.lock() {
                review.begin_turn(turn);
            }
            let turn_started_at = Instant::now();
            turn_state.enter(TurnPhase::BuildContext);
            self.emit_timing(
                turn,
                TimingStage::ContextAssemblyStart,
                turn_started_at,
                None,
            )
            .await;
            let context_assembly_started_at = Instant::now();

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
            self.emit_timing_with_details(
                turn,
                TimingStage::ContextAssemblyEnd,
                turn_started_at,
                None,
                Some(context_assembly_started_at.elapsed().as_millis() as u64),
                None,
                Some(true),
            )
            .await;

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
            turn_state.enter(TurnPhase::SampleModel);
            let llm_request_started_at = Instant::now();
            self.emit_recovery_checkpoint(Self::recovery_checkpoint(
                turn,
                RecoveryCheckpointKind::ProviderRequestStart,
                None,
                None,
                None,
                None,
                None,
            ))
            .await;
            self.emit_timing(
                turn,
                TimingStage::LlmRequestStart,
                turn_started_at,
                Some(llm_request_started_at),
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
                                Some(llm_request_started_at),
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
                                        Some(llm_request_started_at),
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
                                        Some(llm_request_started_at),
                                    )
                                    .await;
                                }
                                let args_hash = Self::tool_args_hash(&arguments);
                                self.emit_recovery_checkpoint(Self::recovery_checkpoint(
                                    turn,
                                    RecoveryCheckpointKind::AssistantToolCallObserved,
                                    Some(id.clone()),
                                    Some(name.clone()),
                                    Some(args_hash),
                                    None,
                                    None,
                                ))
                                .await;
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
                                    Some(llm_request_started_at),
                                )
                                .await;
                                self.emit_recovery_checkpoint(Self::recovery_checkpoint(
                                    turn,
                                    RecoveryCheckpointKind::ProviderRequestCompleted,
                                    None,
                                    None,
                                    None,
                                    Some(true),
                                    None,
                                ))
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
                                turn_state.enter(TurnPhase::RecordObservations);
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
                                    status: RunFinalStatus::Failed {
                                        message: "stream error".to_string(),
                                    },
                                })
                                .await;
                                return Err(crate::error::Error::Llm(imp_llm::Error::Provider(
                                    "Stream error".to_string(),
                                )));
                            }
                        }
                    }
                    Err(e) => {
                        let had_partial_output =
                            !ordered_content.is_empty() || !tool_calls.is_empty();
                        let error = match &e {
                            imp_llm::Error::Stream(message) => {
                                if had_partial_output {
                                    format!(
                                        "Provider stream failed after partial output: {message}"
                                    )
                                } else {
                                    format!("Provider stream failed before output: {message}")
                                }
                            }
                            imp_llm::Error::Http(http_error) if http_error.is_decode() => {
                                if had_partial_output {
                                    format!(
                                        "Provider response body decode failed after partial output; not retrying to avoid duplicated tool output: {http_error}"
                                    )
                                } else {
                                    format!(
                                        "Provider response body decode failed before output after retry attempts were exhausted: {http_error}"
                                    )
                                }
                            }
                            _ => {
                                if had_partial_output {
                                    format!("Provider stream failed after partial output: {e}")
                                } else {
                                    e.to_string()
                                }
                            }
                        };
                        self.emit(AgentEvent::Error {
                            error: error.clone(),
                        })
                        .await;
                        let cost = total_usage.cost(&self.model.meta.pricing);
                        self.emit(AgentEvent::AgentEnd {
                            usage: total_usage,
                            cost,
                            status: RunFinalStatus::Failed {
                                message: error.clone(),
                            },
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
                        status: RunFinalStatus::Failed {
                            message: error.clone(),
                        },
                    })
                    .await;
                    return Err(crate::error::Error::Llm(imp_llm::Error::Stream(error)));
                }
                None => build_assistant_message(&ordered_content, &tool_calls, None),
            };

            turn_state.enter(TurnPhase::FinalizeAssistantMessage);
            self.emit_recovery_checkpoint(Self::recovery_checkpoint(
                turn,
                RecoveryCheckpointKind::AssistantMessageFinalized,
                None,
                None,
                None,
                Some(true),
                None,
            ))
            .await;
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

                self.emit_timing(
                    turn,
                    TimingStage::PostTurnAssessmentStart,
                    turn_started_at,
                    None,
                )
                .await;
                turn_state.enter(TurnPhase::AssessTurn);
                let assessment_started_at = Instant::now();
                let assessment = self.assess_post_turn(&msg, &[], false, &mana_review);
                self.emit_timing_with_details(
                    turn,
                    TimingStage::PostTurnAssessmentEnd,
                    turn_started_at,
                    None,
                    Some(assessment_started_at.elapsed().as_millis() as u64),
                    None,
                    Some(true),
                )
                .await;
                self.emit(AgentEvent::TurnAssessment {
                    index: turn,
                    assessment: assessment.debug_view(),
                })
                .await;
                turn_state.enter(TurnPhase::DecideNext);
                let decision = self.loop_decision_after_turn(&assessment);
                match decision {
                    LoopDecision::Continue { prompt, reason } => {
                        self.mark_continue_reason(reason);
                        turn_state.record_continue(reason);
                        queued_follow_ups.push_back(prompt);
                    }
                    LoopDecision::Finish { status } => {
                        final_status = Some(status);
                    }
                }

                if let Some(follow_up) = queued_follow_ups.pop_front() {
                    turn_state.record_continue(super::ContinueReason::QueuedUserFollowUp);
                    self.messages.push(Message::user(&follow_up));
                    turn += 1;
                    continue;
                }
                break;
            }

            // Execute tool calls
            turn_state.enter(TurnPhase::PlanTools);
            let tool_plan = self.plan_tools(tool_calls);
            turn_state.record_tool_plan(tool_plan.len());
            for call in &tool_plan.calls {
                self.emit_recovery_checkpoint(Self::recovery_checkpoint(
                    turn,
                    RecoveryCheckpointKind::ToolPlanCreated,
                    Some(call.id.clone()),
                    Some(call.name.clone()),
                    Some(Self::tool_args_hash(&call.args)),
                    Some(call.retry_safe),
                    None,
                ))
                .await;
            }
            turn_state.enter(TurnPhase::ExecuteTools);
            let results = self
                .execute_tool_plan(turn, turn_started_at, tool_plan, cancel_token)
                .await;
            turn_state.record_tool_results(results.len());
            turn_state.enter(TurnPhase::RecordObservations);

            for result in &results {
                self.emit_recovery_checkpoint(Self::recovery_checkpoint(
                    turn,
                    RecoveryCheckpointKind::ToolResultAddedToContext,
                    Some(result.tool_call_id.clone()),
                    Some(result.tool_name.clone()),
                    None,
                    Some(!result.is_error),
                    None,
                ))
                .await;
                self.messages.push(Message::ToolResult(result.clone()));
            }

            record_mana_mutation_results(&self.turn_mana_review, &results);
            let mana_review = self.finish_turn_mana_review(turn);
            self.emit(AgentEvent::TurnEnd {
                index: turn,
                message: msg.clone(),
                mana_review: mana_review.clone(),
            })
            .await;

            self.emit_timing(
                turn,
                TimingStage::PostTurnAssessmentStart,
                turn_started_at,
                None,
            )
            .await;
            turn_state.enter(TurnPhase::AssessTurn);
            let assessment_started_at = Instant::now();
            let assessment = self.assess_post_turn(&msg, &results, true, &mana_review);
            self.emit_timing_with_details(
                turn,
                TimingStage::PostTurnAssessmentEnd,
                turn_started_at,
                None,
                Some(assessment_started_at.elapsed().as_millis() as u64),
                None,
                Some(true),
            )
            .await;
            self.emit(AgentEvent::TurnAssessment {
                index: turn,
                assessment: assessment.debug_view(),
            })
            .await;
            turn_state.enter(TurnPhase::DecideNext);
            let decision = self.loop_decision_after_turn(&assessment);
            let should_stop_after_tool_turn = matches!(
                decision,
                LoopDecision::Finish {
                    status: RunFinalStatus::Blocked {
                        reason: AgentStopReason::RepeatedAction,
                        ..
                    }
                }
            );
            match decision {
                LoopDecision::Continue { prompt, reason } => {
                    self.mark_continue_reason(reason);
                    turn_state.record_continue(reason);
                    queued_follow_ups.push_back(prompt);
                }
                LoopDecision::Finish { status } => {
                    final_status = Some(status);
                }
            }

            if let Some(follow_up) = queued_follow_ups.pop_front() {
                self.messages.push(Message::user(&follow_up));
            }

            if should_stop_after_tool_turn {
                break;
            }

            turn_state.record_continue(super::ContinueReason::ToolResultsNeedInterpretation);
            turn += 1;
        }

        let cost = total_usage.cost(&self.model.meta.pricing);
        self.emit(AgentEvent::AgentEnd {
            usage: total_usage,
            cost,
            status: if cancelled {
                RunFinalStatus::Cancelled
            } else {
                final_status.unwrap_or_else(|| {
                    RunFinalStatus::from_stop_reason(AgentStopReason::NoAutomaticFollowUp)
                })
            },
        })
        .await;

        if cancelled {
            return Err(crate::error::Error::Cancelled);
        }

        Ok(())
    }
}
