use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{CheckResult, RunOutcome, WorkId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EventCursor(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkEvent {
    pub cursor: EventCursor,
    pub kind: WorkEventKind,
    pub work_id: Option<WorkId>,
    pub run_id: Option<WorkId>,
    pub lease_id: Option<WorkId>,
    pub summary: String,
    pub artifacts: Vec<ArtifactRef>,
}

impl WorkEvent {
    pub fn new(kind: WorkEventKind, summary: impl Into<String>) -> Self {
        Self {
            cursor: EventCursor(0),
            kind,
            work_id: None,
            run_id: None,
            lease_id: None,
            summary: summary.into(),
            artifacts: Vec::new(),
        }
    }

    pub fn for_work(mut self, work_id: WorkId) -> Self {
        self.work_id = Some(work_id);
        self
    }

    pub fn with_artifact(mut self, artifact: ArtifactRef) -> Self {
        self.artifacts.push(artifact);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum WorkEventKind {
    TaskCreated,
    TaskUpdated,
    TaskClosed { outcome: RunOutcome },
    TaskFailed,
    CheckRecorded { passed: bool },
    RunStarted,
    RunCompleted { outcome: RunOutcome },
    LeaseAcquired,
    LeaseHeartbeat,
    LeaseReleased,
    PrototypeRecorded,
    EvidenceAppended,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub id: WorkId,
    pub kind: ArtifactKind,
    pub path: Option<PathBuf>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Log,
    CheckOutput,
    Patch,
    PrototypeOutput,
    Evidence,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSummary {
    pub summary: String,
    pub artifacts: Vec<ArtifactRef>,
    pub checks: Vec<CheckResult>,
}

pub struct EventLog {
    path: PathBuf,
}

impl EventLog {
    pub fn open(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn append(&self, event: &WorkEvent) -> crate::Result<WorkEvent> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let next = self.next_cursor()?;
        let mut event = event.clone();
        event.cursor = EventCursor(next);
        let mut line = serde_json::to_string(&event)?;
        line.push('\n');
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?
            .write_all(line.as_bytes())?;
        Ok(event)
    }

    pub fn read_after(&self, cursor: Option<EventCursor>) -> crate::Result<Vec<WorkEvent>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let min = cursor.map(|cursor| cursor.0).unwrap_or(0);
        let content = fs::read_to_string(&self.path)?;
        let mut events = Vec::new();
        for line in content.lines().filter(|line| !line.trim().is_empty()) {
            let event: WorkEvent = serde_json::from_str(line)?;
            if event.cursor.0 > min {
                events.push(event);
            }
        }
        Ok(events)
    }

    fn next_cursor(&self) -> crate::Result<u64> {
        Ok(self
            .read_after(None)?
            .last()
            .map(|event| event.cursor.0 + 1)
            .unwrap_or(1))
    }
}

use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn event_log_appends_and_reads_after_cursor_in_order() {
        let temp = tempdir().unwrap();
        let log = EventLog::open(temp.path().join("events.jsonl"));

        let first = log
            .append(
                &WorkEvent::new(WorkEventKind::TaskCreated, "created")
                    .for_work(WorkId::from("T-1")),
            )
            .unwrap();
        let second = log
            .append(&WorkEvent::new(
                WorkEventKind::CheckRecorded { passed: true },
                "verified",
            ))
            .unwrap();

        assert_eq!(first.cursor, EventCursor(1));
        assert_eq!(second.cursor, EventCursor(2));
        let after_first = log.read_after(Some(first.cursor)).unwrap();
        assert_eq!(after_first, vec![second]);
    }

    #[test]
    fn event_log_preserves_artifact_refs_for_evidence() {
        let temp = tempdir().unwrap();
        let log = EventLog::open(temp.path().join("events.jsonl"));
        let event = WorkEvent::new(WorkEventKind::EvidenceAppended, "captured check output")
            .with_artifact(ArtifactRef {
                id: WorkId::from("A-check"),
                kind: ArtifactKind::CheckOutput,
                path: Some(PathBuf::from("logs/check.txt")),
                summary: Some("cargo test output".into()),
            });

        log.append(&event).unwrap();
        let events = log.read_after(None).unwrap();

        assert_eq!(events[0].artifacts[0].kind, ArtifactKind::CheckOutput);
        assert_eq!(
            events[0].artifacts[0].path.as_deref(),
            Some(Path::new("logs/check.txt"))
        );
    }
}
