use imp_llm::{AssistantMessage, Cost, Message, Model, Usage};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::session::{SessionEntry, SessionManager};

/// Session custom entry type used for canonical usage accounting.
pub const USAGE_CUSTOM_TYPE: &str = "usage-record";

/// Current canonical usage record schema version.
pub const USAGE_RECORD_VERSION: u32 = 1;

/// Where a usage report came from when reading session history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UsageRecordSource {
    Canonical,
    LegacyAssistantMessage,
}

/// Stable request identity used for dedupe across copied/forked session history.
///
/// `request_id` is generated once per upstream model request and copied forward
/// with the canonical record. Global summaries should dedupe on this key so the
/// same request preserved in multiple session files is only counted once.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UsageDedupeKey {
    pub request_id: String,
}

/// Raw token accounting captured at request time.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageTokens {
    pub input: u32,
    pub output: u32,
    pub cache_read: u32,
    pub cache_write: u32,
}

impl From<Usage> for UsageTokens {
    fn from(value: Usage) -> Self {
        Self {
            input: value.input_tokens,
            output: value.output_tokens,
            cache_read: value.cache_read_tokens,
            cache_write: value.cache_write_tokens,
        }
    }
}

impl From<&Usage> for UsageTokens {
    fn from(value: &Usage) -> Self {
        Self {
            input: value.input_tokens,
            output: value.output_tokens,
            cache_read: value.cache_read_tokens,
            cache_write: value.cache_write_tokens,
        }
    }
}

impl From<UsageTokens> for Usage {
    fn from(value: UsageTokens) -> Self {
        Self {
            input_tokens: value.input,
            output_tokens: value.output,
            cache_read_tokens: value.cache_read,
            cache_write_tokens: value.cache_write,
        }
    }
}

impl From<&UsageTokens> for Usage {
    fn from(value: &UsageTokens) -> Self {
        Self {
            input_tokens: value.input,
            output_tokens: value.output,
            cache_read_tokens: value.cache_read,
            cache_write_tokens: value.cache_write,
        }
    }
}

/// Stored dollar cost at request time.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UsageCostBreakdown {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}

impl From<Cost> for UsageCostBreakdown {
    fn from(value: Cost) -> Self {
        Self {
            input: value.input,
            output: value.output,
            cache_read: value.cache_read,
            cache_write: value.cache_write,
            total: value.total,
        }
    }
}

impl From<&Cost> for UsageCostBreakdown {
    fn from(value: &Cost) -> Self {
        Self {
            input: value.input,
            output: value.output,
            cache_read: value.cache_read,
            cache_write: value.cache_write,
            total: value.total,
        }
    }
}

impl From<UsageCostBreakdown> for Cost {
    fn from(value: UsageCostBreakdown) -> Self {
        Self {
            input: value.input,
            output: value.output,
            cache_read: value.cache_read,
            cache_write: value.cache_write,
            total: value.total,
        }
    }
}

impl From<&UsageCostBreakdown> for Cost {
    fn from(value: &UsageCostBreakdown) -> Self {
        Self {
            input: value.input,
            output: value.output,
            cache_read: value.cache_read,
            cache_write: value.cache_write,
            total: value.total,
        }
    }
}

/// Canonical usage record stored inside `SessionEntry::Custom`.
///
/// This schema is intentionally small and versioned. It captures stable request
/// identity, attribution for reporting, raw tokens, and stored cost so later
/// reporting doesn't need to recompute historical values from possibly changed
/// model pricing tables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageRecordV1 {
    pub version: u32,
    pub request_id: String,
    pub recorded_at: u64,
    pub provider: String,
    pub model: String,
    pub session_id: Option<String>,
    pub session_path: Option<String>,
    pub assistant_message_id: Option<String>,
    pub turn_index: Option<u32>,
    pub usage: UsageTokens,
    pub cost: UsageCostBreakdown,
    pub source: UsageRecordSource,
}

