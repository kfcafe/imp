use std::sync::Arc;
use std::time::Instant;

use futures::future::join_all;
use tokio::sync::mpsc;

use crate::agent::{Agent, AgentEvent, RecoveryCheckpointKind, TimingStage};
use crate::guardrails::{self, GuardrailLevel};
use crate::hooks::HookEvent;

use super::{
    extract_file_path, mana_bash_equivalent_hint,
    mana_loop::{enrich_mana_result_details, evaluate_mana_policy},
    RepeatedToolCallCheck, RepeatedToolCallState,
};

impl Agent {
    /// Execute tool calls from a single assistant message.
    pub(super) async fn execute_tools(
        &self,
        turn: u32,
        turn_started_at: Instant,
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
                let result = self
                    .execute_one_tool(&id, &name, args, cancel_token, turn, turn_started_at)
                    .await;
                (index, result)
            }
        }))
        .await;

        for (index, id, name, args) in mutable {
            let result = self
                .execute_one_tool(
                    &id,
                    &name,
                    args,
                    Arc::clone(&cancel_token),
                    turn,
                    turn_started_at,
                )
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
        turn: u32,
        turn_started_at: Instant,
    ) -> imp_llm::ToolResultMessage {
        let args_hash = Self::tool_args_hash(&args);
        let repeat_check = self.repeated_tool_call_check(call_id, tool_name, &args);
        let tool_started_at = Instant::now();
        self.emit_timing_with_details(
            turn,
            TimingStage::ToolExecutionStart,
            turn_started_at,
            None,
            None,
            Some(tool_name.to_string()),
            None,
        )
        .await;

        self.emit_recovery_checkpoint(Self::recovery_checkpoint(
            turn,
            RecoveryCheckpointKind::ToolExecutionStart,
            Some(call_id.to_string()),
            Some(tool_name.to_string()),
            Some(args_hash.clone()),
            None,
            None,
        ))
        .await;

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
            self.emit_timing_with_details(
                turn,
                TimingStage::ToolExecutionEnd,
                turn_started_at,
                None,
                Some(tool_started_at.elapsed().as_millis() as u64),
                Some(tool_name.to_string()),
                Some(false),
            )
            .await;
            self.emit_recovery_checkpoint(Self::recovery_checkpoint(
                turn,
                RecoveryCheckpointKind::ToolExecutionEnd,
                Some(call_id.to_string()),
                Some(tool_name.to_string()),
                Some(args_hash.clone()),
                Some(false),
                Some("repeated_tool_call_blocked".to_string()),
            ))
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
            self.emit_recovery_checkpoint(Self::recovery_checkpoint(
                turn,
                RecoveryCheckpointKind::ToolExecutionEnd,
                Some(call_id.to_string()),
                Some(tool_name.to_string()),
                Some(args_hash.clone()),
                Some(false),
                Some("mode_blocked".to_string()),
            ))
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
            self.emit_recovery_checkpoint(Self::recovery_checkpoint(
                turn,
                RecoveryCheckpointKind::ToolExecutionEnd,
                Some(call_id.to_string()),
                Some(tool_name.to_string()),
                Some(args_hash.clone()),
                Some(false),
                Some("hook_blocked".to_string()),
            ))
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
                    self.emit_recovery_checkpoint(Self::recovery_checkpoint(
                        turn,
                        RecoveryCheckpointKind::ToolExecutionEnd,
                        Some(call_id.to_string()),
                        Some(tool_name.to_string()),
                        Some(args_hash.clone()),
                        Some(false),
                        Some("policy_blocked".to_string()),
                    ))
                    .await;
                    return result;
                }
            }
        }

        if tool_name == "mana" {
            let policy = evaluate_mana_policy(self.mode, &args);
            if !policy.allowed {
                let reason = policy
                    .reason
                    .clone()
                    .unwrap_or_else(|| "Mana action blocked by loop policy".to_string());
                let mut result =
                    crate::tools::ToolOutput::error(reason).into_tool_result(call_id, tool_name);
                result.details = policy.details();
                self.emit(AgentEvent::ToolExecutionEnd {
                    tool_call_id: call_id.to_string(),
                    result: result.clone(),
                })
                .await;
                self.emit_recovery_checkpoint(Self::recovery_checkpoint(
                    turn,
                    RecoveryCheckpointKind::ToolExecutionEnd,
                    Some(call_id.to_string()),
                    Some(tool_name.to_string()),
                    Some(args_hash.clone()),
                    Some(false),
                    Some("mana_policy_blocked".to_string()),
                ))
                .await;
                return result;
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
                self.emit_recovery_checkpoint(Self::recovery_checkpoint(
                    turn,
                    RecoveryCheckpointKind::ToolExecutionEnd,
                    Some(call_id.to_string()),
                    Some(tool_name.to_string()),
                    Some(args_hash.clone()),
                    Some(false),
                    Some("validation_error".to_string()),
                ))
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
                    Ok(output) => {
                        let mut result = output.into_tool_result(call_id, tool_name);
                        if tool_name == "mana" {
                            let policy = evaluate_mana_policy(self.mode, &args);
                            result.details = enrich_mana_result_details(result.details, &policy);
                        }
                        result
                    }
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
        self.emit_timing_with_details(
            turn,
            TimingStage::ToolExecutionEnd,
            turn_started_at,
            None,
            Some(tool_started_at.elapsed().as_millis() as u64),
            Some(tool_name.to_string()),
            Some(!result.is_error),
        )
        .await;

        self.emit_recovery_checkpoint(Self::recovery_checkpoint(
            turn,
            RecoveryCheckpointKind::ToolExecutionEnd,
            Some(call_id.to_string()),
            Some(tool_name.to_string()),
            Some(args_hash),
            Some(!result.is_error),
            result.is_error.then(|| "tool_error".to_string()),
        ))
        .await;

        if let RepeatedToolCallCheck::Warn(warning) = repeat_check {
            result.content.push(imp_llm::ContentBlock::Text {
                text: format!("\n\n{warning}"),
            });
        }

        result
    }
}
