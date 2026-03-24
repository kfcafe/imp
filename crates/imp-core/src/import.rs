use std::path::{Path, PathBuf};

/// A detected external agent source that imp can import from.
#[derive(Debug, Clone)]
pub struct DetectedSource {
    pub agent: AgentSource,
    pub skills: Vec<DetectedSkill>,
    pub agents_md: Vec<DetectedAgentsMd>,
}

/// Which agent tool the source comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSource {
    Pi,
    ClaudeCode,
    Codex,
}

impl AgentSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pi => "pi",
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex",
        }
    }
}

/// A skill discovered in another agent's config.
#[derive(Debug, Clone)]
pub struct DetectedSkill {
    pub name: String,
    pub description: String,
    pub source_path: PathBuf,
}

/// An AGENTS.md or CLAUDE.md discovered in another agent's config.
#[derive(Debug, Clone)]
pub struct DetectedAgentsMd {
    pub path: PathBuf,
    pub kind: AgentsMdKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentsMdKind {
    AgentsMd,
    ClaudeMd,
}

impl AgentsMdKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AgentsMd => "AGENTS.md",
            Self::ClaudeMd => "CLAUDE.md",
        }
    }
}

/// Scan all known agent sources and return what was found.
pub fn detect_sources(home: &Path) -> Vec<DetectedSource> {
    let mut sources = Vec::new();

    if let Some(pi) = detect_pi(home) {
        sources.push(pi);
    }
    if let Some(claude) = detect_claude_code(home) {
        sources.push(claude);
    }
    if let Some(codex) = detect_codex(home) {
        sources.push(codex);
    }

    sources
}

fn detect_pi(home: &Path) -> Option<DetectedSource> {
    let pi_dir = home.join(".pi").join("agent");
    if !pi_dir.exists() {
        return None;
    }

    let skills = discover_skills_in_dir(&pi_dir.join("skills"));

    let mut agents_md = Vec::new();
    let agents_path = pi_dir.join("AGENTS.md");
    if agents_path.exists() {
        agents_md.push(DetectedAgentsMd {
            path: agents_path,
            kind: AgentsMdKind::AgentsMd,
        });
    }

    if skills.is_empty() && agents_md.is_empty() {
        return None;
    }

    Some(DetectedSource {
        agent: AgentSource::Pi,
        skills,
        agents_md,
    })
}

fn detect_claude_code(home: &Path) -> Option<DetectedSource> {
    let claude_dir = home.join(".claude");
    if !claude_dir.exists() {
        return None;
    }

    let mut agents_md = Vec::new();

    // ~/.claude/CLAUDE.md
    let claude_md = claude_dir.join("CLAUDE.md");
    if claude_md.exists() {
        agents_md.push(DetectedAgentsMd {
            path: claude_md,
            kind: AgentsMdKind::ClaudeMd,
        });
    }

    if agents_md.is_empty() {
        return None;
    }

    Some(DetectedSource {
        agent: AgentSource::ClaudeCode,
        skills: Vec::new(),
        agents_md,
    })
}

fn detect_codex(home: &Path) -> Option<DetectedSource> {
    // Codex uses ~/.codex/ or project-level AGENTS.md (which imp already reads).
    // Check for user-level codex config.
    let codex_dir = home.join(".codex");
    if !codex_dir.exists() {
        return None;
    }

    let mut agents_md = Vec::new();

    let instructions = codex_dir.join("instructions.md");
    if instructions.exists() {
        agents_md.push(DetectedAgentsMd {
            path: instructions,
            kind: AgentsMdKind::AgentsMd,
        });
    }

    if agents_md.is_empty() {
        return None;
    }

    Some(DetectedSource {
        agent: AgentSource::Codex,
        skills: Vec::new(),
        agents_md,
    })
}

fn discover_skills_in_dir(dir: &Path) -> Vec<DetectedSkill> {
    let mut skills = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return skills,
    };

    for entry in entries.flatten() {
        let skill_dir = entry.path();
        let skill_file = skill_dir.join("SKILL.md");
        if !skill_file.exists() {
            continue;
        }

        let content = match std::fs::read_to_string(&skill_file) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let name = skill_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let description = extract_skill_description(&content);

        skills.push(DetectedSkill {
            name,
            description,
            source_path: skill_file,
        });
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

fn extract_skill_description(content: &str) -> String {
    // Parse YAML frontmatter for description field.
    // Simple string extraction — no YAML parser dependency.
    let lines: Vec<&str> = content.lines().collect();
    if lines.first().copied() != Some("---") {
        return crate::resources::extract_description(content);
    }

    let end = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(i, l)| (*l == "---").then_some(i));

    let Some(end) = end else {
        return String::new();
    };

    // Look for "description:" in frontmatter lines
    let mut description = String::new();
    let mut in_description = false;

    for line in &lines[1..end] {
        if let Some(rest) = line.strip_prefix("description:") {
            // Inline value: "description: Some text" or "description: >"
            let value = rest.trim();
            if value == ">" || value == "|" {
                // Multi-line scalar follows
                in_description = true;
                continue;
            }
            // Single-line value (may be quoted)
            let value = value.trim_matches('\'').trim_matches('"');
            return value.to_string();
        } else if in_description {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                // End of multi-line block
                break;
            }
            if !line.starts_with(' ') && !line.starts_with('\t') {
                // New key — description block is over
                break;
            }
            if !description.is_empty() {
                description.push(' ');
            }
            description.push_str(trimmed);
        }
    }

    description
}

