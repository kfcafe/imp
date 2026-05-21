use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Result;

/// A bounded, disposable code experiment whose durable output is learning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prototype {
    pub id: String,
    pub title: String,
    pub parent_work: Option<String>,
    pub hypothesis: Option<String>,
    pub question: String,
    pub sandbox: PathBuf,
    pub timebox_seconds: u64,
    pub status: PrototypeStatus,
    pub evidence_required: Vec<String>,
    pub evidence: Vec<PrototypeEvidence>,
    pub learnings: Vec<String>,
    pub decision: Option<PrototypeDecision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrototypeStatus {
    Planned,
    Running,
    Observed,
    Promoted,
    Discarded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypothesisResult {
    Supported,
    Refuted,
    Inconclusive,
    NotAssessed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrototypeEvidence {
    pub claim: String,
    pub proof: String,
    pub artifact: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrototypeDecision {
    pub outcome: PrototypeOutcome,
    pub reason: String,
    pub followups: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrototypeOutcome {
    Promote,
    Discard,
    Iterate,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrototypeRecordPolicy {
    None,
    Memory,
    Prototype,
}

impl Default for PrototypeRecordPolicy {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrototypeObservation {
    pub prototype_id: String,
    pub question: String,
    pub parent_work: Option<String>,
    pub hypothesis: Option<String>,
    pub hypothesis_result: HypothesisResult,
    pub outcome: PrototypeOutcome,
    pub summary: String,
    pub evidence_required: Vec<String>,
    pub evidence: Vec<PrototypeEvidence>,
    pub learnings: Vec<String>,
    pub followups: Vec<String>,
    pub sandbox: PathBuf,
    pub artifacts: Vec<PathBuf>,
}

impl Prototype {
    pub fn new(
        title: impl Into<String>,
        question: impl Into<String>,
        sandbox: impl Into<PathBuf>,
    ) -> Self {
        Self {
            id: format!("P-{}", uuid::Uuid::new_v4().simple()),
            title: title.into(),
            parent_work: None,
            hypothesis: None,
            question: question.into(),
            sandbox: sandbox.into(),
            timebox_seconds: 300,
            status: PrototypeStatus::Planned,
            evidence_required: Vec::new(),
            evidence: Vec::new(),
            learnings: Vec::new(),
            decision: None,
        }
    }

    pub fn with_parent_work(mut self, parent_work: impl Into<String>) -> Self {
        self.parent_work = Some(parent_work.into());
        self
    }

    pub fn with_hypothesis(mut self, hypothesis: impl Into<String>) -> Self {
        self.hypothesis = Some(hypothesis.into());
        self
    }

    pub fn with_timebox_seconds(mut self, seconds: u64) -> Self {
        self.timebox_seconds = seconds;
        self
    }

    pub fn with_evidence_required(mut self, evidence_required: Vec<String>) -> Self {
        self.evidence_required = evidence_required;
        self
    }

    pub fn record_evidence(
        &mut self,
        claim: impl Into<String>,
        proof: impl Into<String>,
        artifact: Option<PathBuf>,
    ) {
        self.evidence.push(PrototypeEvidence {
            claim: claim.into(),
            proof: proof.into(),
            artifact,
        });
        if self.status == PrototypeStatus::Running || self.status == PrototypeStatus::Planned {
            self.status = PrototypeStatus::Observed;
        }
    }

    pub fn record_learning(&mut self, learning: impl Into<String>) {
        self.learnings.push(learning.into());
    }

    pub fn decide(
        &mut self,
        outcome: PrototypeOutcome,
        reason: impl Into<String>,
        followups: Vec<String>,
    ) {
        self.status = match outcome {
            PrototypeOutcome::Promote => PrototypeStatus::Promoted,
            PrototypeOutcome::Discard => PrototypeStatus::Discarded,
            PrototypeOutcome::Iterate | PrototypeOutcome::Inconclusive => PrototypeStatus::Observed,
        };
        self.decision = Some(PrototypeDecision {
            outcome,
            reason: reason.into(),
            followups,
        });
    }
}

/// Minimal file-backed reconciliation for prototype learnings.
///
/// This is intentionally append-only and human-readable while imp-work's full store is still
/// taking shape. It gives the runtime loop a durable landing zone without pretending the final
/// task/memory graph exists yet.
pub struct PrototypeJournal {
    root: PathBuf,
}

impl PrototypeJournal {
    pub fn open(project_root: impl Into<PathBuf>) -> Self {
        Self {
            root: project_root.into(),
        }
    }

    pub fn record(
        &self,
        policy: PrototypeRecordPolicy,
        observation: &PrototypeObservation,
    ) -> Result<Option<PathBuf>> {
        match policy {
            PrototypeRecordPolicy::None => Ok(None),
            PrototypeRecordPolicy::Memory => {
                let path = self.work_dir().join("memory.md");
                self.append_memory(&path, observation)?;
                Ok(Some(path))
            }
            PrototypeRecordPolicy::Prototype => {
                let path = self.work_dir().join("prototypes.md");
                self.append_prototype(&path, observation)?;
                Ok(Some(path))
            }
        }
    }

    fn work_dir(&self) -> PathBuf {
        self.root.join(".imp").join("work")
    }

    fn append_memory(&self, path: &Path, observation: &PrototypeObservation) -> Result<()> {
        ensure_parent(path)?;
        let mut entry = String::new();
        if !path.exists() || fs::metadata(path)?.len() == 0 {
            entry.push_str("# Memory\n\n");
        }
        for learning in &observation.learnings {
            entry.push_str(&format!("- {} @prototype-learning\n", one_line(learning)));
            entry.push_str(&format!("  source: {}\n", observation.prototype_id));
            if let Some(parent_work) = &observation.parent_work {
                entry.push_str(&format!("  parent_work: {}\n", one_line(parent_work)));
            }
        }
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?
            .write_all_str(&entry)?;
        Ok(())
    }

    fn append_prototype(&self, path: &Path, observation: &PrototypeObservation) -> Result<()> {
        ensure_parent(path)?;
        let mut entry = String::new();
        if !path.exists() || fs::metadata(path)?.len() == 0 {
            entry.push_str("# Prototypes\n\n");
        }
        entry.push_str(&format!(
            "- {} @prototype @{}\n",
            one_line(&observation.question),
            observation.outcome
        ));
        entry.push_str(&format!("  id: {}\n", observation.prototype_id));
        if let Some(parent_work) = &observation.parent_work {
            entry.push_str(&format!("  parent_work: {}\n", one_line(parent_work)));
        }
        if let Some(hypothesis) = &observation.hypothesis {
            entry.push_str(&format!("  hypothesis: {}\n", one_line(hypothesis)));
        }
        entry.push_str(&format!(
            "  hypothesis_result: {:?}\n",
            observation.hypothesis_result
        ));
        entry.push_str(&format!("  summary: {}\n", one_line(&observation.summary)));
        entry.push_str(&format!("  sandbox: {}\n", observation.sandbox.display()));
        if !observation.evidence_required.is_empty() {
            entry.push_str("  evidence_required:\n");
            for item in &observation.evidence_required {
                entry.push_str(&format!("    - {}\n", one_line(item)));
            }
        }
        if !observation.evidence.is_empty() {
            entry.push_str("  evidence:\n");
            for evidence in &observation.evidence {
                entry.push_str(&format!(
                    "    - {} — {}\n",
                    one_line(&evidence.claim),
                    one_line(&evidence.proof)
                ));
            }
        }
        if !observation.learnings.is_empty() {
            entry.push_str("  learnings:\n");
            for learning in &observation.learnings {
                entry.push_str(&format!("    - {}\n", one_line(learning)));
            }
        }
        if !observation.followups.is_empty() {
            entry.push_str("  followups:\n");
            for followup in &observation.followups {
                entry.push_str(&format!("    - {}\n", one_line(followup)));
            }
        }
        entry.push('\n');
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?
            .write_all_str(&entry)?;
        Ok(())
    }
}

fn ensure_parent(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

trait WriteAllStr {
    fn write_all_str(&mut self, value: &str) -> std::io::Result<()>;
}

impl WriteAllStr for fs::File {
    fn write_all_str(&mut self, value: &str) -> std::io::Result<()> {
        use std::io::Write;
        self.write_all(value.as_bytes())
    }
}

impl std::fmt::Display for PrototypeOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Promote => f.write_str("promote"),
            Self::Discard => f.write_str("discard"),
            Self::Iterate => f.write_str("iterate"),
            Self::Inconclusive => f.write_str("inconclusive"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prototype_records_evidence_and_promotion_decision() {
        let mut prototype = Prototype::new(
            "Cache-stable context rendering",
            "Can dynamic launch metadata stay outside stable context hashes?",
            ".tmp/imp-prototypes/P-cache",
        )
        .with_parent_work("T-context-renderer")
        .with_hypothesis("Only the dynamic suffix hash should change.")
        .with_timebox_seconds(120)
        .with_evidence_required(vec!["stable prefix hash".into()]);

        prototype.status = PrototypeStatus::Running;
        prototype.record_evidence("stable prefix", "hash unchanged across 10 renders", None);
        prototype.record_learning("Keep timestamps out of cacheable blocks.");
        prototype.decide(
            PrototypeOutcome::Promote,
            "Evidence supports explicit stable/dynamic block split.",
            vec!["Implement ContextBlockStability".into()],
        );

        assert_eq!(prototype.status, PrototypeStatus::Promoted);
        assert_eq!(prototype.parent_work.as_deref(), Some("T-context-renderer"));
        assert_eq!(prototype.evidence_required.len(), 1);
        assert_eq!(prototype.evidence.len(), 1);
        assert_eq!(prototype.learnings.len(), 1);
        assert_eq!(
            prototype.decision.as_ref().map(|d| d.outcome),
            Some(PrototypeOutcome::Promote)
        );
    }

    #[test]
    fn prototype_journal_records_memory_and_prototype_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let observation = PrototypeObservation {
            prototype_id: "P-test".into(),
            question: "Can prototypes write durable learnings?".into(),
            parent_work: Some("T-prototype".into()),
            hypothesis: Some("Append-only markdown is enough for first reconciliation.".into()),
            hypothesis_result: HypothesisResult::Supported,
            outcome: PrototypeOutcome::Promote,
            summary: "The journal wrote memory and prototype entries.".into(),
            evidence_required: vec!["memory.md contains learning".into()],
            evidence: vec![PrototypeEvidence {
                claim: "memory recorded".into(),
                proof: "file contained prototype-learning tag".into(),
                artifact: None,
            }],
            learnings: vec![
                "Prototype learning should be durable even when sandbox code is deleted.".into(),
            ],
            followups: vec!["Replace append-only journal with full file store.".into()],
            sandbox: tmp.path().join("sandbox"),
            artifacts: Vec::new(),
        };
        let journal = PrototypeJournal::open(tmp.path());

        let memory_path = journal
            .record(PrototypeRecordPolicy::Memory, &observation)
            .unwrap()
            .unwrap();
        let prototype_path = journal
            .record(PrototypeRecordPolicy::Prototype, &observation)
            .unwrap()
            .unwrap();

        let memory = fs::read_to_string(memory_path).unwrap();
        let prototypes = fs::read_to_string(prototype_path).unwrap();
        assert!(memory.contains("@prototype-learning"));
        assert!(prototypes.contains("@prototype @promote"));
        assert!(prototypes.contains("parent_work: T-prototype"));
    }
}
