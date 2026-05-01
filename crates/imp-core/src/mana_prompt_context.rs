use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::system_prompt::Fact;

const MAX_RELEVANT_FACTS: usize = 8;
const MAX_FACT_TEXT_CHARS: usize = 160;
const MAX_STATUS_WARNINGS: usize = 3;
const MAX_STATUS_WORKING_ON: usize = 3;
const MAX_STATUS_RECENT_WORK: usize = 3;
const MAX_STATUS_TITLE_CHARS: usize = 80;
const MAX_WARNING_TEXT_CHARS: usize = 120;

/// Session-start mana-backed prompt context owned by imp runtime assembly.
///
/// Facts remain a distinct verified-fact seam. Dynamic status-like project
/// memory is carried separately as a compact optional text block.
#[derive(Debug, Default)]
pub struct SessionPromptContext {
    pub facts: Vec<Fact>,
    pub project_memory_status: Option<String>,
}

pub fn load_session_prompt_context(cwd: &Path) -> SessionPromptContext {
    let Some(mana_dir) = nearest_mana_dir(cwd) else {
        return SessionPromptContext::default();
    };

    load_session_prompt_context_from_mana_dir(&mana_dir).unwrap_or_default()
}

pub fn load_task_prompt_context(mana_dir: &Path, task_paths: &[String]) -> SessionPromptContext {
    load_task_prompt_context_from_mana_dir(mana_dir, task_paths).unwrap_or_default()
}

pub fn nearest_mana_dir(cwd: &Path) -> Option<PathBuf> {
    mana_core::api::find_mana_dir(cwd).ok()
}

fn load_session_prompt_context_from_mana_dir(
    mana_dir: &Path,
) -> Result<SessionPromptContext, String> {
    let memory = mana_core::api::memory_context(mana_dir).map_err(|err| err.to_string())?;

    Ok(SessionPromptContext {
        facts: map_relevant_facts(&memory),
        project_memory_status: format_project_memory_status(&memory),
    })
}

fn load_task_prompt_context_from_mana_dir(
    mana_dir: &Path,
    task_paths: &[String],
) -> Result<SessionPromptContext, String> {
    let memory = mana_core::api::memory_context(mana_dir).map_err(|err| err.to_string())?;

    Ok(SessionPromptContext {
        facts: map_task_relevant_facts(&memory, task_paths),
        project_memory_status: None,
    })
}

fn map_relevant_facts(memory: &mana_core::api::MemoryContext) -> Vec<Fact> {
    memory
        .relevant_facts
        .iter()
        .take(MAX_RELEVANT_FACTS)
        .map(|relevant| Fact {
            text: truncate_for_prompt(&relevant.unit.title, MAX_FACT_TEXT_CHARS),
            verified_ago: format_verified_ago(relevant.unit.last_verified),
        })
        .collect()
}

fn map_task_relevant_facts(
    memory: &mana_core::api::MemoryContext,
    task_paths: &[String],
) -> Vec<Fact> {
    let mut relevant: Vec<_> = memory.relevant_facts.iter().collect();
    if !task_paths.is_empty() {
        relevant.retain(|fact| {
            fact.unit.paths.iter().any(|fact_path| {
                task_paths
                    .iter()
                    .any(|task_path| path_overlap(fact_path, task_path))
            })
        });
    }

    relevant
        .into_iter()
        .take(MAX_RELEVANT_FACTS)
        .map(|relevant| Fact {
            text: truncate_for_prompt(&relevant.unit.title, MAX_FACT_TEXT_CHARS),
            verified_ago: format_verified_ago(relevant.unit.last_verified),
        })
        .collect()
}

fn path_overlap(a: &str, b: &str) -> bool {
    a.starts_with(b) || b.starts_with(a) || a == b
}

fn format_project_memory_status(memory: &mana_core::api::MemoryContext) -> Option<String> {
    let warnings = format_warning_lines(memory);
    let working_on = format_working_on_lines(memory);
    let recent_work = format_recent_work_lines(memory);

    if warnings.is_empty() && working_on.is_empty() && recent_work.is_empty() {
        return None;
    }

    let mut sections = Vec::new();

    if !warnings.is_empty() {
        sections.push(format!("Warnings:\n{}", warnings.join("\n")));
    }

    if !working_on.is_empty() {
        sections.push(format!("Working on:\n{}", working_on.join("\n")));
    }

    if !recent_work.is_empty() {
        sections.push(format!("Recent work:\n{}", recent_work.join("\n")));
    }

    Some(format!("Project memory status:\n{}", sections.join("\n\n")))
}

fn format_warning_lines(memory: &mana_core::api::MemoryContext) -> Vec<String> {
    memory
        .warnings
        .iter()
        .take(MAX_STATUS_WARNINGS)
        .map(|warning| format!("- {}", truncate_for_prompt(warning, MAX_WARNING_TEXT_CHARS)))
        .collect()
}

