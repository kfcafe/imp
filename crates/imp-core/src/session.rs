use std::collections::HashMap;
use std::path::{Path, PathBuf};

use imp_llm::Message;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// A single entry in the session JSONL file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionEntry {
    #[serde(rename = "header")]
    Header {
        version: u32,
        created_at: u64,
        cwd: String,
    },
    #[serde(rename = "message")]
    Message {
        id: String,
        parent_id: Option<String>,
        message: Message,
    },
    #[serde(rename = "compaction")]
    Compaction {
        id: String,
        parent_id: Option<String>,
        summary: String,
        first_kept_id: String,
        #[serde(default)]
        tokens_before: u32,
        #[serde(default)]
        tokens_after: u32,
    },
    #[serde(rename = "custom")]
    Custom {
        id: String,
        parent_id: Option<String>,
        custom_type: String,
        data: serde_json::Value,
    },
    #[serde(rename = "label")]
    Label { entry_id: String, label: String },
}

impl SessionEntry {
    /// Get the id of this entry, if it has one (Header and Label don't).
    pub fn id(&self) -> Option<&str> {
        match self {
            SessionEntry::Header { .. } | SessionEntry::Label { .. } => None,
            SessionEntry::Message { id, .. }
            | SessionEntry::Compaction { id, .. }
            | SessionEntry::Custom { id, .. } => Some(id),
        }
    }

    /// Get the parent_id of this entry, if it has one.
    pub fn parent_id(&self) -> Option<&str> {
        match self {
            SessionEntry::Header { .. } | SessionEntry::Label { .. } => None,
            SessionEntry::Message { parent_id, .. }
            | SessionEntry::Compaction { parent_id, .. }
            | SessionEntry::Custom { parent_id, .. } => parent_id.as_deref(),
        }
    }
}

/// A node in the session tree.
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub entry: SessionEntry,
    pub children: Vec<TreeNode>,
}

/// Summary of a session for listing.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub path: PathBuf,
    pub cwd: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub message_count: usize,
    pub first_message: Option<String>,
}

/// Manages a single session's entries and persistence.
pub struct SessionManager {
    entries: Vec<SessionEntry>,
    path: Option<PathBuf>,
    leaf_id: Option<String>,
    session_name: Option<String>,
}

impl SessionManager {
    /// Create a new session. Writes the header to disk immediately.
    pub fn new(cwd: &Path, session_dir: &Path) -> Result<Self> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let path = session_dir.join(format!("{session_id}.jsonl"));
        let header = SessionEntry::Header {
            version: 1,
            created_at: imp_llm::now(),
            cwd: cwd.to_string_lossy().to_string(),
        };

        // Write header to disk immediately
        {
            use std::io::Write;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = std::fs::File::create(&path)?;
            let line = serde_json::to_string(&header)?;
            writeln!(file, "{line}")?;
        }

