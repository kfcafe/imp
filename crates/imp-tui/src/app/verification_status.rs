use imp_core::workflow::{VerificationCloseoutEffect, VerificationGate, VerificationGateStatus};

pub(super) fn verification_status_text(
    gate: &VerificationGate,
    status: Option<&str>,
    closeout_effect: Option<VerificationCloseoutEffect>,
) -> String {
    let label = verification_gate_label(gate);
    let status = status.unwrap_or(match gate.status {
        VerificationGateStatus::Pending => "pending",
        VerificationGateStatus::Running => "running",
        VerificationGateStatus::Passed => "passed",
        VerificationGateStatus::Failed => "failed",
        VerificationGateStatus::Skipped => "skipped",
        VerificationGateStatus::Blocked => "blocked",
    });
    let mut text = format!("{label} {status}");
    if gate.is_required() {
        text.push_str(" required");
    }
    if matches!(
        closeout_effect,
        Some(VerificationCloseoutEffect::BlocksDone)
            | Some(VerificationCloseoutEffect::BlocksDoneWithConcerns)
    ) {
        text.push_str(" blocks closeout");
    }
    text
}

pub(super) fn verification_gate_label(gate: &VerificationGate) -> String {
    if !gate.name.is_empty() {
        gate.name.clone()
    } else if !gate.id.is_empty() {
        gate.id.clone()
    } else if let Some(command) = &gate.command {
        command.command.clone()
    } else {
        "verification".into()
    }
}
