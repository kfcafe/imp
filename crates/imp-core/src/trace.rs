use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub const TRACE_SCHEMA_VERSION: u32 = 1;
const DEFAULT_MAX_STRING_CHARS: usize = 16 * 1024;

const SECRET_REDACTION: &str = "[REDACTED]";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TraceCorrelation {
    pub message_id: Option<String>,
    pub tool_call_id: Option<String>,
    pub parent_event_id: Option<String>,
    pub verification_gate_id: Option<String>,
    pub evidence_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TraceRedaction {
    pub contains_redactions: bool,
    pub truncated_fields: Vec<String>,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TraceEvent {
    pub schema_version: u32,
    pub sequence: u64,
    pub timestamp_ms: i128,
    pub run_id: String,
    pub workflow_id: Option<String>,
    pub session_id: Option<String>,
    pub turn: Option<u32>,
    pub kind: String,
    pub correlation: TraceCorrelation,
    pub redaction: TraceRedaction,
    pub payload: Value,
}

impl TraceEvent {
    pub fn new(run_id: impl Into<String>, kind: impl Into<String>, payload: Value) -> Self {
        Self {
            schema_version: TRACE_SCHEMA_VERSION,
            sequence: 0,
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_millis() as i128)
                .unwrap_or_default(),
            run_id: run_id.into(),
            workflow_id: None,
            session_id: None,
            turn: None,
            kind: kind.into(),
            correlation: TraceCorrelation::default(),
            redaction: TraceRedaction::default(),
            payload,
        }
    }

    pub fn with_turn(mut self, turn: u32) -> Self {
        self.turn = Some(turn);
        self
    }

    pub fn with_tool_call_id(mut self, tool_call_id: impl Into<String>) -> Self {
        self.correlation.tool_call_id = Some(tool_call_id.into());
        self
    }

    pub fn with_workflow_id(mut self, workflow_id: impl Into<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self
    }
}

impl Default for TraceEvent {
    fn default() -> Self {
        Self::new(String::new(), String::new(), Value::Null)
    }
}

#[derive(Debug, Clone)]
pub struct TraceWriterOptions {
    pub max_string_chars: usize,
}

impl Default for TraceWriterOptions {
    fn default() -> Self {
        Self {
            max_string_chars: DEFAULT_MAX_STRING_CHARS,
        }
    }
}

pub struct TraceWriter {
    writer: BufWriter<File>,
    next_sequence: u64,
    options: TraceWriterOptions,
}

impl TraceWriter {
    pub fn create(path: impl AsRef<Path>) -> std::io::Result<Self> {
        Self::create_with_options(path, TraceWriterOptions::default())
    }

    pub fn create_with_options(
        path: impl AsRef<Path>,
        options: TraceWriterOptions,
    ) -> std::io::Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path.as_ref())?;
        set_private_permissions(path.as_ref())?;
        Ok(Self {
            writer: BufWriter::new(file),
            next_sequence: 0,
            options,
        })
    }

    pub fn append(path: impl AsRef<Path>) -> std::io::Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path.as_ref())?;
        set_private_permissions(path.as_ref())?;
        Ok(Self {
            writer: BufWriter::new(file),
            next_sequence: 0,
            options: TraceWriterOptions::default(),
        })
    }

    pub fn write_event(&mut self, mut event: TraceEvent) -> std::io::Result<u64> {
        event.sequence = self.next_sequence;
        self.next_sequence += 1;
        redact_secret_fields(&mut event);
        truncate_event_strings(&mut event, self.options.max_string_chars);
        serde_json::to_writer(&mut self.writer, &event)?;
        self.writer.write_all(b"\n")?;
        Ok(event.sequence)
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

fn set_private_permissions(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn redact_secret_fields(event: &mut TraceEvent) {
    redact_value_secret_fields(&mut event.payload, "payload", &mut event.redaction);
}

fn redact_value_secret_fields(value: &mut Value, path: &str, redaction: &mut TraceRedaction) {
    match value {
        Value::Object(map) => {
            for (key, value) in map.iter_mut() {
                let child_path = format!("{path}.{key}");
                if is_secret_key(key) {
                    if !value.is_null() {
                        *value = Value::String(SECRET_REDACTION.to_string());
                        redaction.contains_redactions = true;
                        redaction.truncated_fields.push(child_path);
                    }
                } else {
                    redact_value_secret_fields(value, &child_path, redaction);
                }
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                redact_value_secret_fields(item, &format!("{path}[{index}]"), redaction);
            }
        }
        Value::String(_) | Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn is_secret_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    matches!(
        normalized.as_str(),
        "apikey"
            | "authorization"
            | "bearer"
            | "clientsecret"
            | "password"
            | "secret"
            | "secretkey"
            | "secretskey"
            | "token"
    ) || normalized.ends_with("token")
        || normalized.ends_with("secret")
        || normalized.ends_with("apikey")
}

fn truncate_event_strings(event: &mut TraceEvent, max_chars: usize) {
    truncate_value_strings(
        &mut event.payload,
        max_chars,
        "payload",
        &mut event.redaction,
    );
}

fn truncate_value_strings(
    value: &mut Value,
    max_chars: usize,
    path: &str,
    redaction: &mut TraceRedaction,
) {
    match value {
        Value::String(text) => {
            if text.chars().count() > max_chars {
                let truncated = text.chars().take(max_chars).collect::<String>();
                *text = format!("{truncated}…[truncated]");
                redaction.contains_redactions = true;
                redaction.truncated_fields.push(path.to_string());
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                truncate_value_strings(item, max_chars, &format!("{path}[{index}]"), redaction);
            }
        }
        Value::Object(map) => {
            for (key, value) in map.iter_mut() {
                truncate_value_strings(value, max_chars, &format!("{path}.{key}"), redaction);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[cfg(unix)]
    #[test]
    fn trace_jsonl_is_private_on_disk() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("trace.jsonl");
        let mut writer = TraceWriter::create(&path).unwrap();
        writer
            .write_event(TraceEvent::new("run-1", "test", json!({})))
            .unwrap();
        writer.flush().unwrap();

        let mode = std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn trace_jsonl_writes_ordered_roundtrippable_events() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("trace.jsonl");
        let mut writer = TraceWriter::create(&path).unwrap();
        writer
            .write_event(TraceEvent::new(
                "run-1",
                "agent.start",
                json!({"model": "test"}),
            ))
            .unwrap();
        writer
            .write_event(TraceEvent::new("run-1", "turn.start", json!({"index": 1})))
            .unwrap();
        writer.flush().unwrap();

        let contents = std::fs::read_to_string(path).unwrap();
        let events = contents
            .lines()
            .map(|line| serde_json::from_str::<TraceEvent>(line).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence, 0);
        assert_eq!(events[1].sequence, 1);
        assert_eq!(events[0].schema_version, TRACE_SCHEMA_VERSION);
        assert_eq!(events[0].kind, "agent.start");
    }

    #[test]
    fn trace_jsonl_redacts_secret_keyed_payload_fields() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("trace.jsonl");
        let mut writer = TraceWriter::create(&path).unwrap();
        writer
            .write_event(TraceEvent::new(
                "run-1",
                "tool.output",
                json!({
                    "api_key": "sk-test-secret",
                    "nested": {"Authorization": "Bearer token-value"},
                    "items": [{"secret_key": "hidden"}],
                    "safe": "visible"
                }),
            ))
            .unwrap();
        writer.flush().unwrap();

        let contents = std::fs::read_to_string(path).unwrap();
        assert!(!contents.contains("sk-test-secret"));
        assert!(!contents.contains("token-value"));
        assert!(!contents.contains("hidden"));
        let event: TraceEvent = serde_json::from_str(contents.lines().next().unwrap()).unwrap();
        assert!(event.redaction.contains_redactions);
        assert_eq!(event.payload["api_key"], SECRET_REDACTION);
        assert_eq!(event.payload["nested"]["Authorization"], SECRET_REDACTION);
        assert_eq!(event.payload["items"][0]["secret_key"], SECRET_REDACTION);
        assert_eq!(event.payload["safe"], "visible");
    }

    #[test]
    fn trace_jsonl_truncates_large_strings_and_marks_redaction() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("trace.jsonl");
        let mut writer = TraceWriter::create_with_options(
            &path,
            TraceWriterOptions {
                max_string_chars: 8,
            },
        )
        .unwrap();
        writer
            .write_event(TraceEvent::new(
                "run-1",
                "tool.output.delta",
                json!({"text": "abcdefghijklmnopqrstuvwxyz"}),
            ))
            .unwrap();
        writer.flush().unwrap();

        let contents = std::fs::read_to_string(path).unwrap();
        let event: TraceEvent = serde_json::from_str(contents.lines().next().unwrap()).unwrap();
        assert!(event.redaction.contains_redactions);
        assert_eq!(event.redaction.truncated_fields, vec!["payload.text"]);
        assert_eq!(event.payload["text"], "abcdefgh…[truncated]");
    }
}
