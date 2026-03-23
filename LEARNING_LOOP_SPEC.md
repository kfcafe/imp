# Learning Loop — Technical Specification

Addendum to `imp_core_plan.md`. Adds closed-loop learning: agent-curated
memory, skill self-management, session search, and learning nudges.

Inspired by Hermes Agent's learning loop, adapted for Tower's architecture.

---

## 1. Agent Memory

Two bounded markdown files the agent reads and writes. Injected into every
system prompt. The agent manages its own memory — humans can also edit the
files directly.

### Storage

| File | Purpose | Char limit | ~Tokens |
|------|---------|-----------|---------|
| `~/.config/imp/memory.md` | Agent's notes — environment facts, tool quirks, lessons | 2,200 | ~800 |
| `~/.config/imp/user.md` | User profile — preferences, communication style, skill level | 1,400 | ~500 |

Format: plain markdown. Entries separated by `§` (section sign) on its own
line. Each entry is 1-3 lines of dense, actionable text.

```markdown
User runs macOS 15, Homebrew, Docker Desktop. Shell: zsh. Editor: neovim.
§
Project ~/tower uses Rust workspace with sccache. Build with cargo check -p <crate>.
§
User prefers concise responses. Dislikes verbose explanations.
```

### Memory Tool

New tool in imp's tool registry: `memory`.

```rust
pub struct MemoryTool;

// Tool definition
name = "memory"
label = "Memory"
description = "Manage persistent memory across sessions. Use to save environment facts, \
               user preferences, and lessons learned."
readonly = false

// Parameters
{
    "type": "object",
    "required": ["action", "target"],
    "properties": {
        "action": {
            "type": "string",
            "enum": ["add", "replace", "remove"],
            "description": "Action to perform"
        },
        "target": {
            "type": "string",
            "enum": ["memory", "user"],
            "description": "Which store: 'memory' for agent notes, 'user' for user profile"
        },
        "content": {
            "type": "string",
            "description": "Content to add (for 'add') or replacement text (for 'replace')"
        },
        "old_text": {
            "type": "string",
            "description": "Unique substring identifying the entry to replace or remove"
        }
    }
}
```

**Actions:**

- `add` — Append a new entry. Requires `content`. Returns error if at capacity.
- `replace` — Find the entry containing `old_text` (unique substring match),
  replace it with `content`. Error if zero or multiple matches.
- `remove` — Find the entry containing `old_text`, delete it. Error if zero or
  multiple matches.

**Capacity enforcement:** When `add` would exceed the char limit, the tool
returns an error including current entries and usage, prompting the agent to
consolidate or remove before retrying.

**Duplicate detection:** Reject exact duplicate entries. Return success with
"already exists" message.

**Security scanning:** Before persisting, scan content for prompt injection
patterns, credential strings, and invisible Unicode. Block with a clear error
if triggered.

### System Prompt Integration

New **Layer 6: Agent Memory** added after Layer 5 (Task Context) in
`system_prompt.rs`.

```
══════════════════════════════════════════════
MEMORY (your personal notes) [67% — 1,474/2,200 chars]
══════════════════════════════════════════════
User runs macOS 15, Homebrew, Docker Desktop. Shell: zsh. Editor: neovim.
§
Project ~/tower uses Rust workspace with sccache. ...
══════════════════════════════════════════════
USER PROFILE [45% — 630/1,400 chars]
══════════════════════════════════════════════
Name: Asher. Prefers concise output. ...
```

**Frozen snapshot pattern:** Memory is loaded once at session start and
injected as a static block. Writes during the session are persisted to disk
immediately but do NOT mutate the system prompt mid-session. This preserves
LLM prefix caching. The tool result always shows the live state so the agent
knows what's stored.

### Memory Store (Rust)

```rust
/// Persistent memory store backed by a single markdown file.
pub struct MemoryStore {
    path: PathBuf,
    entries: Vec<String>,
    char_limit: usize,
}

impl MemoryStore {
    pub fn load(path: &Path, char_limit: usize) -> Result<Self>;
    pub fn save(&self) -> Result<()>;

    pub fn add(&mut self, content: &str) -> Result<MemoryResult>;
    pub fn replace(&mut self, old_text: &str, content: &str) -> Result<MemoryResult>;
    pub fn remove(&mut self, old_text: &str) -> Result<MemoryResult>;

    pub fn entries(&self) -> &[String];
    pub fn usage(&self) -> (usize, usize); // (used_chars, limit)
    pub fn render(&self, label: &str) -> String; // for system prompt injection
}

pub struct MemoryResult {
    pub success: bool,
    pub message: String,
    pub entries: Vec<String>,
    pub usage: String, // e.g. "1,474/2,200"
}
```

