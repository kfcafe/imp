use crate::agent::{RunFinalStatus, StopReason};
use crate::eval_candidate::{
    EvalActualBehavior, EvalArtifactRef, EvalCandidate, EvalCandidateSource, EvalExpectedBehavior,
    EvalFailureMode, EvalPrivacy, EvalRedactionStatus, EvalVerifier,
};
use crate::workflow::{VerificationGate, VerificationGateRequirement, VerificationGateStatus};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EvalCandidateCloseoutContext {
    pub candidate_id: String,
    pub run_id: Option<String>,
    pub workflow_id: Option<String>,
    pub session_id: Option<String>,
    pub prompt: Option<String>,
    pub expected_summary: Option<String>,
    pub verifiers: Vec<EvalVerifier>,
    pub artifact_refs: Vec<EvalArtifactRef>,
}

impl EvalCandidateCloseoutContext {
    pub fn new(candidate_id: impl Into<String>) -> Self {
        Self {
            candidate_id: candidate_id.into(),
            ..Self::default()
        }
    }
}

pub fn eval_candidate_for_closeout(
    status: &RunFinalStatus,
    gates: &[VerificationGate],
    context: EvalCandidateCloseoutContext,
) -> Option<EvalCandidate> {
    let trigger = eval_failure_mode_for_closeout(status, gates)?;
    let actual_summary = actual_summary_for_closeout(status, gates);
    let mut labels = vec!["eval-candidate".to_string(), "closeout".to_string()];
    labels.push(label_for_failure_mode(&trigger).to_string());

    Some(EvalCandidate {
        id: context.candidate_id,
        source: EvalCandidateSource {
            run_id: context.run_id,
            workflow_id: context.workflow_id,
            session_id: context.session_id,
            parent_candidate_id: None,
        },
        trigger: trigger.clone(),
        failure_mode: trigger,
        labels,
        prompt: context.prompt,
        expected_behavior: EvalExpectedBehavior {
            summary: context
                .expected_summary
                .unwrap_or_else(|| "Run should complete according to its workflow contract".into()),
            assertions: verifier_assertions(gates),
        },
        actual_behavior: Some(EvalActualBehavior {
            summary: actual_summary,
            error_excerpt: closeout_error_excerpt(status, gates),
        }),
        verifiers: merge_context_verifiers(
            context.verifiers,
            gates.iter().map(eval_verifier_from_gate).collect(),
        ),
        artifact_refs: context.artifact_refs,
        privacy: EvalPrivacy {
            redaction_status: EvalRedactionStatus::Unreviewed,
            redaction_rules: Vec::new(),
            contains_sensitive_data: false,
        },
        trust: Default::default(),
        ..EvalCandidate::new("", EvalFailureMode::Unknown)
    })
}

pub fn manual_eval_candidate_with_verifier(
    mut candidate: EvalCandidate,
    command: impl Into<String>,
) -> EvalCandidate {
    let command = command.into();
    candidate
        .expected_behavior
        .assertions
        .push(format!("{command} passes"));
    candidate.verifiers.push(EvalVerifier {
        name: "manual verifier".into(),
        command: Some(command),
        required: true,
        last_status: None,
        exit_code: None,
        output_ref: None,
        failure_excerpt: None,
    });
    candidate
}

fn merge_context_verifiers(
    mut manual: Vec<EvalVerifier>,
    mut gate_verifiers: Vec<EvalVerifier>,
) -> Vec<EvalVerifier> {
    manual.append(&mut gate_verifiers);
    manual
}

fn eval_failure_mode_for_closeout(
    status: &RunFinalStatus,
    gates: &[VerificationGate],
) -> Option<EvalFailureMode> {
    if let Some(mode) = gates
        .iter()
        .filter(|gate| gate.is_required())
        .find_map(|gate| match gate.status {
            VerificationGateStatus::Failed => Some(EvalFailureMode::VerificationFailed),
            VerificationGateStatus::Blocked => Some(EvalFailureMode::VerificationBlocked),
            VerificationGateStatus::Skipped
            | VerificationGateStatus::Pending
            | VerificationGateStatus::Running => Some(EvalFailureMode::VerificationSkippedRequired),
            VerificationGateStatus::Passed => None,
        })
    {
        return Some(mode);
    }

    match status {
        RunFinalStatus::Done { .. } => None,
        RunFinalStatus::DoneWithConcerns { .. } => Some(EvalFailureMode::DoneWithConcerns),
        RunFinalStatus::Blocked { reason, .. } if *reason == StopReason::RepeatedAction => {
            Some(EvalFailureMode::ToolLoop)
        }
        RunFinalStatus::Blocked { .. } => Some(EvalFailureMode::Blocked),
        RunFinalStatus::Failed { .. } => Some(EvalFailureMode::Unknown),
        RunFinalStatus::NeedsUserInput { .. } | RunFinalStatus::Cancelled => None,
    }
}

