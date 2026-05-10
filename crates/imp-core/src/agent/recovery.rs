use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{RecoveryCheckpoint, RecoveryCheckpointKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryReconciliation {
    pub turn: u32,
    pub unsafe_incomplete_tools: Vec<IncompleteToolRecovery>,
    pub retryable_incomplete_tools: Vec<IncompleteToolRecovery>,
}

impl RecoveryReconciliation {
    pub fn is_safe_to_continue(&self) -> bool {
        self.unsafe_incomplete_tools.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncompleteToolRecovery {
    pub tool_call_id: String,
    pub tool_name: Option<String>,
    pub args_hash: Option<String>,
    pub retry_safe: bool,
    pub state: IncompleteToolState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncompleteToolState {
    PlannedNotStarted,
    StartedNotCompleted,
    CompletedNotAppended,
}

#[derive(Debug, Clone, Default)]
pub struct RecoveryLedger {
    checkpoints: Vec<RecoveryCheckpoint>,
}

impl RecoveryLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_checkpoints(checkpoints: Vec<RecoveryCheckpoint>) -> Self {
        Self { checkpoints }
    }

    pub fn record(&mut self, checkpoint: RecoveryCheckpoint) {
        self.checkpoints.push(checkpoint);
    }

    pub fn checkpoints(&self) -> &[RecoveryCheckpoint] {
        &self.checkpoints
    }

    pub fn reconcile_latest_finished_turn(&self) -> Option<RecoveryReconciliation> {
        let latest_finished_turn = self
            .checkpoints
            .iter()
            .filter_map(|checkpoint| {
                let turn = checkpoint.turn;
                let turn_has_tool_checkpoint = self.checkpoints.iter().any(|candidate| {
                    candidate.turn == turn
                        && matches!(
                            candidate.kind,
                            RecoveryCheckpointKind::AssistantToolCallObserved
                                | RecoveryCheckpointKind::ToolPlanCreated
                                | RecoveryCheckpointKind::ToolExecutionStart
                                | RecoveryCheckpointKind::ToolExecutionEnd
                                | RecoveryCheckpointKind::ToolResultAddedToContext
                        )
                });

                match checkpoint.kind {
                    RecoveryCheckpointKind::ToolResultAddedToContext => Some(turn),
                    RecoveryCheckpointKind::AssistantMessageFinalized
                        if !turn_has_tool_checkpoint =>
                    {
                        Some(turn)
                    }
                    _ => None,
                }
            })
            .max()?;
        Some(self.reconcile_turn(latest_finished_turn))
    }

    pub fn reconcile_turn(&self, turn: u32) -> RecoveryReconciliation {
        let mut tools: HashMap<String, ToolRecoveryState> = HashMap::new();

        for checkpoint in self
            .checkpoints
            .iter()
            .filter(|checkpoint| checkpoint.turn == turn)
        {
            let Some(tool_call_id) = checkpoint.tool_call_id.as_ref() else {
                continue;
            };
            let state = tools.entry(tool_call_id.clone()).or_default();
            state.tool_name = checkpoint.tool_name.clone().or(state.tool_name.clone());
            state.args_hash = checkpoint.args_hash.clone().or(state.args_hash.clone());

            match checkpoint.kind {
                RecoveryCheckpointKind::ToolPlanCreated => {
                    state.planned = true;
                    state.retry_safe = checkpoint.success.unwrap_or(false);
                }
                RecoveryCheckpointKind::AssistantToolCallObserved => {
                    state.planned = true;
                }
                RecoveryCheckpointKind::ToolExecutionStart => {
                    state.started = true;
                }
                RecoveryCheckpointKind::ToolExecutionEnd => {
                    state.completed = checkpoint.success.unwrap_or(false);
                    if checkpoint.success == Some(false) {
                        state.retry_safe = false;
                    }
                }
                RecoveryCheckpointKind::ToolResultAddedToContext => {
                    state.appended = true;
                }
                RecoveryCheckpointKind::ProviderRequestStart
                | RecoveryCheckpointKind::AssistantMessageFinalized
                | RecoveryCheckpointKind::ProviderRequestCompleted => {}
            }
        }

        let mut retryable_incomplete_tools = Vec::new();
        let mut unsafe_incomplete_tools = Vec::new();

        for (tool_call_id, state) in tools {
            let incomplete_state = if state.appended {
                None
            } else if state.completed {
                Some(IncompleteToolState::CompletedNotAppended)
            } else if state.started {
                Some(IncompleteToolState::StartedNotCompleted)
            } else if state.planned {
                Some(IncompleteToolState::PlannedNotStarted)
            } else {
                None
            };

            if let Some(incomplete_state) = incomplete_state {
                let recovery = IncompleteToolRecovery {
                    tool_call_id,
                    tool_name: state.tool_name,
                    args_hash: state.args_hash,
                    retry_safe: state.retry_safe,
                    state: incomplete_state,
                };
                if recovery.retry_safe {
                    retryable_incomplete_tools.push(recovery);
                } else {
                    unsafe_incomplete_tools.push(recovery);
                }
            }
        }

        retryable_incomplete_tools
            .sort_by(|left, right| left.tool_call_id.cmp(&right.tool_call_id));
        unsafe_incomplete_tools.sort_by(|left, right| left.tool_call_id.cmp(&right.tool_call_id));

        RecoveryReconciliation {
            turn,
            unsafe_incomplete_tools,
            retryable_incomplete_tools,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct ToolRecoveryState {
    tool_name: Option<String>,
    args_hash: Option<String>,
    planned: bool,
    retry_safe: bool,
    started: bool,
    completed: bool,
    appended: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkpoint(
        kind: RecoveryCheckpointKind,
        tool_call_id: &str,
        success: Option<bool>,
    ) -> RecoveryCheckpoint {
        RecoveryCheckpoint {
            version: 1,
            turn: 3,
            kind,
            tool_call_id: Some(tool_call_id.into()),
            tool_name: Some("tool".into()),
            args_hash: Some("abc".into()),
            success,
            error_class: None,
            timestamp: 0,
        }
    }

    #[test]
    fn latest_finished_turn_ignores_in_progress_next_turn() {
        let mut checkpoints = vec![
            checkpoint(
                RecoveryCheckpointKind::ToolPlanCreated,
                "finished",
                Some(false),
            ),
            checkpoint(RecoveryCheckpointKind::ToolExecutionStart, "finished", None),
            checkpoint(
                RecoveryCheckpointKind::ToolExecutionEnd,
                "finished",
                Some(true),
            ),
            checkpoint(
                RecoveryCheckpointKind::ToolResultAddedToContext,
                "finished",
                Some(true),
            ),
        ];
        checkpoints.push(RecoveryCheckpoint {
            version: 1,
            turn: 3,
            kind: RecoveryCheckpointKind::AssistantMessageFinalized,
            tool_call_id: None,
            tool_name: None,
            args_hash: None,
            success: Some(true),
            error_class: None,
            timestamp: 0,
        });
        checkpoints.push(RecoveryCheckpoint {
            version: 1,
            turn: 4,
            kind: RecoveryCheckpointKind::ToolPlanCreated,
            tool_call_id: Some("in_progress".into()),
            tool_name: Some("edit".into()),
            args_hash: Some("def".into()),
            success: Some(false),
            error_class: None,
            timestamp: 0,
        });
        checkpoints.push(RecoveryCheckpoint {
            version: 1,
            turn: 4,
            kind: RecoveryCheckpointKind::ToolExecutionStart,
            tool_call_id: Some("in_progress".into()),
            tool_name: Some("edit".into()),
            args_hash: Some("def".into()),
            success: None,
            error_class: None,
            timestamp: 0,
        });
        let ledger = RecoveryLedger::from_checkpoints(checkpoints);

        let reconciliation = ledger.reconcile_latest_finished_turn().unwrap();
        assert_eq!(reconciliation.turn, 3);
        assert!(reconciliation.is_safe_to_continue());
    }

    #[test]
    fn later_tool_result_marks_tool_turn_finished() {
        let ledger = RecoveryLedger::from_checkpoints(vec![
            checkpoint(
                RecoveryCheckpointKind::AssistantMessageFinalized,
                "",
                Some(true),
            ),
            checkpoint(RecoveryCheckpointKind::ToolPlanCreated, "call", Some(false)),
            checkpoint(RecoveryCheckpointKind::ToolExecutionStart, "call", None),
            checkpoint(RecoveryCheckpointKind::ToolExecutionEnd, "call", Some(true)),
            checkpoint(
                RecoveryCheckpointKind::ToolResultAddedToContext,
                "call",
                Some(true),
            ),
        ]);

        let reconciliation = ledger.reconcile_latest_finished_turn().unwrap();
        assert_eq!(reconciliation.turn, 3);
        assert!(reconciliation.is_safe_to_continue());
        assert!(reconciliation.retryable_incomplete_tools.is_empty());
        assert!(reconciliation.unsafe_incomplete_tools.is_empty());
    }

    #[test]
    fn assistant_finalized_without_tool_result_does_not_mark_tool_turn_finished() {
        let mut checkpoints = vec![
            checkpoint(
                RecoveryCheckpointKind::AssistantMessageFinalized,
                "previous",
                Some(true),
            ),
            checkpoint(
                RecoveryCheckpointKind::ToolPlanCreated,
                "previous",
                Some(false),
            ),
            checkpoint(RecoveryCheckpointKind::ToolExecutionStart, "previous", None),
            checkpoint(
                RecoveryCheckpointKind::ToolExecutionEnd,
                "previous",
                Some(true),
            ),
            checkpoint(
                RecoveryCheckpointKind::ToolResultAddedToContext,
                "previous",
                Some(true),
            ),
        ];
        checkpoints.push(RecoveryCheckpoint {
            version: 1,
            turn: 4,
            kind: RecoveryCheckpointKind::AssistantMessageFinalized,
            tool_call_id: None,
            tool_name: None,
            args_hash: None,
            success: Some(true),
            error_class: None,
            timestamp: 0,
        });
        checkpoints.push(RecoveryCheckpoint {
            version: 1,
            turn: 4,
            kind: RecoveryCheckpointKind::ToolPlanCreated,
            tool_call_id: Some("interrupted".into()),
            tool_name: Some("edit".into()),
            args_hash: Some("def".into()),
            success: Some(false),
            error_class: None,
            timestamp: 0,
        });
        checkpoints.push(RecoveryCheckpoint {
            version: 1,
            turn: 4,
            kind: RecoveryCheckpointKind::ToolExecutionStart,
            tool_call_id: Some("interrupted".into()),
            tool_name: Some("edit".into()),
            args_hash: Some("def".into()),
            success: None,
            error_class: None,
            timestamp: 0,
        });

        let ledger = RecoveryLedger::from_checkpoints(checkpoints);
        let reconciliation = ledger.reconcile_latest_finished_turn().unwrap();
        assert_eq!(reconciliation.turn, 3);
        assert!(reconciliation.is_safe_to_continue());
    }

    #[test]
    fn appended_tool_is_not_incomplete() {
        let ledger = RecoveryLedger::from_checkpoints(vec![
            checkpoint(RecoveryCheckpointKind::ToolPlanCreated, "call", Some(true)),
            checkpoint(RecoveryCheckpointKind::ToolExecutionStart, "call", None),
            checkpoint(RecoveryCheckpointKind::ToolExecutionEnd, "call", Some(true)),
            checkpoint(
                RecoveryCheckpointKind::ToolResultAddedToContext,
                "call",
                Some(true),
            ),
        ]);

        let reconciliation = ledger.reconcile_turn(3);
        assert!(reconciliation.is_safe_to_continue());
        assert!(reconciliation.retryable_incomplete_tools.is_empty());
        assert!(reconciliation.unsafe_incomplete_tools.is_empty());
    }

    #[test]
    fn read_only_planned_not_started_is_retryable() {
        let ledger = RecoveryLedger::from_checkpoints(vec![checkpoint(
            RecoveryCheckpointKind::ToolPlanCreated,
            "call",
            Some(true),
        )]);

        let reconciliation = ledger.reconcile_turn(3);
        assert!(reconciliation.is_safe_to_continue());
        assert_eq!(reconciliation.retryable_incomplete_tools.len(), 1);
        assert_eq!(
            reconciliation.retryable_incomplete_tools[0].state,
            IncompleteToolState::PlannedNotStarted
        );
    }

    #[test]
    fn mutable_started_not_completed_is_unsafe() {
        let ledger = RecoveryLedger::from_checkpoints(vec![
            checkpoint(RecoveryCheckpointKind::ToolPlanCreated, "call", Some(false)),
            checkpoint(RecoveryCheckpointKind::ToolExecutionStart, "call", None),
        ]);

        let reconciliation = ledger.reconcile_turn(3);
        assert!(!reconciliation.is_safe_to_continue());
        assert_eq!(reconciliation.unsafe_incomplete_tools.len(), 1);
        assert_eq!(
            reconciliation.unsafe_incomplete_tools[0].state,
            IncompleteToolState::StartedNotCompleted
        );
    }

    #[test]
    fn completed_not_appended_is_incomplete() {
        let ledger = RecoveryLedger::from_checkpoints(vec![
            checkpoint(RecoveryCheckpointKind::ToolPlanCreated, "call", Some(false)),
            checkpoint(RecoveryCheckpointKind::ToolExecutionStart, "call", None),
            checkpoint(RecoveryCheckpointKind::ToolExecutionEnd, "call", Some(true)),
        ]);

        let reconciliation = ledger.reconcile_turn(3);
        assert_eq!(reconciliation.unsafe_incomplete_tools.len(), 1);
        assert_eq!(
            reconciliation.unsafe_incomplete_tools[0].state,
            IncompleteToolState::CompletedNotAppended
        );
    }
}
