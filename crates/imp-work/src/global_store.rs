use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Task, WorkId};

#[derive(Debug, Clone)]
pub struct GlobalWorkStore {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectScopedTask {
    pub project_root: PathBuf,
    pub stream_id: Option<String>,
    pub task: Task,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectWorkStream {
    pub project_root: PathBuf,
    pub stream_id: String,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamEvent {
    pub project_root: PathBuf,
    pub stream_id: String,
    pub work_id: Option<WorkId>,
    pub relation: StreamRelation,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamRelation {
    Opened,
    Continues,
    FollowUpTo,
    Supersedes,
    RelatedTo,
    DerivedFrom,
    RegressionOf,
    Closed,
}

impl GlobalWorkStore {
    pub fn open(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn tasks_path(&self) -> PathBuf {
        self.root.join("tasks.jsonl")
    }

    pub fn streams_path(&self) -> PathBuf {
        self.root.join("streams.jsonl")
    }

    pub fn stream_events_path(&self) -> PathBuf {
        self.root.join("stream-events.jsonl")
    }

    pub fn append_task(
        &self,
        project_root: impl AsRef<Path>,
        task: &Task,
    ) -> crate::Result<PathBuf> {
        self.append_task_in_stream(project_root, task, None)
    }

    pub fn append_task_in_stream(
        &self,
        project_root: impl AsRef<Path>,
        task: &Task,
        stream_id: Option<&str>,
    ) -> crate::Result<PathBuf> {
        let record = ProjectScopedTask {
            project_root: normalize_project_root(project_root),
            stream_id: stream_id.map(str::to_string),
            task: task.clone(),
        };
        append_jsonl(&self.tasks_path(), &record)?;
        Ok(self.tasks_path())
    }

    pub fn load_tasks(&self) -> crate::Result<Vec<ProjectScopedTask>> {
        read_jsonl(&self.tasks_path())
    }

    pub fn tasks_for_project(&self, project_root: impl AsRef<Path>) -> crate::Result<Vec<Task>> {
        let project_root = normalize_project_root(project_root);
        Ok(self
            .load_tasks()?
            .into_iter()
            .filter(|record| record.project_root == project_root)
            .map(|record| record.task)
            .collect())
    }

    pub fn tasks_for_stream(
        &self,
        project_root: impl AsRef<Path>,
        stream_id: &str,
    ) -> crate::Result<Vec<Task>> {
        let project_root = normalize_project_root(project_root);
        Ok(self
            .load_tasks()?
            .into_iter()
            .filter(|record| {
                record.project_root == project_root
                    && record.stream_id.as_deref() == Some(stream_id)
            })
            .map(|record| record.task)
            .collect())
    }

    pub fn find_task(&self, id: &WorkId) -> crate::Result<Option<ProjectScopedTask>> {
        Ok(self
            .load_tasks()?
            .into_iter()
            .find(|record| &record.task.id == id))
    }

    pub fn append_stream(&self, stream: &ProjectWorkStream) -> crate::Result<PathBuf> {
        let mut stream = stream.clone();
        stream.project_root = normalize_project_root(&stream.project_root);
        append_jsonl(&self.streams_path(), &stream)?;
        Ok(self.streams_path())
    }

    pub fn load_streams(&self) -> crate::Result<Vec<ProjectWorkStream>> {
        read_jsonl(&self.streams_path())
    }

    pub fn streams_for_project(
        &self,
        project_root: impl AsRef<Path>,
    ) -> crate::Result<Vec<ProjectWorkStream>> {
        let project_root = normalize_project_root(project_root);
        Ok(self
            .load_streams()?
            .into_iter()
            .filter(|stream| stream.project_root == project_root)
            .collect())
    }

    pub fn append_stream_event(&self, event: &StreamEvent) -> crate::Result<PathBuf> {
        let mut event = event.clone();
        event.project_root = normalize_project_root(&event.project_root);
        append_jsonl(&self.stream_events_path(), &event)?;
        Ok(self.stream_events_path())
    }

    pub fn stream_events(
        &self,
        project_root: impl AsRef<Path>,
        stream_id: &str,
    ) -> crate::Result<Vec<StreamEvent>> {
        let project_root = normalize_project_root(project_root);
        Ok(read_jsonl::<StreamEvent>(&self.stream_events_path())?
            .into_iter()
            .filter(|event| event.project_root == project_root && event.stream_id == stream_id)
            .collect())
    }
}

fn append_jsonl<T: Serialize>(path: &Path, value: &T) -> crate::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(value)?;
    line.push('\n');
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(line.as_bytes())?;
    Ok(())
}

fn read_jsonl<T: for<'de> Deserialize<'de>>(path: &Path) -> crate::Result<Vec<T>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)?;
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| Ok(serde_json::from_str(line)?))
        .collect()
}

fn normalize_project_root(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Task, TaskStatus};
    use tempfile::tempdir;