File format: entries separated by `\n§\n`. The store parses on load and
serializes on every write.

---

## 2. Skill Self-Management

The agent can create, patch, and delete skills. Agent-created skills live in a
dedicated subdirectory to distinguish them from human-installed skills.

### Storage

Agent-created skills: `~/.config/imp/skills/agent/`

Each skill follows the agentskills.io standard:
```
~/.config/imp/skills/agent/
├── deploy-k8s/
│   ├── SKILL.md
│   └── references/
├── debug-tokio/
│   └── SKILL.md
└── ...
```

### Skill Manage Tool

New tool in imp's tool registry: `skill_manage`.

```rust
pub struct SkillManageTool;

// Tool definition
name = "skill_manage"
label = "Skill Manager"
description = "Create, update, and delete skills. Use after completing complex tasks \
               to save the approach for future reuse. Use to fix skills that are \
               incomplete or incorrect."
readonly = false

// Parameters
{
    "type": "object",
    "required": ["action", "name"],
    "properties": {
        "action": {
            "type": "string",
            "enum": ["create", "patch", "delete"],
            "description": "Action: create a new skill, patch an existing one, or delete"
        },
        "name": {
            "type": "string",
            "description": "Skill name (lowercase, hyphens, e.g. 'deploy-k8s')"
        },
        "content": {
            "type": "string",
            "description": "Full SKILL.md content (for 'create') including frontmatter"
        },
        "old_text": {
            "type": "string",
            "description": "Text to find in the skill (for 'patch')"
        },
        "new_text": {
            "type": "string",
            "description": "Replacement text (for 'patch')"
        }
    }
}
```

**Actions:**

- `create` — Write a new SKILL.md file at
  `~/.config/imp/skills/agent/{name}/SKILL.md`. Requires `content` (full
  SKILL.md with frontmatter). Error if skill already exists (use `patch`).
- `patch` — Find `old_text` in the existing SKILL.md, replace with `new_text`.
  Preferred over full rewrites for token efficiency.
- `delete` — Remove the skill directory entirely. Only works on agent-created
  skills (inside `agent/` subdirectory).

**Name validation:** Lowercase letters, numbers, hyphens. Max 64 chars. No
leading/trailing hyphens. No consecutive hyphens. (agentskills.io spec)

**Content validation:** Must contain valid YAML frontmatter with `name` and
`description` fields.

### agentskills.io Alignment

Imp's SKILL.md frontmatter adopts the agentskills.io spec fields:

```yaml
---
name: deploy-k8s
description: Deploy services to Kubernetes clusters with rollback safety
license: MIT  # optional
compatibility: Requires kubectl and helm  # optional
metadata:  # optional
  author: imp-agent
  version: "1.0"
  created: "2026-03-23"
---
```

Existing imp skills that lack the full spec fields continue to work — only
`name` and `description` are required by discovery.

---

## 3. Learning Loop Nudges

The glue that makes memory and skill tools get used. Implemented as system
prompt additions plus hook-triggered prompts.

### System Prompt Additions

Added to Layer 1 (Identity) after tool descriptions:

```
## Memory & Learning

You have persistent memory across sessions. Use the memory tool to save:
- Environment facts (OS, tools, project setup) → target: memory
- User preferences and corrections → target: user
- Lessons learned and tool quirks → target: memory

When you complete a complex task (5+ tool calls, error recovery, or user
correction), consider saving the approach as a skill via skill_manage.

When you load a skill and find it incomplete or wrong, patch it.

Do NOT save: trivial facts, easily re-discovered info, raw data dumps, or
anything already in AGENTS.md.
```

### Hook Integration

No new hook events needed. Uses existing `OnAgentEnd`:

In the builder, register a programmatic hook for `on_agent_end` that counts
tool calls in the session. If the count exceeds a threshold (default: 8),
the hook injects a "learning check" user message before the session fully
ends:

```
Before we finish — this was a complex session. Consider:
1. Is there anything worth saving to memory (environment facts, lessons)?
2. Should the approach be saved as a skill for future reuse?
3. If you used a skill that was wrong or incomplete, patch it.
```

This is configurable via config:

```toml
[learning]
enabled = true
skill_nudge_threshold = 8   # tool calls before nudging
```

And can be disabled entirely with `enabled = false`.

---

## 4. Session Search

SQLite FTS5 index over past sessions. Agent can search its own history.

### Session Index

New module: `imp-core/src/session_index.rs`

```rust
pub struct SessionIndex {
    db: rusqlite::Connection,
}

impl SessionIndex {
    /// Open or create the index database.
    pub fn open(path: &Path) -> Result<Self>;

    /// Index a completed session. Extracts user messages, assistant text,
    /// and compaction summaries. Stores session ID, cwd, timestamp, and
    /// full-text content.
    pub fn index_session(&self, session: &SessionManager) -> Result<()>;

    /// Full-text search across all indexed sessions.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SessionSearchHit>>;
}

pub struct SessionSearchHit {
    pub session_id: String,
    pub cwd: String,
    pub created_at: u64,
    pub snippet: String,      // FTS5 snippet with highlights
    pub message_count: usize,
    pub first_message: Option<String>,
}
```

**Database location:** `~/.local/share/imp/session_index.db`

**Schema:**
```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    cwd TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    message_count INTEGER NOT NULL,
    first_message TEXT
);

CREATE VIRTUAL TABLE session_content USING fts5(
    session_id,
    content,
    tokenize='porter unicode61'
);
```

**Indexing pipeline:** When a session ends (or is saved), extract all user
messages and assistant text (not tool results — too noisy), concatenate, and
insert into the FTS5 table. Compaction summaries are also indexed as they're
high-signal condensed versions of conversation chunks.

### Session Search Tool

New tool in imp's tool registry: `session_search`.

```rust
pub struct SessionSearchTool;

name = "session_search"
label = "Session Search"
description = "Search past conversations. Use when you need to recall something \
               discussed in a previous session."
readonly = true

// Parameters
{
    "type": "object",
    "required": ["query"],
    "properties": {
        "query": {
            "type": "string",
            "description": "Search query (supports FTS5 syntax: AND, OR, NOT, phrases)"
        },
        "limit": {
            "type": "integer",
            "description": "Max results (default: 5)"
        }
    }
}
```

Returns matching sessions with FTS5 snippets. No LLM summarization in v1 —
the snippets with search term highlighting are good enough, and avoid the
cost/latency of a summarization call. LLM summarization can be added later as
an enhancement.

---

## 5. Config

New config section:

```toml
[learning]
enabled = true                    # master switch for memory + skill nudges
skill_nudge_threshold = 8         # tool calls before suggesting skill creation
memory_char_limit = 2200          # MEMORY.md char limit
user_char_limit = 1400            # USER.md char limit

[session]
index_enabled = true              # enable FTS5 session indexing
```

---

## 6. Build Order

These features are independent of each other and can be built in parallel,
but the recommended order (each builds on prior understanding):

1. **Memory store + tool** — `MemoryStore` struct, file I/O, `MemoryTool`
   implementation. Unit-testable in isolation.

2. **System prompt Layer 6** — Wire memory into `system_prompt::assemble()`.
   Wire into `AgentBuilder::build()` to load memory at startup.

3. **Skill manage tool** — `SkillManageTool` implementation. Uses existing
   skill discovery infra.

4. **Learning nudges** — System prompt text + `OnAgentEnd` hook in builder.
   Config integration.

5. **Session index** — `SessionIndex` with rusqlite + FTS5. Index on session
   end.

6. **Session search tool** — `SessionSearchTool` wired to the index.

Steps 1-4 form the core learning loop. Steps 5-6 add cross-session recall.

---

## 7. New Dependencies

| Crate | Purpose | Size impact |
|-------|---------|-------------|
| `rusqlite` (with `bundled` + `fts5`) | Session index | ~2MB (bundles SQLite) |

rusqlite is only needed for session search (steps 5-6). Steps 1-4 use plain
files only.