impl UsageRecordV1 {
    pub fn new(
        request_id: impl Into<String>,
        recorded_at: u64,
        provider: impl Into<String>,
        model: impl Into<String>,
        usage: impl Into<UsageTokens>,
        cost: impl Into<UsageCostBreakdown>,
    ) -> Self {
        Self {
            version: USAGE_RECORD_VERSION,
            request_id: request_id.into(),
            recorded_at,
            provider: provider.into(),
            model: model.into(),
            session_id: None,
            session_path: None,
            assistant_message_id: None,
            turn_index: None,
            usage: usage.into(),
            cost: cost.into(),
            source: UsageRecordSource::Canonical,
        }
    }

    /// Stable dedupe identity for global usage rollups.
    pub fn dedupe_key(&self) -> UsageDedupeKey {
        UsageDedupeKey {
            request_id: self.request_id.clone(),
        }
    }

    pub fn usage_value(&self) -> Usage {
        Usage::from(&self.usage)
    }

    pub fn cost_value(&self) -> Cost {
        Cost::from(&self.cost)
    }

    pub fn with_session_context(
        mut self,
        session_id: Option<String>,
        session_path: Option<String>,
        assistant_message_id: Option<String>,
        turn_index: Option<u32>,
    ) -> Self {
        self.session_id = session_id;
        self.session_path = session_path;
        self.assistant_message_id = assistant_message_id;
        self.turn_index = turn_index;
        self
    }

    pub fn into_custom_data(self) -> Result<serde_json::Value> {
        serde_json::to_value(self).map_err(Into::into)
    }

    pub fn from_custom_data(value: serde_json::Value) -> Result<Self> {
        let record: Self = serde_json::from_value(value)?;
        if record.version != USAGE_RECORD_VERSION {
            return Err(Error::Session(format!(
                "unsupported usage record version: {}",
                record.version
            )));
        }
        Ok(record)
    }
}

/// Usage row returned by read helpers, including provenance and attribution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionUsageRecord {
    pub entry_id: String,
    pub parent_id: Option<String>,
    pub request_id: String,
    pub recorded_at: u64,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub session_id: Option<String>,
    pub session_path: Option<String>,
    pub assistant_message_id: Option<String>,
    pub turn_index: Option<u32>,
    pub usage: UsageTokens,
    pub cost: Option<UsageCostBreakdown>,
    pub source: UsageRecordSource,
}

impl SessionUsageRecord {
    pub fn dedupe_key(&self) -> UsageDedupeKey {
        UsageDedupeKey {
            request_id: self.request_id.clone(),
        }
    }

    pub fn usage_value(&self) -> Usage {
        Usage::from(&self.usage)
    }

    pub fn cost_value(&self) -> Option<Cost> {
        self.cost.as_ref().map(Cost::from)
    }
}

/// Aggregate totals across usage records.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageTotals {
    pub usage: Usage,
    pub cost: Cost,
    pub records: usize,
}

impl UsageTotals {
    pub fn add_record(&mut self, record: &SessionUsageRecord) {
        self.usage.add(&record.usage_value());
        if let Some(cost) = record.cost_value() {
            self.cost.add(&cost);
        }
        self.records += 1;
    }
}

/// Build a canonical usage record for a persisted assistant turn.
///
/// Returns `None` when the assistant message has no usage or when an equivalent
/// canonical record already exists for the same assistant turn.
pub fn canonical_usage_record_for_assistant_turn(
    session: &SessionManager,
    model: &Model,
    assistant_message_id: &str,
    turn_index: u32,
    message: &AssistantMessage,
) -> Option<UsageRecordV1> {
    let usage = message.usage.as_ref()?;
    let request_id = canonical_request_id(assistant_message_id);

    if session.has_canonical_usage_request_id(&request_id)
        || session.has_canonical_usage_for_assistant_message(assistant_message_id)
    {
        return None;
    }

    Some(
        UsageRecordV1::new(
            request_id,
            message.timestamp,
            model.meta.provider.clone(),
            model.meta.id.clone(),
            usage,
            usage.cost(&model.meta.pricing),
        )
        .with_session_context(
            session.session_id(),
            session.path().map(|p| p.display().to_string()),
            Some(assistant_message_id.to_string()),
            Some(turn_index),
        ),
    )
}

