use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const EVAL_CANDIDATE_SCHEMA_VERSION: u32 = 1;
pub const EVAL_CANDIDATES_DIR: &str = ".imp/eval-candidates";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum EvalFailureMode {
    Blocked,
    DoneWithConcerns,
    VerificationFailed,
    VerificationBlocked,
    VerificationSkippedRequired,
    PolicyDenied,
    ToolLoop,
    ToolError,
    UserCorrection,
    NegativeFeedback,
    WorktreeApplyConflict,
    Manual,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EvalCandidate {
    pub schema_version: u32,
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub source: EvalCandidateSource,
    pub trigger: EvalFailureMode,
    pub failure_mode: EvalFailureMode,
    pub labels: Vec<String>,
    pub prompt: Option<String>,
    pub task: Option<EvalTask>,
    pub workflow_contract_ref: Option<EvalArtifactRef>,
    pub expected_behavior: EvalExpectedBehavior,
    pub actual_behavior: Option<EvalActualBehavior>,
    pub verifiers: Vec<EvalVerifier>,
    pub artifact_refs: Vec<EvalArtifactRef>,
    pub policy_refs: Vec<EvalPolicyRef>,
    pub privacy: EvalPrivacy,
    pub trust: EvalTrustSummary,
    pub human_notes_ref: Option<PathBuf>,
    pub correction: Option<EvalCorrection>,
}

impl EvalCandidate {
    pub fn new(id: impl Into<String>, failure_mode: EvalFailureMode) -> Self {
        Self {
            id: id.into(),
            trigger: failure_mode.clone(),
            failure_mode,
            ..Self::default()
        }
    }

    pub fn candidate_dir(root: impl AsRef<Path>, id: &str) -> PathBuf {
        root.as_ref().join(EVAL_CANDIDATES_DIR).join(id)
    }

    pub fn candidate_path(root: impl AsRef<Path>, id: &str) -> PathBuf {
        Self::candidate_dir(root, id).join("candidate.json")
    }

    pub fn write_to_dir(&self, root: impl AsRef<Path>) -> io::Result<PathBuf> {
        let path = Self::candidate_path(root, &self.id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(&path, json)?;
        Ok(path)
    }

    pub fn read_from_dir(root: impl AsRef<Path>, id: &str) -> io::Result<Self> {
        let path = Self::candidate_path(root, id);
        let json = fs::read_to_string(path)?;
        serde_json::from_str(&json).map_err(io::Error::other)
    }
}

impl Default for EvalCandidate {
    fn default() -> Self {
        Self {
            schema_version: EVAL_CANDIDATE_SCHEMA_VERSION,
            id: String::new(),
            created_at: Utc::now(),
            source: EvalCandidateSource::default(),
            trigger: EvalFailureMode::Unknown,
            failure_mode: EvalFailureMode::Unknown,
            labels: Vec::new(),
            prompt: None,
            task: None,
            workflow_contract_ref: None,
            expected_behavior: EvalExpectedBehavior::default(),
            actual_behavior: None,
            verifiers: Vec::new(),
            artifact_refs: Vec::new(),
            policy_refs: Vec::new(),
            privacy: EvalPrivacy::default(),
            trust: EvalTrustSummary::default(),
            human_notes_ref: None,
            correction: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EvalCandidateSource {
    pub run_id: Option<String>,
    pub workflow_id: Option<String>,
    pub session_id: Option<String>,
    pub parent_candidate_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EvalTask {
    pub title: Option<String>,
    pub description: Option<String>,
    pub acceptance: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EvalExpectedBehavior {
    pub summary: String,
    pub assertions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EvalActualBehavior {
    pub summary: String,
    pub error_excerpt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EvalVerifier {
    pub name: String,
    pub command: Option<String>,
    pub required: bool,
    pub last_status: Option<String>,
    pub exit_code: Option<i32>,
    pub output_ref: Option<PathBuf>,
    pub failure_excerpt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EvalArtifactRef {
    pub kind: String,
    pub path: PathBuf,
    pub summary: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EvalPolicyRef {
    pub tool_name: Option<String>,
    pub action_kind: Option<String>,
    pub decision: String,
    pub reason_code: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum EvalRedactionStatus {
    #[default]
    Unreviewed,
    Redacted,
    ContainsSensitiveData,
    SafeToExport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EvalPrivacy {
    pub redaction_status: EvalRedactionStatus,
    pub redaction_rules: Vec<String>,
    pub contains_sensitive_data: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EvalTrustSummary {
    pub sources: Vec<String>,
    pub low_trust_influences: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn redact_eval_candidate(mut candidate: EvalCandidate) -> EvalCandidate {
    let mut rules = candidate.privacy.redaction_rules.clone();
    redact_string_option(&mut candidate.prompt, &mut rules);
    redact_string(&mut candidate.expected_behavior.summary, &mut rules);
    redact_strings(&mut candidate.expected_behavior.assertions, &mut rules);
    if let Some(task) = &mut candidate.task {
        redact_string_option(&mut task.title, &mut rules);
        redact_string_option(&mut task.description, &mut rules);
        redact_string_option(&mut task.acceptance, &mut rules);
    }
    if let Some(actual) = &mut candidate.actual_behavior {
        redact_string(&mut actual.summary, &mut rules);
        redact_string_option(&mut actual.error_excerpt, &mut rules);
    }
    for verifier in &mut candidate.verifiers {
        redact_string(&mut verifier.name, &mut rules);
        redact_string_option(&mut verifier.command, &mut rules);
        redact_string_option(&mut verifier.last_status, &mut rules);
        redact_string_option(&mut verifier.failure_excerpt, &mut rules);
    }
    for artifact in &mut candidate.artifact_refs {
        redact_string(&mut artifact.kind, &mut rules);
        redact_string_option(&mut artifact.summary, &mut rules);
    }
    for policy in &mut candidate.policy_refs {
        redact_string_option(&mut policy.tool_name, &mut rules);
        redact_string_option(&mut policy.action_kind, &mut rules);
        redact_string(&mut policy.decision, &mut rules);
        redact_string_option(&mut policy.reason_code, &mut rules);
        redact_string_option(&mut policy.reason, &mut rules);
    }
    if let Some(correction) = &mut candidate.correction {
        redact_string(&mut correction.kind, &mut rules);
        redact_string(&mut correction.summary, &mut rules);
        if let Some(artifact) = &mut correction.artifact_ref {
            redact_string(&mut artifact.kind, &mut rules);
            redact_string_option(&mut artifact.summary, &mut rules);
        }
    }
    redact_strings(&mut candidate.trust.sources, &mut rules);
    redact_strings(&mut candidate.trust.low_trust_influences, &mut rules);
    redact_strings(&mut candidate.trust.warnings, &mut rules);

    if rules.iter().any(|rule| rule == "secret-like-patterns") {
        candidate.privacy.redaction_status = EvalRedactionStatus::Redacted;
        candidate.privacy.contains_sensitive_data = false;
    }
    candidate.privacy.redaction_rules = rules;
    candidate
}

fn redact_string_option(value: &mut Option<String>, rules: &mut Vec<String>) {
    if let Some(value) = value {
        redact_string(value, rules);
    }
}

fn redact_strings(values: &mut [String], rules: &mut Vec<String>) {
    for value in values {
        redact_string(value, rules);
    }
}

fn redact_string(value: &mut String, rules: &mut Vec<String>) {
    let redacted = redact_secret_like_patterns(value);
    if redacted != *value {
        *value = redacted;
        if !rules.iter().any(|rule| rule == "secret-like-patterns") {
            rules.push("secret-like-patterns".into());
        }
    }
}

fn redact_secret_like_patterns(value: &str) -> String {
    value
        .split_whitespace()
        .map(|token| {
            if is_secret_like_token(token) {
                redact_token_preserving_edges(token)
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_secret_like_token(token: &str) -> bool {
    let trimmed =
        token.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-');
    trimmed.len() >= 20
        && trimmed.chars().any(|ch| ch.is_ascii_lowercase())
        && trimmed.chars().any(|ch| ch.is_ascii_uppercase())
        && trimmed.chars().any(|ch| ch.is_ascii_digit())
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn redact_token_preserving_edges(token: &str) -> String {
    let start = token
        .find(|ch: char| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        .unwrap_or(0);
    let end = token
        .rfind(|ch: char| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        .map(|index| index + 1)
        .unwrap_or(token.len());
    format!("{}[REDACTED_SECRET]{}", &token[..start], &token[end..])
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EvalCorrection {
    pub kind: String,
    pub summary: String,
    pub artifact_ref: Option<EvalArtifactRef>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_candidate() -> EvalCandidate {
        EvalCandidate {
            id: "2026-05-14-run-abc-verification-failed".into(),
            source: EvalCandidateSource {
                run_id: Some("run-abc".into()),
                workflow_id: Some("394.12.2".into()),
                ..EvalCandidateSource::default()
            },
            trigger: EvalFailureMode::VerificationFailed,
            failure_mode: EvalFailureMode::VerificationFailed,
            labels: vec!["eval-candidate".into(), "verification".into()],
            prompt: Some("Fix the failing parser test".into()),
            expected_behavior: EvalExpectedBehavior {
                summary: "Parser handles empty input without panic".into(),
                assertions: vec!["cargo test -p imp-core parser_empty_input passes".into()],
            },
            actual_behavior: Some(EvalActualBehavior {
                summary: "Verification failed with parser_empty_input panic".into(),
                error_excerpt: Some("thread panicked at parser.rs:42".into()),
            }),
            verifiers: vec![EvalVerifier {
                name: "parser tests".into(),
                command: Some("cargo test -p imp-core parser_empty_input".into()),
                required: true,
                last_status: Some("failed".into()),
                exit_code: Some(101),
                output_ref: Some(".imp/runs/run-abc/verification/parser-tests.txt".into()),
                failure_excerpt: Some("parser_empty_input failed".into()),
            }],
            artifact_refs: vec![EvalArtifactRef {
                kind: "evidence".into(),
                path: ".imp/runs/run-abc/evidence.md".into(),
                summary: Some("run evidence".into()),
                sha256: None,
            }],
            privacy: EvalPrivacy {
                redaction_status: EvalRedactionStatus::Redacted,
                redaction_rules: vec!["secrets".into()],
                contains_sensitive_data: false,
            },
            ..EvalCandidate::new(
                "2026-05-14-run-abc-verification-failed",
                EvalFailureMode::VerificationFailed,
            )
        }
    }

    #[test]
    fn eval_candidate_serde_roundtrip_preserves_failure_mode_and_refs() {
        let candidate = sample_candidate();
        let json = serde_json::to_string_pretty(&candidate).unwrap();
        assert!(json.contains("verification-failed"));
        assert!(json.contains(".imp/runs/run-abc/evidence.md"));

        let decoded: EvalCandidate = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.failure_mode, EvalFailureMode::VerificationFailed);
        assert_eq!(decoded.artifact_refs[0].kind, "evidence");
        assert_eq!(decoded.verifiers[0].exit_code, Some(101));
    }

    #[test]
    fn eval_candidate_writes_and_reads_candidate_file() {
        let temp = tempfile::TempDir::new().unwrap();
        let candidate = sample_candidate();

        let path = candidate.write_to_dir(temp.path()).unwrap();
        assert_eq!(
            path,
            temp.path()
                .join(EVAL_CANDIDATES_DIR)
                .join(&candidate.id)
                .join("candidate.json")
        );

        let decoded = EvalCandidate::read_from_dir(temp.path(), &candidate.id).unwrap();
        assert_eq!(decoded.id, candidate.id);
        assert_eq!(
            decoded.expected_behavior.summary,
            candidate.expected_behavior.summary
        );
    }

    #[test]
    fn eval_candidate_path_helpers_use_project_local_eval_directory() {
        let root = Path::new("/tmp/project");
        assert_eq!(
            EvalCandidate::candidate_dir(root, "candidate-1"),
            PathBuf::from("/tmp/project/.imp/eval-candidates/candidate-1")
        );
        assert_eq!(
            EvalCandidate::candidate_path(root, "candidate-1"),
            PathBuf::from("/tmp/project/.imp/eval-candidates/candidate-1/candidate.json")
        );
    }

    #[test]
    fn eval_candidate_redaction_removes_secret_like_content_before_serializing() {
        let secret = "AbcdEfghIjkl1234567890";
        let candidate = EvalCandidate {
            id: "secret-candidate".into(),
            prompt: Some(format!("use token {secret}")),
            expected_behavior: EvalExpectedBehavior {
                summary: format!("must not leak {secret}"),
                assertions: vec![format!("command with {secret} fails safely")],
            },
            actual_behavior: Some(EvalActualBehavior {
                summary: "failed".into(),
                error_excerpt: Some(format!("stderr included {secret}")),
            }),
            verifiers: vec![EvalVerifier {
                name: "secret verifier".into(),
                command: Some(format!("echo {secret}")),
                failure_excerpt: Some(format!("saw {secret}")),
                ..EvalVerifier::default()
            }],
            trust: EvalTrustSummary {
                warnings: vec![format!("low-trust output contained {secret}")],
                ..EvalTrustSummary::default()
            },
            ..EvalCandidate::new("secret-candidate", EvalFailureMode::Manual)
        };

        let redacted = redact_eval_candidate(candidate);
        let json = serde_json::to_string(&redacted).unwrap();

        assert!(!json.contains(secret));
        assert!(json.contains("[REDACTED_SECRET]"));
        assert_eq!(
            redacted.privacy.redaction_status,
            EvalRedactionStatus::Redacted
        );
        assert!(redacted
            .privacy
            .redaction_rules
            .contains(&"secret-like-patterns".into()));
        assert!(!redacted.privacy.contains_sensitive_data);
    }

    #[test]
    fn eval_candidate_trust_summary_roundtrips() {
        let candidate = EvalCandidate {
            id: "trust-candidate".into(),
            trust: EvalTrustSummary {
                sources: vec!["source=ToolResult; trust=ExternalUntrusted".into()],
                low_trust_influences: vec!["untrusted web content".into()],
                warnings: vec!["external content cannot authorize escalation".into()],
            },
            ..EvalCandidate::new("trust-candidate", EvalFailureMode::Manual)
        };

        let json = serde_json::to_string(&candidate).unwrap();
        let decoded: EvalCandidate = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.trust, candidate.trust);
    }
}
