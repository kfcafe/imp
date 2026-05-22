use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    Check, CheckKind, GlobalWorkStore, Link, LinkKind, MemoryItem, MemoryKind, SourceKind,
    SourceRef, Task, TaskStatus, WorkId, WorkStore,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManaMigrationReport {
    pub source: PathBuf,
    pub dry_run: bool,
    pub imported_units: usize,
    pub skipped_files: Vec<PathBuf>,
    pub matched_fields: BTreeSet<String>,
    pub missing_fields: BTreeSet<String>,
    pub lossy_mappings: Vec<String>,
    pub warnings: Vec<String>,
    pub archived_units: usize,
    pub history_refs: Vec<ManaHistoryRef>,
    pub imported: Vec<ManaShadowImport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManaHistoryRef {
    pub unit_id: String,
    pub source_path: PathBuf,
    pub archived: bool,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectLocalMigrationReport {
    pub source: PathBuf,
    pub project_root: PathBuf,
    pub global_store_root: PathBuf,
    pub dry_run: bool,
    pub tasks: usize,
    pub memory_items: usize,
    pub decisions: usize,
    pub skipped: Vec<String>,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
}

impl ManaMigrationReport {
    pub fn parity_summary(&self) -> String {
        format!(
            "imported={} archived={} skipped={} missing={} lossy={} warnings={}",
            self.imported_units,
            self.archived_units,
            self.skipped_files.len(),
            self.missing_fields.len(),
            self.lossy_mappings.len(),
            self.warnings.len()
        )
    }
}

pub fn dry_run_mana_migration(path: impl AsRef<Path>) -> crate::Result<ManaMigrationReport> {
    migrate_mana(path, None)
}

pub fn migrate_mana_to_store(
    path: impl AsRef<Path>,
    store: &WorkStore,
) -> crate::Result<ManaMigrationReport> {
    migrate_mana(path, Some(store))
}

pub fn dry_run_project_local_migration(
    project_root: impl AsRef<Path>,
    local_store: &WorkStore,
    global_store: &GlobalWorkStore,
) -> crate::Result<ProjectLocalMigrationReport> {
    project_local_migration_report(project_root, local_store, global_store, true)
}

pub fn migrate_project_local_to_global(
    project_root: impl AsRef<Path>,
    local_store: &WorkStore,
    global_store: &GlobalWorkStore,
) -> crate::Result<ProjectLocalMigrationReport> {
    let report = project_local_migration_report(&project_root, local_store, global_store, false)?;
    let project_root = normalize_project_root(project_root);
    for task in local_store.load_tasks()? {
        global_store.append_task(&project_root, &task)?;
    }
    for memory in local_store.load_memory_index()?.all_items() {
        global_store.append_memory(&project_root, memory)?;
    }
    for decision in local_store.load_decisions()? {
        global_store.append_decision(&project_root, &decision)?;
    }
    Ok(report)
}

fn project_local_migration_report(
    project_root: impl AsRef<Path>,
    local_store: &WorkStore,
    global_store: &GlobalWorkStore,
    dry_run: bool,
) -> crate::Result<ProjectLocalMigrationReport> {
    let project_root = normalize_project_root(project_root);
    let tasks = local_store.load_tasks()?;
    let memory = local_store.load_memory_index()?;
    let decisions = local_store.load_decisions()?;
    let existing = global_store.tasks_for_project(&project_root)?;
    let mut conflicts = Vec::new();
    for task in &tasks {
        if existing.iter().any(|candidate| candidate.id == task.id) {
            conflicts.push(format!("task {} already exists in global project store", task.id));
        }
    }
    let mut warnings = Vec::new();
    if tasks.is_empty() && memory.all_items().is_empty() && decisions.is_empty() {
        warnings.push("project-local imp-work store contains no migratable tasks, memory, or decisions".to_string());
    }
    Ok(ProjectLocalMigrationReport {
        source: local_store.root().to_path_buf(),
        project_root,
        global_store_root: global_store.root().to_path_buf(),
        dry_run,
        tasks: tasks.len(),
        memory_items: memory.all_items().len(),
        decisions: decisions.len(),
        skipped: Vec::new(),
        conflicts,
        warnings,
    })
}

fn normalize_project_root(project_root: impl AsRef<Path>) -> PathBuf {
    project_root
        .as_ref()
        .canonicalize()
        .unwrap_or_else(|_| project_root.as_ref().to_path_buf())
}

fn migrate_mana(
    path: impl AsRef<Path>,
    store: Option<&WorkStore>,
) -> crate::Result<ManaMigrationReport> {
    let source = path.as_ref();
    let files = mana_unit_files(source)?;
    let mut report = ManaMigrationReport {
        source: source.to_path_buf(),
        dry_run: store.is_none(),
        imported_units: 0,
        skipped_files: Vec::new(),
        matched_fields: BTreeSet::new(),
        missing_fields: BTreeSet::new(),
        lossy_mappings: Vec::new(),
        warnings: Vec::new(),
        archived_units: 0,
        history_refs: Vec::new(),
        imported: Vec::new(),
    };

    for file in files {
        let content = fs::read_to_string(&file)?;
        let Some(unit) = parse_mana_unit_file(&content) else {
            report.skipped_files.push(file);
            continue;
        };
        let archived = is_archived_mana_path(&file);
        if archived {
            report.archived_units += 1;
            report.matched_fields.insert("archive_history".to_string());
            report.lossy_mappings.push(format!(
                "{}: archive/history source preserved as migration metadata",
                unit.id
            ));
        }
        report.history_refs.push(ManaHistoryRef {
            unit_id: unit.id.clone(),
            source_path: file.clone(),
            archived,
            status: unit.status.clone(),
        });
        record_field_parity(&unit, &mut report);
        let imported = import_mana_unit_shadow(&unit);
        report.warnings.extend(
            imported
                .warnings
                .iter()
                .map(|warning| format!("{}: {warning}", unit.id)),
        );
        if let Some(store) = store {
            store.append_task(&imported.task)?;
            for memory in &imported.context_memory {
                store.append_memory(memory)?;
            }
        }
        report.imported_units += 1;
        report.imported.push(imported);
    }

    Ok(report)
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

fn mana_unit_files(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    let mut files = Vec::new();
    collect_mana_unit_files(path, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_mana_unit_files(path: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let candidate = entry.path();
        if candidate.is_dir() {
            collect_mana_unit_files(&candidate, files)?;
            continue;
        }
        if candidate.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(candidate);
        }
    }
    Ok(())
}

fn is_archived_mana_path(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "archive")
}

fn parse_mana_unit_file(content: &str) -> Option<ManaShadowUnit> {
    let (frontmatter, body) = split_frontmatter(content)?;
    let mut unit = ManaShadowUnit {
        id: string_field(frontmatter, "id")?,
        title: string_field(frontmatter, "title")
            .unwrap_or_else(|| "Untitled mana unit".to_string()),
        status: string_field(frontmatter, "status").unwrap_or_else(|| "open".to_string()),
        description: section(body, &["Description", "Context"]),
        design: string_field(frontmatter, "design").or_else(|| section(body, &["Design"])),
        acceptance: string_field(frontmatter, "acceptance")
            .or_else(|| section(body, &["Acceptance", "Acceptance Criteria"])),
        verify: string_field(frontmatter, "verify"),
        verify_timeout: number_field(frontmatter, "verify_timeout"),
        paths: string_list_field(frontmatter, "paths")
            .into_iter()
            .map(PathBuf::from)
            .collect(),
        dependencies: string_list_field(frontmatter, "deps"),
        notes: sections_with_prefix(body, "note")
            .into_iter()
            .chain(sections_with_prefix(body, "log"))
            .collect(),
        decisions: sections_with_prefix(body, "decision"),
        labels: string_list_field(frontmatter, "labels"),
        priority: number_field(frontmatter, "priority").map(|value| value as i64),
    };
    if unit.description.is_none() {
        unit.description = body_summary(body);
    }
    Some(unit)
}

fn split_frontmatter(content: &str) -> Option<(&str, &str)> {
    let rest = content.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    let frontmatter = &rest[..end];
    let body_start = end + "\n---".len();
    let body = rest
        .get(body_start..)
        .unwrap_or_default()
        .trim_start_matches('\n');
    Some((frontmatter, body))
}

fn string_field(frontmatter: &str, key: &str) -> Option<String> {
    let value = frontmatter_value(frontmatter, key)?;
    if value.starts_with('[') {
        return None;
    }
    Some(unquote(value).trim().to_string()).filter(|value| !value.is_empty())
}

fn number_field(frontmatter: &str, key: &str) -> Option<u64> {
    string_field(frontmatter, key)?.parse().ok()
}

fn string_list_field(frontmatter: &str, key: &str) -> Vec<String> {
    let Some(value) = frontmatter_value(frontmatter, key) else {
        return Vec::new();
    };
    if value.starts_with('[') {
        let jsonish = yamlish_inline_list_to_json(value);
        let Ok(parsed) = serde_json::from_str::<Value>(&jsonish) else {
            return Vec::new();
        };
        return parsed
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| {
                item.as_str()
                    .map(str::to_string)
                    .or_else(|| item.as_i64().map(|value| value.to_string()))
            })
            .collect();
    }

    vec![unquote(value).trim().to_string()]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect()
}

fn yamlish_inline_list_to_json(value: &str) -> String {
    let inner = value.trim().trim_start_matches('[').trim_end_matches(']');
    let items = inner
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| format!("\"{}\"", unquote(item)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn frontmatter_value<'a>(frontmatter: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}:");
    frontmatter
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix).map(str::trim))
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
        .or_else(|| {
            value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
        })
        .unwrap_or(value)
}