/// Read usage rows from a single session entry slice.
///
/// Canonical custom records are preferred. Legacy assistant-message usage is
/// only surfaced when no canonical usage record exists for the same
/// `assistant_message_id`, which preserves backward compatibility while
/// avoiding local double counting once canonical persistence lands.
pub fn usage_records_from_entries(entries: &[SessionEntry]) -> Vec<SessionUsageRecord> {
    let session_id = infer_session_id_from_entries(entries);
    let session_path = infer_session_path_from_entries(entries);

    let mut records = Vec::new();
    let mut canonical_assistant_ids = std::collections::HashSet::new();

    for entry in entries {
        if let Some(record) =
            canonical_usage_record_from_entry(entry, session_id.clone(), session_path.clone())
        {
            if let Some(assistant_message_id) = record.assistant_message_id.clone() {
                canonical_assistant_ids.insert(assistant_message_id);
            }
            records.push(record);
        }
    }

    let mut turn_index = 0u32;
    for entry in entries {
        if let SessionEntry::Message {
            id,
            parent_id,
            message: Message::Assistant(message),
        } = entry
        {
            if let Some(usage) = &message.usage {
                if !canonical_assistant_ids.contains(id) {
                    records.push(SessionUsageRecord {
                        entry_id: id.clone(),
                        parent_id: parent_id.clone(),
                        request_id: legacy_request_id(id),
                        recorded_at: message.timestamp,
                        provider: None,
                        model: None,
                        session_id: session_id.clone(),
                        session_path: session_path.clone(),
                        assistant_message_id: Some(id.clone()),
                        turn_index: Some(turn_index),
                        usage: UsageTokens::from(usage),
                        cost: None,
                        source: UsageRecordSource::LegacyAssistantMessage,
                    });
                }
                turn_index += 1;
            }
        }
    }

    records
}

/// Read usage rows from a session manager, attaching the session path when known.
pub fn usage_records_from_session(session: &SessionManager) -> Vec<SessionUsageRecord> {
    let session_id = session
        .session_id()
        .or_else(|| infer_session_id_from_entries(session.entries()));
    let session_path = session
        .path()
        .map(|p| p.display().to_string())
        .or_else(|| infer_session_path_from_entries(session.entries()));

    let mut records = Vec::new();
    let mut canonical_assistant_ids = std::collections::HashSet::new();

    for entry in session.entries() {
        if let Some(record) =
            canonical_usage_record_from_entry(entry, session_id.clone(), session_path.clone())
        {
            if let Some(assistant_message_id) = record.assistant_message_id.clone() {
                canonical_assistant_ids.insert(assistant_message_id);
            }
            records.push(record);
        }
    }

    let mut turn_index = 0u32;
    for entry in session.entries() {
        if let SessionEntry::Message {
            id,
            parent_id,
            message: Message::Assistant(message),
        } = entry
        {
            if let Some(usage) = &message.usage {
                if !canonical_assistant_ids.contains(id) {
                    records.push(SessionUsageRecord {
                        entry_id: id.clone(),
                        parent_id: parent_id.clone(),
                        request_id: legacy_request_id(id),
                        recorded_at: message.timestamp,
                        provider: None,
                        model: None,
                        session_id: session_id.clone(),
                        session_path: session_path.clone(),
                        assistant_message_id: Some(id.clone()),
                        turn_index: Some(turn_index),
                        usage: UsageTokens::from(usage),
                        cost: None,
                        source: UsageRecordSource::LegacyAssistantMessage,
                    });
                }
                turn_index += 1;
            }
        }
    }

    records
}

/// Sum usage rows without dedupe.
pub fn aggregate_usage(records: &[SessionUsageRecord]) -> UsageTotals {
    let mut totals = UsageTotals::default();
    for record in records {
        totals.add_record(record);
    }
    totals
}

/// Sum usage rows while deduping copied/forked history by stable request id.
pub fn aggregate_usage_deduped(records: &[SessionUsageRecord]) -> UsageTotals {
    let mut seen = std::collections::HashSet::new();
    let mut totals = UsageTotals::default();

    for record in records {
        if seen.insert(record.dedupe_key()) {
            totals.add_record(record);
        }
    }

    totals
}