fn format_working_on_lines(memory: &mana_core::api::MemoryContext) -> Vec<String> {
    memory
        .working_on
        .iter()
        .take(MAX_STATUS_WORKING_ON)
        .map(|working| {
            let mut parts = vec![format!(
                "[{}] {}",
                working.unit.id,
                truncate_for_prompt(&working.unit.title, MAX_STATUS_TITLE_CHARS)
            )];

            if working.failed_attempts > 0 {
                parts.push(format!("{} failed attempt(s)", working.failed_attempts));
            }

            if let Some(claimed_by) = working.unit.claimed_by.as_deref() {
                parts.push(format!("claimed by {}", claimed_by));
            }

            format!("- {}", parts.join(" — "))
        })
        .collect()
}

fn format_recent_work_lines(memory: &mana_core::api::MemoryContext) -> Vec<String> {
    memory
        .recent_work
        .iter()
        .take(MAX_STATUS_RECENT_WORK)
        .map(|recent| {
            let closed = recent
                .unit
                .closed_at
                .map(|closed_at| format_verified_ago(Some(closed_at)))
                .unwrap_or_else(|| "recently".to_string());

            format!(
                "- [{}] {} — closed {}",
                recent.unit.id,
                truncate_for_prompt(&recent.unit.title, MAX_STATUS_TITLE_CHARS),
                closed
            )
        })
        .collect()
}

fn truncate_for_prompt(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}…", truncated.trim_end())
    } else {
        text.to_string()
    }
}

