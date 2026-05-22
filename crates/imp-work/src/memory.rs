use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::{MemoryItem, MemoryKind, SourceKind, SourceRef, WorkId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationMemoryInput {
    pub text: String,
    pub kind: Option<MemoryKind>,
    pub parent_work: Option<WorkId>,
    pub topics: Vec<String>,
    pub paths: Vec<PathBuf>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationMemoryQuery {
    pub text: Option<String>,
    pub topic: Option<String>,
    pub parent_work: Option<WorkId>,
    pub path: Option<PathBuf>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationMemoryMatch {
    pub memory: MemoryItem,
    pub score: i64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ConversationMemoryIndex {
    items: Vec<MemoryItem>,
}

impl ConversationMemoryIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_items(items: Vec<MemoryItem>) -> Self {
        Self { items }
    }

    pub fn capture(&mut self, input: ConversationMemoryInput) -> MemoryItem {
        let item = capture_conversation_memory(input);
        self.items.push(item.clone());
        item
    }

    pub fn all_items(&self) -> &[MemoryItem] {
        &self.items
    }

    pub fn recent(&self, limit: usize) -> Vec<MemoryItem> {
        let mut items = self
            .items
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        items.reverse();
        items
    }

    pub fn retrieve(&self, query: ConversationMemoryQuery) -> Vec<ConversationMemoryMatch> {
        let mut matches = self
            .items
            .iter()
            .filter_map(|item| score_memory(item, &query))
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.memory.text.cmp(&right.memory.text))
        });
        matches.truncate(query.limit.max(1));
        matches
    }
}

pub fn capture_conversation_memory(input: ConversationMemoryInput) -> MemoryItem {
    let kind = input
        .kind
        .unwrap_or_else(|| classify_memory_kind(&input.text));
    let mut memory = MemoryItem::new(kind, input.text);
    memory.parent_work = input.parent_work;
    memory.topics = if input.topics.is_empty() {
        infer_topics(&memory.text)
    } else {
        normalize_topics(input.topics)
    };
    memory.paths = input.paths;
    memory.source_refs.push(SourceRef {
        kind: SourceKind::Conversation,
        reference: input
            .source
            .unwrap_or_else(|| "conversation:current".into()),
        fingerprint: None,
    });
    memory
}

pub fn classify_memory_kind(text: &str) -> MemoryKind {
    let lower = text.to_ascii_lowercase();
    if contains_any(
        &lower,
        &["we decided", "decision:", "accepted direction", "should be"],
    ) {
        MemoryKind::Decision
    } else if contains_any(
        &lower,
        &[
            "prefer",
            "preference",
            "i like",
            "i don't want",
            "do not want",
        ],
    ) {
        MemoryKind::Preference
    } else if contains_any(
        &lower,
        &["todo", "follow up", "follow-up", "next step", "we need to"],
    ) {
        MemoryKind::FollowUp
    } else if contains_any(&lower, &["prototype", "evidence", "learned", "learning"])
        && contains_any(&lower, &["durable", "temporary", "supported", "refuted"])
    {
        MemoryKind::PrototypeLearning
    } else if contains_any(&lower, &["is ", "are ", "currently", "shipped", "exists"]) {
        MemoryKind::Fact
    } else {
        MemoryKind::Note
    }
}

fn score_memory(
    item: &MemoryItem,
    query: &ConversationMemoryQuery,
) -> Option<ConversationMemoryMatch> {
    let mut score = 0;
    let mut reasons = Vec::new();

    if let Some(parent_work) = &query.parent_work {
        if item.parent_work.as_ref() == Some(parent_work) {
            score += 50;
            reasons.push("parent_work".into());
        }
    }

    if let Some(topic) = &query.topic {
        let needle = normalize_topic(topic);
        if item
            .topics
            .iter()
            .any(|topic| normalize_topic(topic) == needle)
        {
            score += 35;
            reasons.push("topic".into());
        }
    }

    if let Some(path) = &query.path {
        if item
            .paths
            .iter()
            .any(|item_path| path_matches(item_path, path))
        {
            score += 35;
            reasons.push("path".into());
        }
    }

    if let Some(text) = &query.text {
        let terms = query_terms(text);
        if !terms.is_empty() {
            let haystack = searchable_text(item);
            let matched_terms = terms
                .iter()
                .filter(|term| haystack.contains(term.as_str()))
                .count();
            if matched_terms > 0 {
                score += (matched_terms as i64) * 10;
                reasons.push(format!("text:{matched_terms}"));
            }
        }
    }

    if score == 0
        && query.text.is_none()
        && query.topic.is_none()
        && query.parent_work.is_none()
        && query.path.is_none()
    {
        score = 1;
        reasons.push("recent".into());
    }

    (score > 0).then(|| ConversationMemoryMatch {
        memory: item.clone(),
        score,
        reasons,
    })
}