/// Build a canonical session custom entry for persistence.
pub fn usage_record_entry(
    entry_id: impl Into<String>,
    record: UsageRecordV1,
) -> Result<SessionEntry> {
    Ok(SessionEntry::Custom {
        id: entry_id.into(),
        parent_id: None,
        custom_type: USAGE_CUSTOM_TYPE.to_string(),
        data: record.into_custom_data()?,
    })
}

fn canonical_usage_record_from_entry(
    entry: &SessionEntry,
    fallback_session_id: Option<String>,
    fallback_session_path: Option<String>,
) -> Option<SessionUsageRecord> {
    let SessionEntry::Custom {
        id,
        parent_id,
        custom_type,
        data,
    } = entry
    else {
        return None;
    };

    if custom_type != USAGE_CUSTOM_TYPE {
        return None;
    }

    let record = UsageRecordV1::from_custom_data(data.clone()).ok()?;
    Some(SessionUsageRecord {
        entry_id: id.clone(),
        parent_id: parent_id.clone(),
        request_id: record.request_id,
        recorded_at: record.recorded_at,
        provider: Some(record.provider),
        model: Some(record.model),
        session_id: record.session_id.or(fallback_session_id),
        session_path: record.session_path.or(fallback_session_path),
        assistant_message_id: record.assistant_message_id,
        turn_index: record.turn_index,
        usage: record.usage,
        cost: Some(record.cost),
        source: record.source,
    })
}

fn infer_session_id_from_entries(entries: &[SessionEntry]) -> Option<String> {
    entries.iter().find_map(|entry| {
        let SessionEntry::Custom {
            custom_type, data, ..
        } = entry
        else {
            return None;
        };

        if custom_type != USAGE_CUSTOM_TYPE {
            return None;
        }

        UsageRecordV1::from_custom_data(data.clone())
            .ok()
            .and_then(|record| record.session_id)
    })
}

fn infer_session_path_from_entries(entries: &[SessionEntry]) -> Option<String> {
    entries.iter().find_map(|entry| {
        let SessionEntry::Custom {
            custom_type, data, ..
        } = entry
        else {
            return None;
        };

        if custom_type != USAGE_CUSTOM_TYPE {
            return None;
        }

        UsageRecordV1::from_custom_data(data.clone())
            .ok()
            .and_then(|record| record.session_path)
    })
}

fn canonical_request_id(assistant_message_id: &str) -> String {
    format!("assistant:{assistant_message_id}")
}