fn eval_verifier_from_gate(gate: &VerificationGate) -> EvalVerifier {
    let result = gate.result.as_ref();
    EvalVerifier {
        name: if gate.name.is_empty() {
            gate.id.clone()
        } else {
            gate.name.clone()
        },
        command: gate.command.as_ref().map(|command| command.command.clone()),
        required: gate.requirement == VerificationGateRequirement::Required,
        last_status: Some(verification_status_label(gate.status).into()),
        exit_code: result.and_then(|result| result.exit_code),
        output_ref: gate.artifacts.first().map(|artifact| artifact.path.clone()),
        failure_excerpt: gate
            .reason
            .clone()
            .or_else(|| result.and_then(|result| result.stderr_summary.clone()))
            .or_else(|| result.and_then(|result| result.summary.clone())),
    }
}

fn verification_status_label(status: VerificationGateStatus) -> &'static str {
    match status {
        VerificationGateStatus::Pending => "pending",
        VerificationGateStatus::Running => "running",
        VerificationGateStatus::Passed => "passed",
        VerificationGateStatus::Failed => "failed",
        VerificationGateStatus::Skipped => "skipped",
        VerificationGateStatus::Blocked => "blocked",
    }
}

fn verifier_assertions(gates: &[VerificationGate]) -> Vec<String> {
    gates
        .iter()
        .filter_map(|gate| {
            gate.command
                .as_ref()
                .map(|command| format!("{} passes", command.command))
        })
        .collect()
}

fn actual_summary_for_closeout(status: &RunFinalStatus, gates: &[VerificationGate]) -> String {
    if let Some(gate) = gates
        .iter()
        .find(|gate| gate.is_required() && gate.status != VerificationGateStatus::Passed)
    {
        return format!(
            "Required verification gate '{}' ended with status {:?}",
            if gate.name.is_empty() {
                &gate.id
            } else {
                &gate.name
            },
            gate.status
        );
    }

    match status {
        RunFinalStatus::Done { .. } => "Run completed successfully".into(),
        RunFinalStatus::DoneWithConcerns { concerns, .. } => {
            format!("Run completed with concerns: {}", concerns.join("; "))
        }
        RunFinalStatus::Blocked { message, .. } => format!("Run blocked: {message}"),
        RunFinalStatus::NeedsUserInput { question } => format!("Run needs user input: {question}"),
        RunFinalStatus::Cancelled => "Run was cancelled".into(),
        RunFinalStatus::Failed { message } => format!("Run failed: {message}"),
    }
}

fn closeout_error_excerpt(status: &RunFinalStatus, gates: &[VerificationGate]) -> Option<String> {
    gates
        .iter()
        .filter(|gate| gate.is_required() && gate.status != VerificationGateStatus::Passed)
        .find_map(|gate| {
            gate.reason.clone().or_else(|| {
                gate.result.as_ref().and_then(|result| {
                    result
                        .stderr_summary
                        .clone()
                        .or_else(|| result.summary.clone())
                })
            })
        })
        .or_else(|| match status {
            RunFinalStatus::Blocked { message, .. } | RunFinalStatus::Failed { message } => {
                Some(message.clone())
            }
            RunFinalStatus::DoneWithConcerns { concerns, .. } => Some(concerns.join("; ")),
            _ => None,
        })
}