fn section(body: &str, names: &[&str]) -> Option<String> {
    for name in names {
        if let Some(section) = named_section(body, name) {
            return Some(section);
        }
    }
    None
}

fn named_section(body: &str, name: &str) -> Option<String> {
    let mut in_section = false;
    let mut lines = Vec::new();
    let wanted = name.to_ascii_lowercase();
    for line in body.lines() {
        if let Some(heading) = line.trim_start().strip_prefix("#") {
            let heading = heading.trim_start_matches('#').trim().to_ascii_lowercase();
            if in_section {
                break;
            }
            in_section = heading == wanted;
            continue;
        }
        if in_section {
            lines.push(line);
        }
    }
    let text = lines.join("\n").trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn sections_with_prefix(body: &str, prefix: &str) -> Vec<String> {
    let mut values = Vec::new();
    let prefix = prefix.to_ascii_lowercase();
    let mut current = None::<String>;
    let mut lines = Vec::new();
    for line in body.lines() {
        if let Some(heading) = line.trim_start().strip_prefix("#") {
            if current.take().is_some() {
                let text = lines.join("\n").trim().to_string();
                if !text.is_empty() {
                    values.push(text);
                }
                lines.clear();
            }
            let heading = heading.trim_start_matches('#').trim().to_ascii_lowercase();
            if heading.starts_with(&prefix) {
                current = Some(heading);
            }
            continue;
        }
        if current.is_some() {
            lines.push(line);
        }
    }
    if current.is_some() {
        let text = lines.join("\n").trim().to_string();
        if !text.is_empty() {
            values.push(text);
        }
    }
    values
}

fn body_summary(body: &str) -> Option<String> {
    body.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
}

fn record_field_parity(unit: &ManaShadowUnit, report: &mut ManaMigrationReport) {
    let fields = [
        ("id", Some(unit.id.as_str())),
        ("title", Some(unit.title.as_str())),
        ("status", Some(unit.status.as_str())),
        ("description", unit.description.as_deref()),
        ("design", unit.design.as_deref()),
        ("acceptance", unit.acceptance.as_deref()),
        ("verify", unit.verify.as_deref()),
    ];
    for (field, value) in fields {
        if value.is_some_and(|value| !value.trim().is_empty()) {
            report.matched_fields.insert(field.to_string());
        } else {
            report.missing_fields.insert(field.to_string());
        }
    }
    if unit.verify_timeout.is_some() {
        report.matched_fields.insert("verify_timeout".to_string());
    } else {
        report.missing_fields.insert("verify_timeout".to_string());
    }
    if unit.paths.is_empty() {
        report.missing_fields.insert("paths".to_string());
    } else {
        report.matched_fields.insert("paths".to_string());
    }
    if unit.dependencies.is_empty() {
        report.missing_fields.insert("deps".to_string());
    } else {
        report.matched_fields.insert("deps".to_string());
    }
    if unit.notes.is_empty() {
        report.missing_fields.insert("notes".to_string());
    } else {
        report.matched_fields.insert("notes".to_string());
        report
            .lossy_mappings
            .push(format!("{}: notes imported as memory items", unit.id));
    }
    if unit.decisions.is_empty() {
        report.missing_fields.insert("decisions".to_string());
    } else {
        report.matched_fields.insert("decisions".to_string());
        report
            .lossy_mappings
            .push(format!("{}: decisions imported as memory items", unit.id));
    }
    if !unit.labels.is_empty() {
        report
            .lossy_mappings
            .push(format!("{}: labels recorded in report only", unit.id));
    }
    if unit.priority.is_some() {
        report
            .lossy_mappings
            .push(format!("{}: priority recorded in report only", unit.id));
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
        topics: vec!["mana_migration".to_string(), mana_id.to_string()],
        parent_work: Some(WorkId(format!("mana-{mana_id}"))),
        paths: paths.to_vec(),
        source_refs: vec![source_ref.clone()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn project_local_migration_dry_run_reports_without_writing_global_store() {
        let temp = tempdir().unwrap();
        let project = temp.path().join("project");
        fs::create_dir_all(&project).unwrap();
        let local = WorkStore::open(project.join(".imp").join("work"));
        let global = GlobalWorkStore::open(temp.path().join("global-work"));
        let mut task = Task::new("Local task");
        task.id = WorkId::from("local-task");
        local.append_task(&task).unwrap();
        local
            .append_memory(&MemoryItem::new(MemoryKind::Fact, "Local memory"))
            .unwrap();

        let report = dry_run_project_local_migration(&project, &local, &global).unwrap();

        assert!(report.dry_run);
        assert_eq!(report.source, local.root());
        assert_eq!(report.project_root, project.canonicalize().unwrap());
        assert_eq!(report.global_store_root, global.root());
        assert_eq!(report.tasks, 1);
        assert_eq!(report.memory_items, 1);
        assert_eq!(report.decisions, 0);
        assert!(report.conflicts.is_empty());
        assert!(global.tasks_for_project(&project).unwrap().is_empty());
        assert!(global.memories_for_project(&project).unwrap().is_empty());
    }

    #[test]
    fn project_local_migration_write_mode_imports_project_scoped_records_and_reports_conflicts() {
        let temp = tempdir().unwrap();
        let project = temp.path().join("project");
        fs::create_dir_all(&project).unwrap();
        let local = WorkStore::open(project.join(".imp").join("work"));
        let global = GlobalWorkStore::open(temp.path().join("global-work"));
        let mut task = Task::new("Local task");
        task.id = WorkId::from("local-task");
        local.append_task(&task).unwrap();
        global.append_task(&project, &task).unwrap();

        let dry_run = dry_run_project_local_migration(&project, &local, &global).unwrap();
        assert_eq!(dry_run.conflicts.len(), 1);
        assert!(dry_run.conflicts[0].contains("local-task"));

        let report = migrate_project_local_to_global(&project, &local, &global).unwrap();

        assert!(!report.dry_run);
        assert_eq!(report.conflicts.len(), 1);
        let tasks = global.tasks_for_project(&project).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, WorkId::from("local-task"));
    }

    #[test]
    fn mana_migration_dry_run_imports_real_frontmatter_shape_without_writing() {
        let temp = tempdir().unwrap();
        let mana_dir = temp.path().join(".mana");
        fs::create_dir(&mana_dir).unwrap();
        let unit_path = mana_dir.join("1-test-unit.md");
        let original = "---
id: '1'
title: Test imported unit
status: open
priority: 2
labels: [imp-work, migration]
deps: ['0']
paths: [crates/imp-work, docs/design/plan.md]
verify: cargo test -p imp-work
verify_timeout: 120
---
# Description
Import this mana unit.

# Design
Use a dry-run importer.

# Acceptance
- Preserve verify
- Preserve paths

# Note 1
Existing mana graph stays untouched.

# Decision 1
No immediate deletion of mana.
";
        fs::write(&unit_path, original).unwrap();

        let report = dry_run_mana_migration(&mana_dir).unwrap();

        assert!(report.dry_run);
        assert_eq!(report.imported_units, 1);
        assert_eq!(fs::read_to_string(&unit_path).unwrap(), original);
        assert!(report.matched_fields.contains("verify"));
        assert!(report.matched_fields.contains("paths"));
        assert!(report
            .lossy_mappings
            .iter()
            .any(|entry| entry.contains("labels")));
        let imported = &report.imported[0];
        assert_eq!(imported.task.id, WorkId("mana-1".to_string()));
        assert_eq!(
            imported.task.acceptance,
            ["Preserve verify", "Preserve paths"]
        );
        assert_eq!(
            imported.task.checks[0].command.as_deref(),
            Some("cargo test -p imp-work")
        );
        assert_eq!(imported.dependency_ids(), ["mana-0"]);
        assert!(imported
            .context_memory
            .iter()
            .any(|item| item.kind == MemoryKind::Decision
                && item.text.contains("No immediate deletion")));
    }

    #[test]
    fn mana_migration_write_mode_persists_imp_work_records_only() {
        let temp = tempdir().unwrap();
        let mana_dir = temp.path().join(".mana");
        let work_dir = temp.path().join(".imp").join("work");
        fs::create_dir(&mana_dir).unwrap();
        fs::write(
            mana_dir.join("2-write-unit.md"),
            "---
id: '2'
title: Write imported unit
status: closed
verify: cargo check
---
# Description
Persist this unit.
",
        )
        .unwrap();
        let store = WorkStore::open(&work_dir);

        let report = migrate_mana_to_store(&mana_dir, &store).unwrap();

        assert!(!report.dry_run);
        assert_eq!(report.imported_units, 1);
        let tasks = store.load_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, WorkId("mana-2".to_string()));
        assert_eq!(tasks[0].status, TaskStatus::Done);
        assert_eq!(tasks[0].checks[0].command.as_deref(), Some("cargo check"));
        let memory = store.load_memory_index().unwrap();
        assert!(memory
            .recent(10)
            .iter()
            .any(|item| item.parent_work == Some(WorkId("mana-2".to_string()))));
    }

    #[test]
    fn mana_migration_reports_skipped_and_missing_fields() {
        let temp = tempdir().unwrap();
        let mana_dir = temp.path().join(".mana");
        fs::create_dir(&mana_dir).unwrap();
        fs::write(mana_dir.join("bad.md"), "not a mana unit").unwrap();
        fs::write(
            mana_dir.join("3-minimal.md"),
            "---
id: '3'
title: Minimal
---
Body summary.
",
        )
        .unwrap();

        let report = dry_run_mana_migration(&mana_dir).unwrap();

        assert_eq!(report.imported_units, 1);
        assert_eq!(report.skipped_files.len(), 1);
        assert!(report.missing_fields.contains("verify"));
        assert!(report.missing_fields.contains("paths"));
        assert!(report.parity_summary().contains("imported=1"));
    }

    #[test]
    fn mana_migration_real_project_dry_run_and_scratch_write_are_non_destructive() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .to_path_buf();
        let mana_dir = repo_root.join(".mana");
        if !mana_dir.exists() {
            return;
        }
        let before = fs::read_dir(&mana_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("md"))
            .count();

        let dry_run = dry_run_mana_migration(&mana_dir).unwrap();
        assert!(dry_run.dry_run);
        assert!(dry_run.imported_units > 0);
        assert!(!dry_run.history_refs.is_empty());

        let scratch = tempdir().unwrap();
        let store = WorkStore::open(scratch.path().join("work"));
        let written = migrate_mana_to_store(&mana_dir, &store).unwrap();
        assert!(!written.dry_run);
        assert_eq!(written.imported_units, dry_run.imported_units);
        assert_eq!(written.archived_units, dry_run.archived_units);
        assert!(!store.load_tasks().unwrap().is_empty());

        let after = fs::read_dir(&mana_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("md"))
            .count();
        assert_eq!(after, before, "migration must not mutate .mana file count");
    }

    #[test]
    fn mana_migration_imports_archived_units_and_records_history_refs() {
        let temp = tempdir().unwrap();
        let mana_dir = temp.path().join(".mana");
        let archive_dir = mana_dir.join("archive").join("2026").join("05");
        fs::create_dir_all(&archive_dir).unwrap();
        let active_path = mana_dir.join("4-active.md");
        let archived_path = archive_dir.join("5-archived.md");
        fs::write(
            &active_path,
            "---\nid: '4'\ntitle: Active\nstatus: open\n---\nActive body.\n",
        )
        .unwrap();
        let archived_original = "---\nid: '5'\ntitle: Archived\nstatus: closed\n---\n# Note 1\nClosed with historical context.\n";
        fs::write(&archived_path, archived_original).unwrap();

        let report = dry_run_mana_migration(&mana_dir).unwrap();

        assert_eq!(report.imported_units, 2);
        assert_eq!(report.archived_units, 1);
        assert!(report.matched_fields.contains("archive_history"));
        assert!(report.lossy_mappings.iter().any(|entry| {
            entry.contains("5") && entry.contains("archive/history source preserved")
        }));
        let archived_ref = report
            .history_refs
            .iter()
            .find(|history| history.unit_id == "5")
            .expect("archived history ref");
        assert!(archived_ref.archived);
        assert_eq!(archived_ref.status, "closed");
        assert_eq!(
            fs::read_to_string(&archived_path).unwrap(),
            archived_original
        );
        let archived = report
            .imported
            .iter()
            .find(|imported| imported.task.id == WorkId("mana-5".to_string()))
            .expect("archived import");
        assert_eq!(archived.task.status, TaskStatus::Done);
        assert!(archived
            .context_memory
            .iter()
            .any(|memory| memory.text.contains("historical context")));
    }

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
