use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    Check, CheckKind, Link, LinkKind, MemoryItem, MemoryKind, SourceKind, SourceRef, Task,
    TaskStatus, WorkId,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManaShadowUnit {
    pub id: String,
    pub title: String,
    pub status: String,
    pub description: Option<String>,
    pub design: Option<String>,
    pub acceptance: Option<String>,
    pub verify: Option<String>,
    pub verify_timeout: Option<u64>,
    pub paths: Vec<PathBuf>,
    pub dependencies: Vec<String>,
    pub notes: Vec<String>,
    pub decisions: Vec<String>,
    pub labels: Vec<String>,
    pub priority: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManaShadowImport {
    pub task: Task,
    pub context_memory: Vec<MemoryItem>,
    pub warnings: Vec<String>,
}

impl ManaShadowImport {
    pub fn dependency_ids(&self) -> Vec<&str> {
        self.task
            .links
            .iter()
            .filter(|link| link.kind == LinkKind::DependsOn)
            .map(|link| link.target.0.as_str())
            .collect()
    }
}

pub fn import_mana_unit_shadow(unit: &ManaShadowUnit) -> ManaShadowImport {
    let mana_ref = SourceRef {
        kind: SourceKind::WorkItem,
        reference: format!("mana:{}", unit.id),
        fingerprint: None,
    };

    let mut task = Task::new(unit.title.clone());
    task.id = WorkId(format!("mana-{}", unit.id));
    task.status = map_mana_status(&unit.status);
    task.acceptance = split_acceptance(unit.acceptance.as_deref());
    task.checks = unit
        .verify
        .as_ref()
        .map(|command| Check {
            id: WorkId(format!("mana-{}-verify", unit.id)),
            kind: CheckKind::Command,
            description: match unit.verify_timeout {
                Some(timeout) => format!("mana verify command (timeout {timeout}s)"),
                None => "mana verify command".to_string(),
            },
            command: Some(command.clone()),
        })
        .into_iter()
        .collect();
    task.links = unit
        .dependencies
        .iter()
        .map(|dep| Link {
            kind: LinkKind::DependsOn,
            target: WorkId(format!("mana-{dep}")),
        })
        .collect();
    task.source_refs = vec![mana_ref.clone()];

    let mut context_memory = Vec::new();
    if let Some(description) = unit
        .description
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        context_memory.push(shadow_memory(
            &unit.id,
            "description",
            MemoryKind::Note,
            description,
            &unit.paths,
            &mana_ref,
        ));
    }
    if let Some(design) = unit
        .design
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        context_memory.push(shadow_memory(
            &unit.id,
            "design",
            MemoryKind::Note,
            design,
            &unit.paths,
            &mana_ref,
        ));
    }
    for (index, note) in unit.notes.iter().enumerate() {
        if !note.trim().is_empty() {
            context_memory.push(shadow_memory(
                &unit.id,
                &format!("note-{index}"),
                MemoryKind::Note,
                note,
                &unit.paths,
                &mana_ref,
            ));
        }
    }
    for (index, decision) in unit.decisions.iter().enumerate() {
        if !decision.trim().is_empty() {
            context_memory.push(shadow_memory(
                &unit.id,
                &format!("decision-{index}"),
                MemoryKind::Decision,
                decision,
                &unit.paths,
                &mana_ref,
            ));
        }
    }

    let mut warnings = Vec::new();
    if unit.verify.is_none() {
        warnings.push("mana unit has no verify command".to_string());
    }
    if unit.dependencies.is_empty() && unit.status.eq_ignore_ascii_case("blocked") {
        warnings.push("mana unit is blocked without dependency metadata".to_string());
    }

    ManaShadowImport {
        task,
        context_memory,
        warnings,
    }
}

fn map_mana_status(status: &str) -> TaskStatus {
    match status {
        "open" | "todo" => TaskStatus::Todo,
        "ready" => TaskStatus::Ready,
        "active" | "claimed" | "in_progress" => TaskStatus::Active,
        "blocked" | "failed" => TaskStatus::Blocked,
        "review" => TaskStatus::Review,
        "closed" | "done" => TaskStatus::Done,
        "dropped" | "archived" => TaskStatus::Dropped,
        _ => TaskStatus::Todo,
    }
}

fn split_acceptance(acceptance: Option<&str>) -> Vec<String> {
    acceptance
        .unwrap_or_default()
        .lines()
        .map(|line| line.trim().trim_start_matches('-').trim())
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn shadow_memory(
    mana_id: &str,
    suffix: &str,
    kind: MemoryKind,
    text: &str,
    paths: &[PathBuf],
    source_ref: &SourceRef,
) -> MemoryItem {
    MemoryItem {
        id: WorkId(format!("mana-{mana_id}-{suffix}")),
        kind,
        text: text.to_string(),
        topics: vec!["mana_shadow_import".to_string(), mana_id.to_string()],
        parent_work: Some(WorkId(format!("mana-{mana_id}"))),
        paths: paths.to_vec(),
        source_refs: vec![source_ref.clone()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_import_preserves_open_unit_verify_dependencies_and_context() {
        let unit = ManaShadowUnit {
            id: "357.1".to_string(),
            title: "Implement shadow import".to_string(),
            status: "open".to_string(),
            description: Some("Read mana without mutating it.".to_string()),
            design: Some("Adapter-first migration.".to_string()),
            acceptance: Some("- Preserve verify\n- Preserve deps".to_string()),
            verify: Some("cargo test -p imp-work".to_string()),
            verify_timeout: Some(300),
            paths: vec![PathBuf::from("crates/imp-work")],
            dependencies: vec!["44".to_string(), "50.16".to_string()],
            notes: vec!["Existing mana graphs stay authoritative.".to_string()],
            decisions: vec!["No immediate deletion of mana.".to_string()],
            labels: vec!["imp-work".to_string(), "migration".to_string()],
            priority: Some(0),
        };

        let imported = import_mana_unit_shadow(&unit);

        assert_eq!(imported.task.id, WorkId("mana-357.1".to_string()));
        assert_eq!(imported.task.status, TaskStatus::Todo);
        assert_eq!(
            imported.task.acceptance,
            ["Preserve verify", "Preserve deps"]
        );
        assert_eq!(imported.task.checks.len(), 1);
        assert_eq!(
            imported.task.checks[0].command.as_deref(),
            Some("cargo test -p imp-work")
        );
        assert_eq!(imported.dependency_ids(), ["mana-44", "mana-50.16"]);
        assert_eq!(imported.context_memory.len(), 4);
        assert!(imported
            .context_memory
            .iter()
            .any(|item| item.kind == MemoryKind::Decision
                && item.text.contains("No immediate deletion")));
        assert!(imported.warnings.is_empty());
    }

    #[test]
    fn shadow_import_warns_without_mutating_mana_state() {
        let unit = ManaShadowUnit {
            id: "blocked".to_string(),
            title: "Blocked unit".to_string(),
            status: "blocked".to_string(),
            description: None,
            design: None,
            acceptance: None,
            verify: None,
            verify_timeout: None,
            paths: Vec::new(),
            dependencies: Vec::new(),
            notes: Vec::new(),
            decisions: Vec::new(),
            labels: Vec::new(),
            priority: None,
        };

        let imported = import_mana_unit_shadow(&unit);

        assert_eq!(imported.task.status, TaskStatus::Blocked);
        assert!(imported.task.checks.is_empty());
        assert_eq!(
            imported.warnings,
            [
                "mana unit has no verify command",
                "mana unit is blocked without dependency metadata"
            ]
        );
        assert_eq!(imported.task.source_refs[0].reference, "mana:blocked");
    }
}