        Ok(Self {
            entries: vec![header],
            path: Some(path),
            leaf_id: None,
            session_name: None,
        })
    }

    /// Open an existing session file, skipping malformed lines.
    pub fn open(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut entries = Vec::new();
        let mut last_id = None;

        for (line_num, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<SessionEntry>(line) {
                Ok(entry) => {
                    if let Some(id) = entry.id() {
                        last_id = Some(id.to_string());
                    }
                    entries.push(entry);
                }
                Err(e) => {
                    eprintln!(
                        "warning: skipping malformed line {} in {}: {e}",
                        line_num + 1,
                        path.display()
                    );
                }
            }
        }

        Ok(Self {
            entries,
            path: Some(path.to_path_buf()),
            leaf_id: last_id,
            session_name: None,
        })
    }

    /// In-memory session (no persistence).
    pub fn in_memory() -> Self {
        Self {
            entries: Vec::new(),
            path: None,
            leaf_id: None,
            session_name: None,
        }
    }

    /// Find the most recently modified session for a given cwd.
    pub fn continue_recent(cwd: &Path, session_dir: &Path) -> Result<Option<Self>> {
        if !session_dir.exists() {
            return Ok(None);
        }

        let cwd_str = cwd.to_string_lossy().to_string();
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;

        for dir_entry in std::fs::read_dir(session_dir)? {
            let dir_entry = dir_entry?;
            let path = dir_entry.path();
            if path.extension().is_none_or(|e| e != "jsonl") {
                continue;
            }
            // Check modification time first (cheap) before parsing
            let modified = dir_entry
                .metadata()?
                .modified()
                .unwrap_or(std::time::UNIX_EPOCH);

            // Only parse if this could be newer than our current best
            if best.as_ref().is_none_or(|(t, _)| modified > *t) {
                // Read just the first line to check cwd without parsing the whole file
                if let Ok(first_line) = read_first_line(&path) {
                    if let Ok(SessionEntry::Header { cwd, .. }) =
                        serde_json::from_str::<SessionEntry>(&first_line).as_ref()
                    {
                        if *cwd == cwd_str {
                            best = Some((modified, path));
                        }
                    }
                }
            }
        }

        match best {
            Some((_, path)) => Ok(Some(Self::open(&path)?)),
            None => Ok(None),
        }
    }

    /// Get the session name.
    pub fn name(&self) -> Option<&str> {
        self.session_name.as_deref()
    }

    /// Set the session name.
    pub fn set_name(&mut self, name: &str) {
        self.session_name = Some(name.to_string());
    }

    /// Append an entry. Sets parent_id to current leaf_id, updates leaf_id,
    /// and writes to file if persisted.
    pub fn append(&mut self, mut entry: SessionEntry) -> Result<()> {
        // Set parent_id on entries that support it
        match &mut entry {
            SessionEntry::Message { parent_id, .. }
            | SessionEntry::Compaction { parent_id, .. }
            | SessionEntry::Custom { parent_id, .. } => {
                *parent_id = self.leaf_id.clone();
            }
            SessionEntry::Header { .. } | SessionEntry::Label { .. } => {}
        }

        // Update leaf_id
        if let Some(id) = entry.id() {
            self.leaf_id = Some(id.to_string());
        }

        // Write to file
        if let Some(ref path) = self.path {
            use std::io::Write;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
            let line = serde_json::to_string(&entry)?;
            writeln!(file, "{line}")?;
        }

        self.entries.push(entry);
        Ok(())
    }

    /// Walk parent_ids from leaf_id to root, return entries in chronological order.
    pub fn get_branch(&self) -> Vec<&SessionEntry> {
        let Some(ref leaf) = self.leaf_id else {
            // No messages yet — return just the header if present
            return self
                .entries
                .iter()
                .filter(|e| matches!(e, SessionEntry::Header { .. }))
                .collect();
        };

        // Build id -> entry index for fast lookups
        let id_map: HashMap<&str, usize> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.id().map(|id| (id, i)))
            .collect();

        // Walk from leaf to root
        let mut branch = Vec::new();
        let mut current = Some(leaf.as_str());

        while let Some(id) = current {
            if let Some(&idx) = id_map.get(id) {
                let entry = &self.entries[idx];
                branch.push(entry);
                current = entry.parent_id();
            } else {
                break;
            }
        }

        // Include the header
        for entry in &self.entries {
            if matches!(entry, SessionEntry::Header { .. }) {
                branch.push(entry);
                break;
            }
        }

        branch.reverse();
        branch
    }

    /// Get messages for the current branch (for LLM context).
    pub fn get_messages(&self) -> Vec<&Message> {
        self.get_branch()
            .into_iter()
            .filter_map(|e| match e {
                SessionEntry::Message { message, .. } => Some(message),
                _ => None,
            })
            .collect()
    }

    /// Build the full tree structure from all entries.
    pub fn get_tree(&self) -> Vec<TreeNode> {
        // Separate roots (entries with no parent_id that have an id) and children
        let mut children_map: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut roots: Vec<usize> = Vec::new();

        for (i, entry) in self.entries.iter().enumerate() {
            match entry.parent_id() {
                Some(pid) => {
                    children_map.entry(pid).or_default().push(i);
                }
                None => {
                    roots.push(i);
                }
            }
        }

        roots
            .into_iter()
            .map(|i| self.build_subtree(i, &children_map))
            .collect()
    }

    fn build_subtree(&self, idx: usize, children_map: &HashMap<&str, Vec<usize>>) -> TreeNode {
        let entry = &self.entries[idx];
        let children = entry
            .id()
            .and_then(|id| children_map.get(id))
            .map(|child_indices| {
                child_indices
                    .iter()
                    .map(|&ci| self.build_subtree(ci, children_map))
                    .collect()
            })
            .unwrap_or_default();

        TreeNode {
            entry: entry.clone(),
            children,
        }
    }

    /// Change the current position in the tree to a different entry.
    pub fn navigate(&mut self, target_id: &str) -> Result<()> {
        let exists = self.entries.iter().any(|e| e.id() == Some(target_id));
        if !exists {
            return Err(crate::error::Error::Session(format!(
                "entry not found: {target_id}"
            )));
        }
        self.leaf_id = Some(target_id.to_string());
        Ok(())
    }

    /// Create a new session file containing only entries up to (and including) the
    /// given entry_id, following its branch from root.
    pub fn fork(&self, entry_id: &str, new_path: &Path) -> Result<SessionManager> {
        // Build the branch to this entry
        let id_map: HashMap<&str, usize> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.id().map(|id| (id, i)))
            .collect();

        let mut branch_indices = Vec::new();
        let mut current = Some(entry_id);

        while let Some(id) = current {
            if let Some(&idx) = id_map.get(id) {
                branch_indices.push(idx);
                current = self.entries[idx].parent_id();
            } else {
                break;
            }
        }

        branch_indices.reverse();

        // Collect header + branch entries
        let mut forked_entries = Vec::new();
        for entry in &self.entries {
            if matches!(entry, SessionEntry::Header { .. }) {
                forked_entries.push(entry.clone());
                break;
            }
        }
        for idx in &branch_indices {
            forked_entries.push(self.entries[*idx].clone());
        }

        // Also include any Label entries that reference entries in our branch
        let branch_ids: std::collections::HashSet<String> = forked_entries
            .iter()
            .filter_map(|e| e.id().map(String::from))
            .collect();
        let labels: Vec<SessionEntry> = self
            .entries
            .iter()
            .filter(|e| {
                matches!(e, SessionEntry::Label { entry_id, .. } if branch_ids.contains(entry_id.as_str()))
            })
            .cloned()
            .collect();
        forked_entries.extend(labels);

        // Write to new file
        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        {
            use std::io::Write;
            let mut file = std::fs::File::create(new_path)?;
            for entry in &forked_entries {
                let line = serde_json::to_string(entry)?;
                writeln!(file, "{line}")?;
            }
        }

        let leaf_id = forked_entries
            .iter()
            .rev()
            .find_map(|e| e.id())
            .map(String::from);

        Ok(SessionManager {
            entries: forked_entries,
            path: Some(new_path.to_path_buf()),
            leaf_id,
            session_name: None,
        })
    }

    /// Get all entries.
    pub fn entries(&self) -> &[SessionEntry] {
        &self.entries
    }

    /// Get the session file path.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Get the current leaf id.
    pub fn leaf_id(&self) -> Option<&str> {
        self.leaf_id.as_deref()
    }

    /// List available sessions in a directory.
    pub fn list(session_dir: &Path) -> Result<Vec<SessionInfo>> {
        let mut sessions = Vec::new();
        if !session_dir.exists() {
            return Ok(sessions);
        }

        for dir_entry in std::fs::read_dir(session_dir)? {
            let dir_entry = dir_entry?;
            let path = dir_entry.path();
            if path.extension().is_none_or(|e| e != "jsonl") {
                continue;
            }

            let updated_at = dir_entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            if let Ok(session) = Self::open(&path) {
                let cwd = session
                    .entries
                    .iter()
                    .find_map(|e| match e {
                        SessionEntry::Header { cwd, .. } => Some(cwd.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();

                let created_at = session
                    .entries
                    .iter()
                    .find_map(|e| match e {
                        SessionEntry::Header { created_at, .. } => Some(*created_at),
                        _ => None,
                    })
                    .unwrap_or(0);

                let message_count = session
                    .entries
                    .iter()
                    .filter(|e| matches!(e, SessionEntry::Message { .. }))
                    .count();

                let first_message = session.entries.iter().find_map(|e| match e {
                    SessionEntry::Message { message, .. } => extract_text(message),
                    _ => None,
                });

                // Skip sessions with no messages — nothing to resume
                if message_count == 0 {
                    continue;
                }

                sessions.push(SessionInfo {
                    id: path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    path,
                    cwd,
                    created_at,
                    updated_at,
                    message_count,
                    first_message,
                });
            }
        }

        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(sessions)
    }
}

/// Sanitize a message history for API submission.
///
/// Strips unpaired tool_call blocks (assistant tool_use without matching tool_result)
/// and orphaned tool_result messages (tool_result without matching tool_use).
/// This handles both old sessions (before tool_result persistence) and corrupted
/// sessions where tool calls were partially recorded.
pub fn sanitize_messages(messages: &mut Vec<Message>) {
    use std::collections::HashSet;

    // Collect tool_result IDs to find which tool_calls have results
    let result_ids: HashSet<String> = messages
        .iter()
        .filter_map(|m| match m {
            Message::ToolResult(tr) => Some(tr.tool_call_id.clone()),
            _ => None,
        })
        .collect();

    // Strip unpaired tool_call blocks from assistant messages
    for msg in messages.iter_mut() {
        if let Message::Assistant(assistant) = msg {
            assistant.content.retain(|block| match block {
                imp_llm::ContentBlock::ToolCall { id, .. } => result_ids.contains(id),
                _ => true,
            });
        }
    }

    // Remove empty assistant messages left after stripping
    messages.retain(|msg| match msg {
        Message::Assistant(a) => !a.content.is_empty(),
        _ => true,
    });

    // Strip orphaned tool_results whose tool_call no longer exists
    let remaining_call_ids: HashSet<String> = messages
        .iter()
        .filter_map(|m| match m {
            Message::Assistant(a) => Some(a.content.iter().filter_map(|b| match b {
                imp_llm::ContentBlock::ToolCall { id, .. } => Some(id.clone()),
                _ => None,
            })),
            _ => None,
        })
        .flatten()
        .collect();
    messages.retain(|msg| match msg {
        Message::ToolResult(tr) => remaining_call_ids.contains(&tr.tool_call_id),
        _ => true,
    });

    // Reorder: ensure each tool_result follows the assistant message that
    // contains its tool_call. Session persistence can write tool_results
    // before the assistant message (ToolExecutionEnd fires before TurnEnd).
    reorder_tool_results(messages);
}

/// Move tool_result messages so they immediately follow the assistant
/// message containing the matching tool_call.
fn reorder_tool_results(messages: &mut Vec<Message>) {
    use std::collections::HashMap;

    // Build map: tool_call_id → index of the assistant message that has it
    let mut call_to_assistant: HashMap<String, usize> = HashMap::new();
    for (i, msg) in messages.iter().enumerate() {
        if let Message::Assistant(a) = msg {
            for block in &a.content {
                if let imp_llm::ContentBlock::ToolCall { id, .. } = block {
                    call_to_assistant.insert(id.clone(), i);
                }
            }
        }
    }

    // Separate tool_results that are out of order
    let mut deferred: Vec<(usize, Message)> = Vec::new(); // (target_after_idx, msg)
    let mut i = 0;
    while i < messages.len() {
        if let Message::ToolResult(tr) = &messages[i] {
            if let Some(&assistant_idx) = call_to_assistant.get(&tr.tool_call_id) {
                if i < assistant_idx {
                    // tool_result appears before its assistant — pull it out
                    let msg = messages.remove(i);
                    deferred.push((assistant_idx, msg));
                    // Adjust assistant indices after removal
                    for v in call_to_assistant.values_mut() {
                        if *v > i {
                            *v -= 1;
                        }
                    }
                    for d in &mut deferred {
                        if d.0 > i {
                            d.0 -= 1;
                        }
                    }
                    continue; // don't increment i
                }
            }
        }
        i += 1;
    }

    // Re-insert deferred tool_results after their assistant messages
    // Sort by target index descending so insertions don't shift earlier targets
    deferred.sort_by(|a, b| b.0.cmp(&a.0));
    for (target_idx, msg) in deferred {
        let insert_at = (target_idx + 1).min(messages.len());
        messages.insert(insert_at, msg);
    }
}

/// Extract the first text content from a message.
fn extract_text(message: &Message) -> Option<String> {
    let blocks = match message {
        Message::User(u) => &u.content,
        Message::Assistant(a) => &a.content,
        Message::ToolResult(t) => &t.content,
    };
    blocks.iter().find_map(|b| match b {
        imp_llm::ContentBlock::Text { text } => Some(text.clone()),
        _ => None,
    })
}

/// Read just the first non-empty line of a file.
fn read_first_line(path: &Path) -> Result<String> {
    use std::io::BufRead;
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            return Ok(line);
        }
    }
    Err(crate::error::Error::Session("empty file".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use imp_llm::Message;
    use tempfile::TempDir;

    fn make_msg_entry(id: &str, text: &str) -> SessionEntry {
        SessionEntry::Message {
            id: id.to_string(),
            parent_id: None, // append() will set this
            message: Message::user(text),
        }
    }

    #[test]
    fn session_create_append_reopen() {
        let tmp = TempDir::new().unwrap();
        let session_dir = tmp.path().join("sessions");
        let cwd = tmp.path().join("project");

        let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
        mgr.append(make_msg_entry("m1", "hello")).unwrap();
        mgr.append(make_msg_entry("m2", "world")).unwrap();
        mgr.append(make_msg_entry("m3", "!")).unwrap();

        let path = mgr.path().unwrap().to_path_buf();
        assert!(path.exists());

        // Reopen and verify messages match
        let reopened = SessionManager::open(&path).unwrap();
        let original_msgs = mgr.get_messages();
        let reopened_msgs = reopened.get_messages();
        assert_eq!(original_msgs.len(), reopened_msgs.len());
        assert_eq!(reopened_msgs.len(), 3);

        // Verify parent chain: m1 has no parent, m2's parent is m1, m3's parent is m2
        let entries = reopened.entries();
        for entry in entries {
            if let SessionEntry::Message { id, parent_id, .. } = entry {
                match id.as_str() {
                    "m1" => assert_eq!(*parent_id, None),
                    "m2" => assert_eq!(parent_id.as_deref(), Some("m1")),
                    "m3" => assert_eq!(parent_id.as_deref(), Some("m2")),
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn session_branch() {
        let mut mgr = SessionManager::in_memory();
        // Append 5 messages (m1..m5)
        for i in 1..=5 {
            mgr.append(make_msg_entry(&format!("m{i}"), &format!("msg {i}")))
                .unwrap();
        }
        assert_eq!(mgr.get_messages().len(), 5);
        assert_eq!(mgr.leaf_id(), Some("m5"));

        // Navigate back to m3
        mgr.navigate("m3").unwrap();
        assert_eq!(mgr.leaf_id(), Some("m3"));

        // Append 2 new messages on the branch
        mgr.append(make_msg_entry("b1", "branch 1")).unwrap();
        mgr.append(make_msg_entry("b2", "branch 2")).unwrap();

        // get_branch should return: header-less chain of m1, m2, m3, b1, b2
        let branch = mgr.get_branch();
        let branch_ids: Vec<Option<&str>> = branch.iter().map(|e| e.id()).collect();
        assert_eq!(
            branch_ids,
            vec![Some("m1"), Some("m2"), Some("m3"), Some("b1"), Some("b2")]
        );
        assert_eq!(mgr.get_messages().len(), 5);

        // Navigate back to m5 to verify original branch still works
        mgr.navigate("m5").unwrap();
        let main_branch = mgr.get_branch();
        let main_ids: Vec<Option<&str>> = main_branch.iter().map(|e| e.id()).collect();
        assert_eq!(
            main_ids,
            vec![Some("m1"), Some("m2"), Some("m3"), Some("m4"), Some("m5")]
        );
    }

    #[test]
    fn session_fork() {
        let tmp = TempDir::new().unwrap();
        let session_dir = tmp.path().join("sessions");
        let cwd = tmp.path().join("project");

        let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
        for i in 1..=5 {
            mgr.append(make_msg_entry(&format!("m{i}"), &format!("msg {i}")))
                .unwrap();
        }

        let fork_path = session_dir.join("forked.jsonl");
        let forked = mgr.fork("m3", &fork_path).unwrap();

        // Forked session should have header + m1, m2, m3
        assert_eq!(forked.get_messages().len(), 3);
        assert_eq!(forked.leaf_id(), Some("m3"));
        assert!(fork_path.exists());

        // Reopen the forked file and verify
        let reopened = SessionManager::open(&fork_path).unwrap();
        assert_eq!(reopened.get_messages().len(), 3);
    }

    #[test]
    fn session_list() {
        let tmp = TempDir::new().unwrap();
        let session_dir = tmp.path().join("sessions");
        let cwd = tmp.path().join("project");

        // Create two sessions
        let mut s1 = SessionManager::new(&cwd, &session_dir).unwrap();
        s1.append(make_msg_entry("a1", "first session")).unwrap();

        let mut s2 = SessionManager::new(&cwd, &session_dir).unwrap();
        s2.append(make_msg_entry("b1", "second session")).unwrap();
        s2.append(make_msg_entry("b2", "more stuff")).unwrap();

        let sessions = SessionManager::list(&session_dir).unwrap();
        assert_eq!(sessions.len(), 2);

        // Both should have the right cwd
        for s in &sessions {
            assert_eq!(s.cwd, cwd.to_string_lossy().to_string());
        }

        // One has 1 message, the other has 2
        let mut counts: Vec<usize> = sessions.iter().map(|s| s.message_count).collect();
        counts.sort();
        assert_eq!(counts, vec![1, 2]);

        // first_message should be set
        for s in &sessions {
            assert!(s.first_message.is_some());
        }
    }

    #[test]
    fn session_continue_recent() {
        let tmp = TempDir::new().unwrap();
        let session_dir = tmp.path().join("sessions");
        let cwd_a = tmp.path().join("project-a");
        let cwd_b = tmp.path().join("project-b");

        // Create a session for cwd_a
        let mut s1 = SessionManager::new(&cwd_a, &session_dir).unwrap();
        s1.append(make_msg_entry("a1", "hello from a")).unwrap();

        // Create a session for cwd_b
        let mut s2 = SessionManager::new(&cwd_b, &session_dir).unwrap();
        s2.append(make_msg_entry("b1", "hello from b")).unwrap();

        // continue_recent for cwd_a should find s1
        let continued = SessionManager::continue_recent(&cwd_a, &session_dir)
            .unwrap()
            .expect("should find a session");
        assert_eq!(continued.get_messages().len(), 1);

        // continue_recent for a non-existent cwd returns None
        let none =
            SessionManager::continue_recent(Path::new("/nonexistent"), &session_dir).unwrap();
        assert!(none.is_none());
    }

    #[test]
    fn session_in_memory() {
        let mut mgr = SessionManager::in_memory();
        assert!(mgr.path().is_none());

        mgr.append(make_msg_entry("m1", "hello")).unwrap();
        mgr.append(make_msg_entry("m2", "world")).unwrap();

        assert_eq!(mgr.get_messages().len(), 2);
        assert_eq!(mgr.entries().len(), 2);
    }

    #[test]
    fn session_malformed_jsonl() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.jsonl");

        // Write a file with a mix of valid and invalid lines
        let content = format!(
            "{}\n\
             NOT VALID JSON\n\
             {}\n\
             {{\"type\":\"unknown_variant\",\"foo\":1}}\n\
             {}\n",
            serde_json::to_string(&SessionEntry::Header {
                version: 1,
                created_at: 1000,
                cwd: "/tmp".into(),
            })
            .unwrap(),
            serde_json::to_string(&SessionEntry::Message {
                id: "m1".into(),
                parent_id: None,
                message: Message::user("hello"),
            })
            .unwrap(),
            serde_json::to_string(&SessionEntry::Message {
                id: "m2".into(),
                parent_id: Some("m1".into()),
                message: Message::user("world"),
            })
            .unwrap(),
        );
        std::fs::write(&path, content).unwrap();

        // Should succeed, skipping the bad lines
        let mgr = SessionManager::open(&path).unwrap();
        // Header + 2 valid messages (bad lines skipped)
        assert_eq!(mgr.entries().len(), 3);
        assert_eq!(mgr.get_messages().len(), 2);
    }

    #[test]
    fn session_get_tree() {
        let mut mgr = SessionManager::in_memory();
        for i in 1..=3 {
            mgr.append(make_msg_entry(&format!("m{i}"), &format!("msg {i}")))
                .unwrap();
        }
        // Branch from m2
        mgr.navigate("m2").unwrap();
        mgr.append(make_msg_entry("b1", "branch")).unwrap();

        let tree = mgr.get_tree();
        // Root should be m1 (no parent)
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].entry.id(), Some("m1"));

        // m1 -> m2
        assert_eq!(tree[0].children.len(), 1);
        let m2_node = &tree[0].children[0];
        assert_eq!(m2_node.entry.id(), Some("m2"));

        // m2 has two children: m3 and b1
        assert_eq!(m2_node.children.len(), 2);
        let child_ids: Vec<Option<&str>> = m2_node.children.iter().map(|n| n.entry.id()).collect();
        assert!(child_ids.contains(&Some("m3")));
        assert!(child_ids.contains(&Some("b1")));
    }

    #[test]
    fn session_navigate_invalid() {
        let mut mgr = SessionManager::in_memory();
        mgr.append(make_msg_entry("m1", "hello")).unwrap();

        let result = mgr.navigate("nonexistent");
        assert!(result.is_err());
    }
}