fn format_verified_ago(last_verified: Option<DateTime<Utc>>) -> String {
    let Some(last_verified) = last_verified else {
        return "unverified".to_string();
    };

    let ago = Utc::now() - last_verified;
    if ago.num_days() > 0 {
        format!("{}d ago", ago.num_days())
    } else if ago.num_hours() > 0 {
        format!("{}h ago", ago.num_hours())
    } else if ago.num_minutes() > 0 {
        format!("{}m ago", ago.num_minutes())
    } else {
        "just now".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use mana_core::config::Config;
    use mana_core::ops::memory_context::MemoryContext;
    use mana_core::unit::{Status, Unit};
    use tempfile::TempDir;

    fn setup_mana_dir() -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().unwrap();
        let mana_dir = dir.path().join(".mana");
        std::fs::create_dir(&mana_dir).unwrap();

        let mut config = Config::default();
        config.project = "test".to_string();
        config.save(&mana_dir).unwrap();

        (dir, mana_dir)
    }

    fn write_unit(mana_dir: &Path, unit: &Unit) {
        let slug = mana_core::util::title_to_slug(&unit.title);
        unit.to_file(mana_dir.join(format!("{}-{}.md", unit.id, slug)))
            .unwrap();
    }

    #[test]
    fn finds_nearest_mana_dir_from_nested_cwd() {
        let (dir, mana_dir) = setup_mana_dir();
        let nested = dir.path().join("project/src/module");
        std::fs::create_dir_all(&nested).unwrap();

        assert_eq!(nearest_mana_dir(&nested), Some(mana_dir));
    }

    #[test]
    fn missing_mana_dir_yields_empty_prompt_context() {
        let dir = TempDir::new().unwrap();
        let context = load_session_prompt_context(dir.path());
        assert!(context.facts.is_empty());
        assert!(context.project_memory_status.is_none());
    }

    #[test]
    fn invalid_mana_dir_load_yields_empty_prompt_context() {
        let dir = TempDir::new().unwrap();
        let mana_dir = dir.path().join(".mana");
        std::fs::create_dir(&mana_dir).unwrap();

        let context = load_session_prompt_context(dir.path());
        assert!(context.facts.is_empty());
        assert!(context.project_memory_status.is_none());
    }

    #[test]
    fn maps_memory_context_to_bounded_prompt_facts() {
        let mut recent = Unit::new("1", "Recent verified fact");
        recent.last_verified = Some(Utc::now() - Duration::hours(2));

        let mut stale = Unit::new(
            "2",
            "A very long fact title that should be truncated before it reaches the prompt because prompt context should stay bounded and selective for interactive startup and this suffix forces truncation",
        );
        stale.last_verified = None;

        let memory = MemoryContext {
            warnings: vec!["warn".into()],
            working_on: vec![],
            relevant_facts: vec![
                mana_core::ops::memory_context::RelevantFact {
                    unit: recent,
                    score: 10,
                },
                mana_core::ops::memory_context::RelevantFact {
                    unit: stale,
                    score: 9,
                },
            ],
            recent_work: vec![],
        };

        let facts = map_relevant_facts(&memory);

        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].text, "Recent verified fact");
        assert_eq!(facts[0].verified_ago, "2h ago");
        assert!(facts[1].text.ends_with('…'));
        assert_eq!(facts[1].verified_ago, "unverified");
    }

    #[test]
    fn loads_relevant_facts_from_mana_memory_context() {
        let (_dir, mana_dir) = setup_mana_dir();

        let mut working = Unit::new("1", "Implement auth flow");
        working.status = Status::InProgress;
        working.paths = vec!["src/auth.rs".to_string()];
        working.requires = vec!["AuthProvider".to_string()];
        write_unit(&mana_dir, &working);

        let mut fact = Unit::new("2", "Auth uses RS256 signing");
        fact.unit_type = "fact".to_string();
        fact.paths = vec!["src/auth.rs".to_string()];
        fact.produces = vec!["AuthProvider".to_string()];
        fact.last_verified = Some(Utc::now() - Duration::minutes(30));
        write_unit(&mana_dir, &fact);

        let context = load_session_prompt_context_from_mana_dir(&mana_dir).unwrap();
        assert_eq!(context.facts.len(), 1);
        assert_eq!(context.facts[0].text, "Auth uses RS256 signing");
        assert_eq!(context.facts[0].verified_ago, "30m ago");
        assert!(context.project_memory_status.is_some());
        let status = context.project_memory_status.as_deref().unwrap();
        assert!(status.contains("Project memory status:"));
        assert!(status.contains("Working on:"));
        assert!(status.contains("[1] Implement auth flow"));
        assert!(!status.contains("Auth uses RS256 signing"));
    }

    #[test]
    fn loads_task_specific_relevant_facts_from_context_paths() {
        let (_dir, mana_dir) = setup_mana_dir();

        let mut fact_auth = Unit::new("2", "Auth uses RS256 signing");
        fact_auth.unit_type = "fact".to_string();
        fact_auth.paths = vec!["src/auth.rs".to_string()];
        fact_auth.last_verified = Some(Utc::now() - Duration::minutes(30));
        write_unit(&mana_dir, &fact_auth);

        let mut fact_cache = Unit::new("3", "Cache keys must include tenant id");
        fact_cache.unit_type = "fact".to_string();
        fact_cache.paths = vec!["src/cache.rs".to_string()];
        fact_cache.last_verified = Some(Utc::now() - Duration::minutes(45));
        write_unit(&mana_dir, &fact_cache);

        let context = load_task_prompt_context(
            &mana_dir,
            &["src/auth.rs".to_string(), "tests/auth.rs".to_string()],
        );
        assert_eq!(context.facts.len(), 1);
        assert_eq!(context.facts[0].text, "Auth uses RS256 signing");
    }

    #[test]
    fn formats_compact_project_memory_status_block() {
        let mut working = Unit::new(
            "1",
            "A very long working unit title that should be truncated before it reaches the prompt because startup context should stay compact and preview oriented",
        );
        working.status = Status::InProgress;
        working.claimed_by = Some("imp".into());

        let mut recent = Unit::new("9", "Recently closed cleanup task");
        recent.closed_at = Some(Utc::now() - Duration::hours(3));

        let status = format_project_memory_status(&MemoryContext {
            warnings: vec![
                "STALE: \"Old fact\" — not verified in 5d".into(),
                "PAST FAILURE [1]: \"retry with narrower verify\"".into(),
                "warn three".into(),
                "warn four should be omitted".into(),
            ],
            working_on: vec![mana_core::ops::memory_context::WorkingUnit {
                unit: working,
                failed_attempts: 2,
                last_failure_notes: Some("narrow verify first".into()),
            }],
            relevant_facts: vec![],
            recent_work: vec![mana_core::ops::memory_context::RecentWork { unit: recent }],
        })
        .unwrap();

        assert!(status.contains("Project memory status:"));
        assert!(status.contains("Warnings:"));
        assert!(status.contains("Working on:"));
        assert!(status.contains("Recent work:"));
        assert!(status.contains("warn three"));
        assert!(!status.contains("warn four should be omitted"));
        assert!(status.contains("[1]"));
        assert!(status.contains("2 failed attempt(s)"));
        assert!(status.contains("claimed by imp"));
        assert!(status.contains("[9] Recently closed cleanup task — closed 3h ago"));
        assert!(status.contains('…'));
    }

    #[test]
    fn caps_fact_count_for_prompt_budget() {
        let relevant_facts = (0..12)
            .map(|idx| {
                let mut unit = Unit::new(format!("{}", idx + 1), format!("Fact {idx}"));
                unit.last_verified = Some(Utc::now() - Duration::minutes(idx.into()));
                mana_core::ops::memory_context::RelevantFact {
                    unit,
                    score: 100 - idx,
                }
            })
            .collect();

        let facts = map_relevant_facts(&MemoryContext {
            warnings: vec![],
            working_on: vec![],
            relevant_facts,
            recent_work: vec![],
        });

        assert_eq!(facts.len(), MAX_RELEVANT_FACTS);
        assert_eq!(facts[0].text, "Fact 0");
        assert_eq!(facts[MAX_RELEVANT_FACTS - 1].text, "Fact 7");
    }
}
