use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::system_prompt::Fact;

const MAX_RELEVANT_FACTS: usize = 8;
const MAX_FACT_TEXT_CHARS: usize = 160;

/// Session-start mana-backed prompt context owned by imp runtime assembly.
///
/// This currently injects only bounded project facts. A future follow-up can add
/// a compact textual project-memory block for warnings / working-on / recent work
/// without changing builder call sites again.
#[derive(Debug, Default)]
pub(crate) struct SessionPromptContext {
    pub facts: Vec<Fact>,
}

pub(crate) fn load_session_prompt_context(cwd: &Path) -> SessionPromptContext {
    let Some(mana_dir) = nearest_mana_dir(cwd) else {
        return SessionPromptContext::default();
    };

    let Ok(memory) = mana_core::ops::memory_context::memory_context(&mana_dir) else {
        return SessionPromptContext::default();
    };

    SessionPromptContext {
        facts: map_relevant_facts(memory),
    }
}

pub(crate) fn nearest_mana_dir(cwd: &Path) -> Option<PathBuf> {
    mana_core::discovery::find_mana_dir(cwd).ok()
}

fn load_session_prompt_context_from_mana_dir(mana_dir: &Path) -> SessionPromptContext {
    let Ok(memory) = mana_core::ops::memory_context::memory_context(mana_dir) else {
        return SessionPromptContext::default();
    };

    SessionPromptContext {
        facts: map_relevant_facts(memory),
    }
}

fn map_relevant_facts(memory: mana_core::ops::memory_context::MemoryContext) -> Vec<Fact> {
    memory
        .relevant_facts
        .into_iter()
        .take(MAX_RELEVANT_FACTS)
        .map(|relevant| Fact {
            text: truncate_for_prompt(&relevant.unit.title, MAX_FACT_TEXT_CHARS),
            verified_ago: format_verified_ago(relevant.unit.last_verified),
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
    }

    #[test]
    fn invalid_mana_dir_load_yields_empty_prompt_context() {
        let dir = TempDir::new().unwrap();
        let mana_dir = dir.path().join(".mana");
        std::fs::create_dir(&mana_dir).unwrap();

        let context = load_session_prompt_context(dir.path());
        assert!(context.facts.is_empty());
    }

    #[test]
    fn maps_memory_context_to_bounded_prompt_facts() {
        let mut recent = Unit::new("1", "Recent verified fact");
        recent.last_verified = Some(Utc::now() - Duration::hours(2));

        let mut stale = Unit::new(
            "2",
            "A very long fact title that should be truncated before it reaches the prompt because prompt context should stay bounded and selective for interactive startup and this extra suffix pushes it over the limit",
        );
        stale.last_verified = None;

        let facts = map_relevant_facts(MemoryContext {
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
        });

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

        let context = load_session_prompt_context_from_mana_dir(&mana_dir);
        assert_eq!(context.facts.len(), 1);
        assert_eq!(context.facts[0].text, "Auth uses RS256 signing");
        assert_eq!(context.facts[0].verified_ago, "30m ago");
    }

    #[test]
    fn caps_fact_count_for_prompt_budget() {
        let relevant_facts = (0..12)
            .map(|idx| {
                let mut unit = Unit::new(format!("{}", idx + 1), format!("Fact {idx}"));
                unit.last_verified = Some(Utc::now() - Duration::minutes(idx.into()));
                mana_core::ops::memory_context::RelevantFact { unit, score: 100 - idx }
            })
            .collect();

        let facts = map_relevant_facts(MemoryContext {
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