fn label_for_failure_mode(mode: &EvalFailureMode) -> &'static str {
    match mode {
        EvalFailureMode::Blocked => "blocked",
        EvalFailureMode::DoneWithConcerns => "done-with-concerns",
        EvalFailureMode::VerificationFailed => "verification-failed",
        EvalFailureMode::VerificationBlocked => "verification-blocked",
        EvalFailureMode::VerificationSkippedRequired => "verification-skipped-required",
        EvalFailureMode::PolicyDenied => "policy-denied",
        EvalFailureMode::ToolLoop => "tool-loop",
        EvalFailureMode::ToolError => "tool-error",
        EvalFailureMode::UserCorrection => "user-correction",
        EvalFailureMode::NegativeFeedback => "negative-feedback",
        EvalFailureMode::WorktreeApplyConflict => "worktree-apply-conflict",
        EvalFailureMode::Manual => "manual",
        EvalFailureMode::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{VerificationArtifactRef, VerificationGateResult};

    #[test]
    fn workflow_closeout_done_does_not_create_eval_candidate() {
        let candidate = eval_candidate_for_closeout(
            &RunFinalStatus::Done {
                reason: StopReason::WorkCompleted,
            },
            &[],
            EvalCandidateCloseoutContext::new("candidate-1"),
        );
        assert!(candidate.is_none());
    }

    #[test]
    fn workflow_closeout_blocked_creates_eval_candidate() {
        let candidate = eval_candidate_for_closeout(
            &RunFinalStatus::Blocked {
                reason: StopReason::ExecutionBlocked,
                message: "dependency unavailable".into(),
            },
            &[],
            EvalCandidateCloseoutContext {
                run_id: Some("run-1".into()),
                prompt: Some("finish the task".into()),
                ..EvalCandidateCloseoutContext::new("candidate-1")
            },
        )
        .unwrap();

        assert_eq!(candidate.failure_mode, EvalFailureMode::Blocked);
        assert_eq!(candidate.source.run_id.as_deref(), Some("run-1"));
        assert_eq!(candidate.prompt.as_deref(), Some("finish the task"));
        assert!(candidate
            .actual_behavior
            .unwrap()
            .summary
            .contains("blocked"));
    }

    #[test]
    fn workflow_closeout_concerned_creates_eval_candidate() {
        let candidate = eval_candidate_for_closeout(
            &RunFinalStatus::DoneWithConcerns {
                reason: StopReason::NoProgress,
                concerns: vec!["verification was not available".into()],
            },
            &[],
            EvalCandidateCloseoutContext::new("candidate-2"),
        )
        .unwrap();

        assert_eq!(candidate.failure_mode, EvalFailureMode::DoneWithConcerns);
        assert!(candidate.labels.contains(&"done-with-concerns".into()));
        assert!(candidate
            .actual_behavior
            .unwrap()
            .error_excerpt
            .unwrap()
            .contains("verification"));
    }

    #[test]
    fn verification_gate_failure_takes_precedence_over_done_status() {
        let mut gate = VerificationGate::command("unit-tests", "cargo test -p imp-core");
        gate.artifacts.push(VerificationArtifactRef::new(
            "verification-output",
            ".imp/runs/run-1/verification/unit-tests.txt",
        ));
        gate.mark_failed(VerificationGateResult {
            exit_code: Some(101),
            stderr_summary: Some("test failed".into()),
            summary: Some("unit test command failed".into()),
            ..VerificationGateResult::default()
        });

        let candidate = eval_candidate_for_closeout(
            &RunFinalStatus::Done {
                reason: StopReason::WorkCompleted,
            },
            &[gate],
            EvalCandidateCloseoutContext::new("candidate-3"),
        )
        .unwrap();

        assert_eq!(candidate.failure_mode, EvalFailureMode::VerificationFailed);
        assert_eq!(
            candidate.expected_behavior.assertions,
            vec!["cargo test -p imp-core passes"]
        );
        assert_eq!(candidate.verifiers[0].name, "unit-tests");
        assert_eq!(
            candidate.verifiers[0].command.as_deref(),
            Some("cargo test -p imp-core")
        );
        assert!(candidate.verifiers[0].required);
        assert_eq!(
            candidate.verifiers[0].last_status.as_deref(),
            Some("failed")
        );
        assert_eq!(candidate.verifiers[0].exit_code, Some(101));
        assert_eq!(
            candidate.verifiers[0].output_ref.as_deref(),
            Some(std::path::Path::new(
                ".imp/runs/run-1/verification/unit-tests.txt"
            ))
        );
        assert_eq!(
            candidate.verifiers[0].failure_excerpt.as_deref(),
            Some("test failed")
        );
    }

    #[test]
    fn manual_candidate_accepts_optional_verifier_command() {
        let candidate = manual_eval_candidate_with_verifier(
            EvalCandidate::new("manual-1", EvalFailureMode::Manual),
            "cargo test -p imp-core smoke",
        );

        assert_eq!(
            candidate.expected_behavior.assertions,
            vec!["cargo test -p imp-core smoke passes"]
        );
        assert_eq!(candidate.verifiers.len(), 1);
        assert_eq!(
            candidate.verifiers[0].command.as_deref(),
            Some("cargo test -p imp-core smoke")
        );
        assert_eq!(candidate.verifiers[0].last_status, None);
    }

    #[test]
    fn repeated_action_blocker_is_classified_as_tool_loop() {
        let candidate = eval_candidate_for_closeout(
            &RunFinalStatus::Blocked {
                reason: StopReason::RepeatedAction,
                message: "same tool call repeated without progress".into(),
            },
            &[],
            EvalCandidateCloseoutContext::new("candidate-4"),
        )
        .unwrap();

        assert_eq!(candidate.failure_mode, EvalFailureMode::ToolLoop);
    }
}