    fn task(id: &str, title: &str) -> Task {
        let mut task = Task::new(title);
        task.id = WorkId::from(id);
        task.status = TaskStatus::Ready;
        task
    }

    #[test]
    fn global_store_keeps_two_project_roots_in_one_task_file() {
        let temp = tempdir().unwrap();
        let global = GlobalWorkStore::open(temp.path().join("global-work"));
        let project_a = temp.path().join("project-a");
        let project_b = temp.path().join("project-b");
        fs::create_dir_all(&project_a).unwrap();
        fs::create_dir_all(&project_b).unwrap();

        global
            .append_task(&project_a, &task("T-a", "A task"))
            .unwrap();
        global
            .append_task(&project_b, &task("T-b", "B task"))
            .unwrap();

        let all = global.load_tasks().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(global.tasks_for_project(&project_a).unwrap().len(), 1);
        assert_eq!(global.tasks_for_project(&project_b).unwrap().len(), 1);
        assert_eq!(
            global.tasks_for_project(&project_a).unwrap()[0].id,
            WorkId::from("T-a")
        );
    }

    #[test]
    fn global_store_finds_task_with_project_scope() {
        let temp = tempdir().unwrap();
        let global = GlobalWorkStore::open(temp.path().join("global-work"));
        let project = temp.path().join("project");
        fs::create_dir_all(&project).unwrap();

        global
            .append_task(&project, &task("T-find", "Find me"))
            .unwrap();

        let found = global.find_task(&WorkId::from("T-find")).unwrap().unwrap();
        assert_eq!(found.project_root, project.canonicalize().unwrap());
        assert_eq!(found.task.title, "Find me");
    }

    #[test]
    fn project_work_stream_preserves_context_across_closed_and_follow_up_tasks() {
        let temp = tempdir().unwrap();
        let global = GlobalWorkStore::open(temp.path().join("global-work"));
        let project = temp.path().join("project");
        fs::create_dir_all(&project).unwrap();
        let stream = ProjectWorkStream {
            project_root: project.clone(),
            stream_id: "imp-work-migration".to_string(),
            title: "imp-work migration".to_string(),
            summary: "Replace mana while preserving project context.".to_string(),
        };
        global.append_stream(&stream).unwrap();

        let mut closed = task("T-closed", "Closed epic");
        closed.status = TaskStatus::Done;
        let follow_up = task("T-follow", "Follow-up task");
        global
            .append_task_in_stream(&project, &closed, Some("imp-work-migration"))
            .unwrap();
        global
            .append_task_in_stream(&project, &follow_up, Some("imp-work-migration"))
            .unwrap();
        global
            .append_stream_event(&StreamEvent {
                project_root: project.clone(),
                stream_id: "imp-work-migration".to_string(),
                work_id: Some(closed.id.clone()),
                relation: StreamRelation::Closed,
                summary: "Closed first migration epic with durable findings.".to_string(),
            })
            .unwrap();
        global
            .append_stream_event(&StreamEvent {
                project_root: project.clone(),
                stream_id: "imp-work-migration".to_string(),
                work_id: Some(follow_up.id.clone()),
                relation: StreamRelation::Continues,
                summary: "Continue from prior migration findings.".to_string(),
            })
            .unwrap();

        let tasks = global
            .tasks_for_stream(&project, "imp-work-migration")
            .unwrap();
        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().any(|task| task.status == TaskStatus::Done));
        assert!(tasks.iter().any(|task| task.id == WorkId::from("T-follow")));
        let events = global
            .stream_events(&project, "imp-work-migration")
            .unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].relation, StreamRelation::Continues);
        assert!(events[1].summary.contains("prior migration findings"));
    }
}