fn searchable_text(item: &MemoryItem) -> String {
    let mut text = item.text.to_ascii_lowercase();
    for topic in &item.topics {
        text.push(' ');
        text.push_str(&topic.to_ascii_lowercase());
    }
    if let Some(parent_work) = &item.parent_work {
        text.push(' ');
        text.push_str(&parent_work.0.to_ascii_lowercase());
    }
    text
}

fn query_terms(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_alphanumeric() && ch != '-' && ch != '_')
        .map(str::trim)
        .filter(|term| term.len() > 2)
        .map(str::to_ascii_lowercase)
        .collect()
}

fn infer_topics(text: &str) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let mut topics = Vec::new();
    for topic in [
        "imp-work",
        "prototype",
        "memory",
        "context-pack",
        "subagent",
        "cache",
        "mana",
        "scheduler",
        "task",
    ] {
        if lower.contains(topic) || lower.contains(&topic.replace('-', " ")) {
            topics.push(topic.to_string());
        }
    }
    topics
}

fn normalize_topics(topics: Vec<String>) -> Vec<String> {
    let mut topics = topics
        .into_iter()
        .map(|topic| normalize_topic(&topic))
        .filter(|topic| !topic.is_empty())
        .collect::<Vec<_>>();
    topics.sort();
    topics.dedup();
    topics
}

fn normalize_topic(topic: &str) -> String {
    topic.trim().to_ascii_lowercase().replace(' ', "-")
}

fn path_matches(item_path: &std::path::Path, query_path: &std::path::Path) -> bool {
    item_path == query_path
        || item_path.starts_with(query_path)
        || query_path.starts_with(item_path)
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversational_memory_classifies_preferences_and_decisions() {
        assert_eq!(
            classify_memory_kind("I don't want mana import code in the repo"),
            MemoryKind::Preference
        );
        assert_eq!(
            classify_memory_kind("We decided imp-work should own task context packs"),
            MemoryKind::Decision
        );
    }

    #[test]
    fn conversational_memory_capture_links_source_topics_and_parent() {
        let memory = capture_conversation_memory(ConversationMemoryInput {
            text: "Prototype learning is durable even when code is temporary".into(),
            kind: None,
            parent_work: Some(WorkId::from("P-prototype")),
            topics: vec![],
            paths: vec![PathBuf::from("crates/imp-work/src/prototype.rs")],
            source: Some("conversation:abc".into()),
        });

        assert_eq!(memory.kind, MemoryKind::PrototypeLearning);
        assert_eq!(
            memory.parent_work.as_ref().map(|id| id.0.as_str()),
            Some("P-prototype")
        );
        assert!(memory.topics.contains(&"prototype".into()));
        assert_eq!(memory.source_refs[0].reference, "conversation:abc");
    }

    #[test]
    fn retrieval_scores_by_topic_path_parent_and_text() {
        let mut index = ConversationMemoryIndex::new();
        index.capture(ConversationMemoryInput {
            text: "Task context packs should exclude dynamic lease ids from cacheable blocks"
                .into(),
            kind: None,
            parent_work: Some(WorkId::from("T-context")),
            topics: vec!["context-pack".into(), "cache".into()],
            paths: vec![PathBuf::from("crates/imp-work/src/context_pack.rs")],
            source: None,
        });
        index.capture(ConversationMemoryInput {
            text: "Prototype code can be deleted after learnings are captured".into(),
            kind: None,
            parent_work: Some(WorkId::from("P-prototype")),
            topics: vec!["prototype".into()],
            paths: vec![PathBuf::from("crates/imp-work/src/prototype.rs")],
            source: None,
        });

        let matches = index.retrieve(ConversationMemoryQuery {
            text: Some("lease cache".into()),
            topic: Some("context pack".into()),
            parent_work: Some(WorkId::from("T-context")),
            path: Some(PathBuf::from("crates/imp-work/src/context_pack.rs")),
            limit: 5,
        });

        assert_eq!(matches.len(), 1);
        assert!(matches[0].score >= 100);
        assert!(matches[0]
            .reasons
            .iter()
            .any(|reason| reason == "parent_work"));
    }

    #[test]
    fn recent_returns_last_items_in_original_order() {
        let mut index = ConversationMemoryIndex::new();
        index.capture(ConversationMemoryInput {
            text: "first memory".into(),
            kind: Some(MemoryKind::Note),
            parent_work: None,
            topics: vec![],
            paths: vec![],
            source: None,
        });
        index.capture(ConversationMemoryInput {
            text: "second memory".into(),
            kind: Some(MemoryKind::Note),
            parent_work: None,
            topics: vec![],
            paths: vec![],
            source: None,
        });

        let recent = index.recent(1);
        assert_eq!(recent[0].text, "second memory");
    }
}