fn legacy_request_id(assistant_message_id: &str) -> String {
    format!("legacy-assistant:{assistant_message_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionEntry;
    use imp_llm::{AssistantMessage, ContentBlock, StopReason};

    fn assistant_message(timestamp: u64, usage: Option<Usage>) -> Message {
        Message::Assistant(AssistantMessage {
            content: vec![ContentBlock::Text {
                text: "done".into(),
            }],
            usage,
            stop_reason: StopReason::EndTurn,
            timestamp,
        })
    }

    fn legacy_assistant_entry(id: &str, timestamp: u64, usage: Usage) -> SessionEntry {
        SessionEntry::Message {
            id: id.to_string(),
            parent_id: None,
            message: assistant_message(timestamp, Some(usage)),
        }
    }

    fn canonical_entry(
        entry_id: &str,
        request_id: &str,
        assistant_message_id: Option<&str>,
        session_id: Option<&str>,
        usage: Usage,
        cost: Cost,
    ) -> SessionEntry {
        usage_record_entry(
            entry_id,
            UsageRecordV1::new(
                request_id,
                123,
                "anthropic",
                "claude-3-7-sonnet",
                usage,
                cost,
            )
            .with_session_context(
                session_id.map(str::to_string),
                Some("/tmp/session.jsonl".into()),
                assistant_message_id.map(str::to_string),
                Some(2),
            ),
        )
        .unwrap()
    }

    #[test]
    fn canonical_usage_record_round_trips_through_custom_entry() {
        let entry = canonical_entry(
            "entry-1",
            "req-1",
            Some("assistant-1"),
            Some("session-1"),
            Usage {
                input_tokens: 100,
                output_tokens: 20,
                cache_read_tokens: 5,
                cache_write_tokens: 2,
            },
            Cost {
                input: 1.0,
                output: 2.0,
                cache_read: 0.1,
                cache_write: 0.2,
                total: 3.3,
            },
        );

        let records = usage_records_from_entries(&[entry]);
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.request_id, "req-1");
        assert_eq!(record.provider.as_deref(), Some("anthropic"));
        assert_eq!(record.model.as_deref(), Some("claude-3-7-sonnet"));
        assert_eq!(record.assistant_message_id.as_deref(), Some("assistant-1"));
        assert_eq!(record.turn_index, Some(2));
        assert_eq!(record.source, UsageRecordSource::Canonical);
        assert_eq!(record.usage.input, 100);
        assert_eq!(record.cost.as_ref().unwrap().total, 3.3);
    }

    #[test]
    fn usage_reader_falls_back_to_legacy_assistant_usage() {
        let entries = vec![legacy_assistant_entry(
            "assistant-legacy",
            456,
            Usage {
                input_tokens: 50,
                output_tokens: 10,
                cache_read_tokens: 3,
                cache_write_tokens: 0,
            },
        )];

        let records = usage_records_from_entries(&entries);
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.request_id, "legacy-assistant:assistant-legacy");
        assert_eq!(record.recorded_at, 456);
        assert_eq!(record.source, UsageRecordSource::LegacyAssistantMessage);
        assert_eq!(record.provider, None);
        assert_eq!(record.model, None);
        assert_eq!(record.cost, None);
        assert_eq!(record.turn_index, Some(0));
    }

    #[test]
    fn canonical_record_suppresses_legacy_fallback_for_same_assistant_message() {
        let usage = Usage {
            input_tokens: 80,
            output_tokens: 12,
            cache_read_tokens: 4,
            cache_write_tokens: 1,
        };
        let entries = vec![
            legacy_assistant_entry("assistant-1", 100, usage.clone()),
            canonical_entry(
                "usage-1",
                "req-1",
                Some("assistant-1"),
                Some("session-1"),
                usage,
                Cost {
                    input: 0.8,
                    output: 0.12,
                    cache_read: 0.04,
                    cache_write: 0.01,
                    total: 0.97,
                },
            ),
        ];

        let records = usage_records_from_entries(&entries);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].source, UsageRecordSource::Canonical);
        assert_eq!(records[0].request_id, "req-1");
    }

    #[test]
    fn aggregate_usage_dedupes_forked_history_by_request_id() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 25,
            cache_read_tokens: 10,
            cache_write_tokens: 5,
        };
        let cost = Cost {
            input: 1.0,
            output: 2.0,
            cache_read: 0.3,
            cache_write: 0.4,
            total: 3.7,
        };
        let original = usage_records_from_entries(&[canonical_entry(
            "usage-original",
            "req-shared",
            Some("assistant-1"),
            Some("session-a"),
            usage.clone(),
            cost.clone(),
        )]);
        let forked = usage_records_from_entries(&[canonical_entry(
            "usage-fork",
            "req-shared",
            Some("assistant-1"),
            Some("session-b"),
            usage,
            cost,
        )]);

        let mut all = Vec::new();
        all.extend(original);
        all.extend(forked);

        let raw = aggregate_usage(&all);
        assert_eq!(raw.records, 2);
        assert_eq!(raw.usage.input_tokens, 200);

        let deduped = aggregate_usage_deduped(&all);
        assert_eq!(deduped.records, 1);
        assert_eq!(deduped.usage.input_tokens, 100);
        assert_eq!(deduped.usage.output_tokens, 25);
        assert!((deduped.cost.total - 3.7).abs() < f64::EPSILON);
    }

    #[test]
    fn aggregate_usage_keeps_distinct_legacy_records() {
        let records = usage_records_from_entries(&[
            legacy_assistant_entry(
                "assistant-1",
                100,
                Usage {
                    input_tokens: 10,
                    output_tokens: 2,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0,
                },
            ),
            legacy_assistant_entry(
                "assistant-2",
                200,
                Usage {
                    input_tokens: 20,
                    output_tokens: 4,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0,
                },
            ),
        ]);

        let totals = aggregate_usage_deduped(&records);
        assert_eq!(totals.records, 2);
        assert_eq!(totals.usage.input_tokens, 30);
        assert_eq!(totals.usage.output_tokens, 6);
    }
}