/// Result of importing skills.
#[derive(Debug)]
pub struct ImportResult {
    pub copied: Vec<String>,
    pub skipped: Vec<(String, SkipReason)>,
}

#[derive(Debug)]
pub enum SkipReason {
    AlreadyExists,
    CopyFailed(String),
}

/// Copy skills from a detected source into imp's skill directory.
pub fn import_skills(
    skills: &[DetectedSkill],
    imp_skills_dir: &Path,
) -> std::io::Result<ImportResult> {
    std::fs::create_dir_all(imp_skills_dir)?;

    let mut result = ImportResult {
        copied: Vec::new(),
        skipped: Vec::new(),
    };

    for skill in skills {
        let dest_dir = imp_skills_dir.join(&skill.name);

        if dest_dir.exists() {
            result
                .skipped
                .push((skill.name.clone(), SkipReason::AlreadyExists));
            continue;
        }

        // Copy the entire skill directory
        let source_dir = skill.source_path.parent().unwrap_or(Path::new("."));
        match copy_dir_recursive(source_dir, &dest_dir) {
            Ok(()) => result.copied.push(skill.name.clone()),
            Err(e) => result
                .skipped
                .push((skill.name.clone(), SkipReason::CopyFailed(e.to_string()))),
        }
    }

    Ok(result)
}

