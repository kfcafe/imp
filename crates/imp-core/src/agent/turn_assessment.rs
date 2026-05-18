use super::{ContinueReason, StopReason};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NextAction {
    Continue {
        prompt: String,
        reason: ContinueReason,
    },
    Stop {
        reason: NextActionStopReason,
    },
}

pub(super) type NextActionStopReason = StopReason;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeEvidence {
    pub(super) repeated_action: bool,
    pub(super) execution_stop_reason: Option<NextActionStopReason>,
    pub(super) work_completed: bool,
    pub(super) execution_debt: bool,
    pub(super) execution_evidence: bool,
    pub(super) planning_only_progress: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ManaEvidence {
    pub(super) stop_reason: Option<NextActionStopReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TextFallbackEvidence {
    pub(super) planner_stop_reason: Option<NextActionStopReason>,
    pub(super) execution_stop_reason: Option<NextActionStopReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ContinueRecommendation {
    pub(super) prompt: String,
    pub(super) reason: ContinueReason,
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
pub(super) struct PostTurnAssessment {
    pub(super) runtime: RuntimeEvidence,
    pub(super) mana: ManaEvidence,
    pub(super) text_fallback: TextFallbackEvidence,
    pub(super) continue_recommendation: Option<ContinueRecommendation>,
}

impl PostTurnAssessment {
    pub(super) fn into_next_action(self) -> NextAction {
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

    pub(super) fn debug_view(&self) -> NextActionAssessment {
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
