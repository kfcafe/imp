use serde::{Deserialize, Serialize};

const FINISHED_RUN_TTL_MS: u128 = 24 * 60 * 60 * 1000;
const INTERRUPTED_RUN_STALE_MS: u128 = 6 * 60 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManaRunAgentSummary {
    pub unit_id: String,
    pub title: String,
    pub action: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManaRunSummary {
    pub run_id: String,
    pub scope: String,
    pub status: String,
    pub total_units: usize,
    pub total_closed: usize,
    pub total_failed: usize,
    pub total_awaiting_verify: usize,
    pub latest: Option<String>,
    pub logs: Vec<String>,
    pub agents: Vec<ManaRunAgentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedRunStore {
    #[allow(dead_code)]
    next_id: u64,
    #[serde(default)]
    runs: Vec<PersistedRunState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedRunState {
    run_id: String,
    scope: String,
    status: String,
    error: Option<String>,
    finished_at_ms: Option<u128>,
    #[serde(default)]
    last_event_at_ms: u128,
    summary: PersistedRunSummary,
    #[serde(default)]
    log_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedRunSummary {
    #[serde(default)]
    total_units: usize,
    #[serde(default)]
    total_closed: usize,
    #[serde(default)]
    total_failed: usize,
    #[serde(default)]
    total_awaiting_verify: usize,
}

pub fn mana_run_summary(run_id: &str) -> Result<Option<ManaRunSummary>, String> {
    let store = load_run_store()?;
    Ok(store
        .runs
        .into_iter()
        .find(|run| run.run_id == run_id)
        .map(PersistedRunState::into_summary))
}

pub fn stop_mana_run(run_id: &str) -> Result<Option<ManaRunSummary>, String> {
    let mut store = load_run_store()?;
    let Some(run) = store.runs.iter_mut().find(|run| run.run_id == run_id) else {
        return Ok(None);
    };

    let now = unix_time_ms();
    if run.finished_at_ms.is_none() {
        run.status = "interrupted".to_string();
        run.error =
            Some("Stopped from imp /stop; external workers may need manual cleanup".to_string());
        run.finished_at_ms = Some(now);
        run.last_event_at_ms = now;
        run.log_lines.push("Run stopped from imp /stop".to_string());
        save_run_store(&store)?;
    }

    Ok(load_run_store()?
        .runs
        .into_iter()
        .find(|run| run.run_id == run_id)
        .map(PersistedRunState::into_summary))
}

fn load_run_store() -> Result<PersistedRunStore, String> {
    let path = run_state_file();
    if !path.exists() {
        return Ok(PersistedRunStore::default());
    }
    let contents =
        std::fs::read_to_string(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
    if contents.trim().is_empty() {
        return Ok(PersistedRunStore::default());
    }
    let mut store: PersistedRunStore = serde_json::from_str(&contents)
        .map_err(|err| format!("parse {}: {err}", path.display()))?;
    classify_stale_unfinished_runs(&mut store);
    Ok(store)
}

fn save_run_store(store: &PersistedRunStore) -> Result<(), String> {
    let path = run_state_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(store)
        .map_err(|err| format!("serialize {}: {err}", path.display()))?;
    std::fs::write(&path, json).map_err(|err| format!("write {}: {err}", path.display()))
}

fn classify_stale_unfinished_runs(store: &mut PersistedRunStore) {
    let cutoff = unix_time_ms().saturating_sub(INTERRUPTED_RUN_STALE_MS);
    for run in &mut store.runs {
        if (run.status == "starting" || run.status == "running")
            && run.finished_at_ms.is_none()
            && run.last_event_at_ms > 0
            && run.last_event_at_ms < cutoff
        {
            run.status = "interrupted".to_string();
            run.error = Some(
                "Run state is stale after process restart or lost background worker; inspect logs before rerun"
                    .to_string(),
            );
            run.finished_at_ms = Some(run.last_event_at_ms);
        }
    }

    let cutoff = unix_time_ms().saturating_sub(FINISHED_RUN_TTL_MS);
    store.runs.retain(|run| match run.finished_at_ms {
        Some(finished_at_ms) => finished_at_ms >= cutoff,
        None => true,
    });
}

impl PersistedRunState {
    fn into_summary(self) -> ManaRunSummary {
        let logs = self
            .log_lines
            .into_iter()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();
        let latest = logs.last().cloned();
        let agents = load_agent_summaries().unwrap_or_default();
        ManaRunSummary {
            run_id: self.run_id,
            scope: self.scope,
            status: self.status,
            total_units: self.summary.total_units,
            total_closed: self.summary.total_closed,
            total_failed: self.summary.total_failed,
            total_awaiting_verify: self.summary.total_awaiting_verify,
            latest,
            logs,
            agents,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedAgentEntry {
    title: String,
    action: String,
    #[serde(default)]
    finished_at: Option<i64>,
    #[serde(default)]
    exit_code: Option<i32>,
}

fn load_agent_summaries() -> Result<Vec<ManaRunAgentSummary>, String> {
    let path = agents_file();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents =
        std::fs::read_to_string(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
    if contents.trim().is_empty() {
        return Ok(Vec::new());
    }
    let agents: std::collections::HashMap<String, PersistedAgentEntry> =
        serde_json::from_str(&contents)
            .map_err(|err| format!("parse {}: {err}", path.display()))?;
    let mut summaries = agents
        .into_iter()
        .map(|(unit_id, entry)| {
            let status = agent_status(&entry);
            ManaRunAgentSummary {
                unit_id,
                title: entry.title,
                action: entry.action,
                status,
            }
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|a, b| a.unit_id.cmp(&b.unit_id));
    Ok(summaries)
}

fn agent_status(entry: &PersistedAgentEntry) -> String {
    match (entry.finished_at, entry.exit_code) {
        (None, _) => "running".to_string(),
        (Some(_), Some(0)) => "done".to_string(),
        (Some(_), Some(code)) => format!("failed({code})"),
        (Some(_), None) => "done".to_string(),
    }
}

fn agents_file() -> std::path::PathBuf {
    if let Ok(path) = mana::commands::agents::agents_file_path() {
        return path;
    }
    let dir = std::env::var("HOME")
        .map(|home| {
            std::path::PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("units")
        })
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp").join("mana"));
    std::fs::create_dir_all(&dir).ok();
    dir.join("agents.json")
}

fn run_state_file() -> std::path::PathBuf {
    if let Ok(path) = mana::commands::agents::agents_file_path() {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).ok();
            return dir.join("run_state.json");
        }
    }

    let dir = std::env::var("HOME")
        .map(|home| {
            std::path::PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("units")
        })
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp").join("mana"));
    std::fs::create_dir_all(&dir).ok();
    dir.join("run_state.json")
}

fn unix_time_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