/// Copy an AGENTS.md/CLAUDE.md file into imp's config as AGENTS.md.
///
/// Returns the destination path, or None if it already exists.
pub fn import_agents_md(
    source: &DetectedAgentsMd,
    imp_config_dir: &Path,
) -> std::io::Result<Option<PathBuf>> {
    let dest = imp_config_dir.join("AGENTS.md");
    if dest.exists() {
        return Ok(None);
    }

    std::fs::create_dir_all(imp_config_dir)?;
    std::fs::copy(&source.path, &dest)?;
    Ok(Some(dest))
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &dest_path)?;
        } else {
            std::fs::copy(&entry_path, &dest_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_skill(dir: &Path, name: &str, description: &str) {
        let skill_dir = dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n"),
        )
        .unwrap();
    }

    #[test]
    fn detect_pi_skills() {
        let home = TempDir::new().unwrap();
        let skills_dir = home.path().join(".pi").join("agent").join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();
        write_skill(&skills_dir, "rust", "Rust conventions");
        write_skill(&skills_dir, "testing", "Write tests");

        let sources = detect_sources(home.path());
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].agent, AgentSource::Pi);
        assert_eq!(sources[0].skills.len(), 2);

        let names: Vec<&str> = sources[0].skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"rust"));
        assert!(names.contains(&"testing"));
    }

    #[test]
    fn detect_pi_agents_md() {
        let home = TempDir::new().unwrap();
        let agent_dir = home.path().join(".pi").join("agent");
        std::fs::create_dir_all(&agent_dir).unwrap();
        std::fs::write(agent_dir.join("AGENTS.md"), "# Global rules").unwrap();

        let sources = detect_sources(home.path());
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].agents_md.len(), 1);
        assert_eq!(sources[0].agents_md[0].kind, AgentsMdKind::AgentsMd);
    }

    #[test]
    fn detect_claude_code() {
        let home = TempDir::new().unwrap();
        let claude_dir = home.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("CLAUDE.md"), "# Claude config").unwrap();

        let sources = detect_sources(home.path());
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].agent, AgentSource::ClaudeCode);
        assert_eq!(sources[0].agents_md.len(), 1);
        assert_eq!(sources[0].agents_md[0].kind, AgentsMdKind::ClaudeMd);
    }

    #[test]
    fn detect_codex_instructions() {
        let home = TempDir::new().unwrap();
        let codex_dir = home.path().join(".codex");
        std::fs::create_dir_all(&codex_dir).unwrap();
        std::fs::write(codex_dir.join("instructions.md"), "# Codex rules").unwrap();

        let sources = detect_sources(home.path());
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].agent, AgentSource::Codex);
    }

    #[test]
    fn detect_nothing_when_no_agents_installed() {
        let home = TempDir::new().unwrap();
        let sources = detect_sources(home.path());
        assert!(sources.is_empty());
    }

    #[test]
    fn detect_multiple_sources() {
        let home = TempDir::new().unwrap();

        // pi
        let pi_skills = home.path().join(".pi").join("agent").join("skills");
        std::fs::create_dir_all(&pi_skills).unwrap();
        write_skill(&pi_skills, "rust", "Rust");

        // claude
        let claude_dir = home.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("CLAUDE.md"), "config").unwrap();

        let sources = detect_sources(home.path());
        assert_eq!(sources.len(), 2);
    }

    #[test]
    fn import_copies_skills() {
        let home = TempDir::new().unwrap();
        let source_dir = home.path().join("source");
        std::fs::create_dir_all(&source_dir).unwrap();
        write_skill(&source_dir, "rust", "Rust conventions");
        write_skill(&source_dir, "testing", "Write tests");

        let skills = discover_skills_in_dir(&source_dir);
        let dest = home.path().join("imp_skills");

        let result = import_skills(&skills, &dest).unwrap();
        assert_eq!(result.copied.len(), 2);
        assert!(result.skipped.is_empty());

        // Verify files exist
        assert!(dest.join("rust").join("SKILL.md").exists());
        assert!(dest.join("testing").join("SKILL.md").exists());
    }

    #[test]
    fn import_skips_existing() {
        let home = TempDir::new().unwrap();
        let source_dir = home.path().join("source");
        std::fs::create_dir_all(&source_dir).unwrap();
        write_skill(&source_dir, "rust", "Rust conventions");

        let dest = home.path().join("imp_skills");
        // Pre-create the destination
        std::fs::create_dir_all(dest.join("rust")).unwrap();
        std::fs::write(dest.join("rust").join("SKILL.md"), "existing").unwrap();

        let skills = discover_skills_in_dir(&source_dir);
        let result = import_skills(&skills, &dest).unwrap();

        assert!(result.copied.is_empty());
        assert_eq!(result.skipped.len(), 1);
        assert!(matches!(result.skipped[0].1, SkipReason::AlreadyExists));

        // Original content preserved
        let content = std::fs::read_to_string(dest.join("rust").join("SKILL.md")).unwrap();
        assert_eq!(content, "existing");
    }

    #[test]
    fn import_agents_md_copies_file() {
        let home = TempDir::new().unwrap();
        let source = home.path().join("source.md");
        std::fs::write(&source, "# Global rules").unwrap();

        let detected = DetectedAgentsMd {
            path: source,
            kind: AgentsMdKind::AgentsMd,
        };

        let imp_config = home.path().join("config");
        let result = import_agents_md(&detected, &imp_config).unwrap();
        assert!(result.is_some());

        let dest = imp_config.join("AGENTS.md");
        assert!(dest.exists());
        assert_eq!(std::fs::read_to_string(dest).unwrap(), "# Global rules");
    }

    #[test]
    fn import_agents_md_skips_existing() {
        let home = TempDir::new().unwrap();
        let source = home.path().join("source.md");
        std::fs::write(&source, "# New rules").unwrap();

        let imp_config = home.path().join("config");
        std::fs::create_dir_all(&imp_config).unwrap();
        std::fs::write(imp_config.join("AGENTS.md"), "# Existing rules").unwrap();

        let detected = DetectedAgentsMd {
            path: source,
            kind: AgentsMdKind::AgentsMd,
        };

        let result = import_agents_md(&detected, &imp_config).unwrap();
        assert!(result.is_none());

        // Original preserved
        let content = std::fs::read_to_string(imp_config.join("AGENTS.md")).unwrap();
        assert_eq!(content, "# Existing rules");
    }

    #[test]
    fn extract_description_from_frontmatter() {
        let content = "---\nname: test\ndescription: A test skill\n---\n\n# Body\n";
        assert_eq!(extract_skill_description(content), "A test skill");
    }

    #[test]
    fn extract_description_multiline() {
        let content = "---\nname: test\ndescription: >\n  Line one\n  line two\n---\n";
        let desc = extract_skill_description(content);
        assert!(desc.contains("Line one"));
    }

    #[test]
    fn extract_description_no_frontmatter() {
        let content = "# Just a heading\nSome body text.";
        assert_eq!(extract_skill_description(content), "Some body text.");
    }

    #[test]
    fn copy_dir_recursive_works() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");

        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.txt"), "hello").unwrap();
        std::fs::write(src.join("sub").join("b.txt"), "world").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert_eq!(std::fs::read_to_string(dst.join("a.txt")).unwrap(), "hello");
        assert_eq!(
            std::fs::read_to_string(dst.join("sub").join("b.txt")).unwrap(),
            "world"
        );
    }
}
