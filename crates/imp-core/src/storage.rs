use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const IMP_DIR_NAME: &str = ".imp";
const LEGACY_APP_NAME: &str = "imp";

pub fn global_root() -> PathBuf {
    global_root_from_env(std::env::var_os("HOME"), std::env::var_os("USERPROFILE"))
}

fn global_root_from_env(
    home: Option<std::ffi::OsString>,
    userprofile: Option<std::ffi::OsString>,
) -> PathBuf {
    home.or(userprofile)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(IMP_DIR_NAME)
}

pub fn project_root(project_dir: &Path) -> PathBuf {
    project_dir.join(IMP_DIR_NAME)
}

pub fn global_config_path() -> PathBuf {
    global_root().join("config.toml")
}

pub fn global_auth_path() -> PathBuf {
    global_root().join("auth.json")
}

pub fn global_soul_path() -> PathBuf {
    global_root().join("soul.md")
}

pub fn global_agents_path() -> PathBuf {
    global_root().join("agents.md")
}

pub fn global_memory_path() -> PathBuf {
    global_root().join("memory.md")
}

pub fn global_user_path() -> PathBuf {
    global_root().join("user.md")
}

pub fn global_sessions_dir() -> PathBuf {
    global_root().join("sessions")
}

pub fn global_runs_dir() -> PathBuf {
    global_root().join("runs")
}

pub fn global_run_index_path() -> PathBuf {
    global_runs_dir().join("index.jsonl")
}

pub fn global_indexes_dir() -> PathBuf {
    global_root().join("indexes")
}

pub fn global_session_index_path() -> PathBuf {
    global_indexes_dir().join("session_index.db")
}

pub fn global_skills_dir() -> PathBuf {
    global_root().join("skills")
}

pub fn global_prompts_dir() -> PathBuf {
    global_root().join("prompts")
}

pub fn global_tools_dir() -> PathBuf {
    global_root().join("tools")
}

pub fn global_lua_dir() -> PathBuf {
    global_root().join("lua")
}

pub fn global_imports_dir() -> PathBuf {
    global_root().join("imports")
}

pub fn project_config_path(project_dir: &Path) -> PathBuf {
    project_root(project_dir).join("config.toml")
}

pub fn project_soul_path(project_dir: &Path) -> PathBuf {
    project_root(project_dir).join("soul.md")
}

pub fn project_agents_path(project_dir: &Path) -> PathBuf {
    project_root(project_dir).join("agents.md")
}

pub fn project_skills_dir(project_dir: &Path) -> PathBuf {
    project_root(project_dir).join("skills")
}

pub fn project_prompts_dir(project_dir: &Path) -> PathBuf {
    project_root(project_dir).join("prompts")
}

pub fn project_tools_dir(project_dir: &Path) -> PathBuf {
    project_root(project_dir).join("tools")
}

pub fn project_lua_dir(project_dir: &Path) -> PathBuf {
    project_root(project_dir).join("lua")
}

pub fn legacy_config_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(root) = xdg_config_root() {
        roots.push(root);
    }
    dedupe(roots)
}

pub fn legacy_data_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(root) = xdg_data_root() {
        roots.push(root);
    }
    if cfg!(target_os = "macos") {
        if let Some(root) = macos_application_support_root() {
            roots.push(root);
        }
    }
    dedupe(roots)
}

pub fn global_config_roots_for_read() -> Vec<PathBuf> {
    let mut roots = vec![global_root()];
    roots.extend(legacy_config_roots());
    dedupe(roots)
}

pub fn global_data_roots_for_read() -> Vec<PathBuf> {
    let mut roots = vec![global_root()];
    roots.extend(legacy_data_roots());
    dedupe(roots)
}

pub fn existing_global_file(path_fn: fn() -> PathBuf, legacy_subpath: &str) -> Option<PathBuf> {
    let canonical = path_fn();
    if canonical.exists() {
        return Some(canonical);
    }

    for root in global_config_roots_for_read() {
        let path = root.join(legacy_subpath);
        if path.exists() {
            return Some(path);
        }
    }

    for root in global_data_roots_for_read() {
        let path = root.join(legacy_subpath);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

pub fn existing_global_auth_path() -> Option<PathBuf> {
    let canonical = global_auth_path();
    if canonical.exists() {
        return Some(canonical);
    }
    legacy_config_roots()
        .into_iter()
        .map(|root| root.join("auth.json"))
        .find(|path| path.exists())
}

pub fn existing_global_config_path() -> Option<PathBuf> {
    let canonical = global_config_path();
    if canonical.exists() {
        return Some(canonical);
    }
    legacy_config_roots()
        .into_iter()
        .map(|root| root.join("config.toml"))
        .find(|path| path.exists())
}

pub fn reconcile_legacy_into_global_root() -> io::Result<Vec<PathBuf>> {
    let mut migrated = Vec::new();

    migrated.extend(reconcile_file_candidates(
        global_config_path(),
        legacy_config_roots()
            .into_iter()
            .map(|root| root.join("config.toml"))
            .collect(),
    )?);
    migrated.extend(reconcile_file_candidates(
        global_auth_path(),
        legacy_config_roots()
            .into_iter()
            .map(|root| root.join("auth.json"))
            .collect(),
    )?);
    migrated.extend(reconcile_file_candidates(
        global_soul_path(),
        legacy_config_roots()
            .into_iter()
            .map(|root| root.join("soul.md"))
            .collect(),
    )?);
    migrated.extend(reconcile_file_candidates(
        global_memory_path(),
        legacy_config_roots()
            .into_iter()
            .map(|root| root.join("memory.md"))
            .collect(),
    )?);
    migrated.extend(reconcile_file_candidates(
        global_user_path(),
        legacy_config_roots()
            .into_iter()
            .map(|root| root.join("user.md"))
            .collect(),
    )?);
    migrated.extend(reconcile_file_candidates(
        global_agents_path(),
        legacy_config_roots()
            .into_iter()
            .flat_map(|root| {
                [
                    root.join("agents.md"),
                    root.join("AGENTS.md"),
                    root.join("CLAUDE.md"),
                ]
            })
            .collect(),
    )?);
    migrated.extend(reconcile_dir_candidates(
        global_skills_dir(),
        legacy_config_roots()
            .into_iter()
            .map(|root| root.join("skills"))
            .collect(),
    )?);
    migrated.extend(reconcile_dir_candidates(
        global_prompts_dir(),
        legacy_config_roots()
            .into_iter()
            .map(|root| root.join("prompts"))
            .collect(),
    )?);
    migrated.extend(reconcile_dir_candidates(
        global_tools_dir(),
        legacy_config_roots()
            .into_iter()
            .map(|root| root.join("tools"))
            .collect(),
    )?);
    migrated.extend(reconcile_dir_candidates(
        global_lua_dir(),
        legacy_config_roots()
            .into_iter()
            .map(|root| root.join("lua"))
            .collect(),
    )?);
    migrated.extend(reconcile_dir_candidates(
        global_sessions_dir(),
        legacy_data_roots()
            .into_iter()
            .map(|root| root.join("sessions"))
            .collect(),
    )?);
    migrated.extend(reconcile_file_candidates(
        global_session_index_path(),
        legacy_data_roots()
            .into_iter()
            .flat_map(|root| {
                [
                    root.join("indexes").join("session_index.db"),
                    root.join("session_index.db"),
                ]
            })
            .collect(),
    )?);

    Ok(migrated)
}

fn reconcile_file_candidates(
    target: PathBuf,
    candidates: Vec<PathBuf>,
) -> io::Result<Vec<PathBuf>> {
    if target.exists() {
        return Ok(Vec::new());
    }

    let Some(source) = candidates.into_iter().find(|path| path.exists()) else {
        return Ok(Vec::new());
    };

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&source, &target)?;
    Ok(vec![target])
}

fn reconcile_dir_candidates(target: PathBuf, candidates: Vec<PathBuf>) -> io::Result<Vec<PathBuf>> {
    if target.exists() {
        return Ok(Vec::new());
    }

    let Some(source) = candidates.into_iter().find(|path| path.exists()) else {
        return Ok(Vec::new());
    };

    copy_dir_recursive(&source, &target)?;
    Ok(vec![target])
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &dest_path)?;
        } else if !dest_path.exists() {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&entry_path, &dest_path)?;
        }
    }

    Ok(())
}

fn xdg_config_root() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(dir).join(LEGACY_APP_NAME));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config").join(LEGACY_APP_NAME))
}

fn xdg_data_root() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
        return Some(PathBuf::from(dir).join(LEGACY_APP_NAME));
    }
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join(LEGACY_APP_NAME)
    })
}

fn macos_application_support_root() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join(LEGACY_APP_NAME)
    })
}

fn dedupe(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut deduped = Vec::new();
    for path in paths {
        if !deduped.iter().any(|existing| existing == &path) {
            deduped.push(path);
        }
    }
    deduped
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn global_root_prefers_home_imp_directory() {
        let path = global_root_from_env(Some("/tmp/home".into()), None);
        assert_eq!(path, PathBuf::from("/tmp/home/.imp"));
    }

    #[test]
    fn global_root_falls_back_to_userprofile_when_home_missing() {
        let path = global_root_from_env(None, Some("C:/Users/test".into()));
        assert_eq!(path, PathBuf::from("C:/Users/test/.imp"));
    }

    #[test]
    fn project_root_uses_dot_imp_directory() {
        assert_eq!(
            project_root(Path::new("/tmp/project")),
            PathBuf::from("/tmp/project/.imp")
        );
    }

    #[test]
    fn global_session_index_lives_under_indexes() {
        let old_home = std::env::var_os("HOME");
        std::env::set_var("HOME", "/tmp/home");
        assert_eq!(
            global_session_index_path(),
            PathBuf::from("/tmp/home/.imp/indexes/session_index.db")
        );
        match old_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
    }

    #[test]
    fn reconcile_file_candidates_copies_first_existing_legacy_file() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join(".imp").join("config.toml");
        let legacy = temp.path().join("legacy").join("config.toml");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::write(&legacy, "model = \"sonnet\"\n").unwrap();

        let migrated = reconcile_file_candidates(target.clone(), vec![legacy.clone()]).unwrap();
        assert_eq!(migrated, vec![target.clone()]);
        assert_eq!(fs::read_to_string(target).unwrap(), "model = \"sonnet\"\n");
    }

    #[test]
    fn reconcile_dir_candidates_copies_directory_tree_when_target_missing() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join(".imp").join("skills");
        let legacy = temp.path().join("legacy").join("skills").join("my-skill");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("SKILL.md"), "# Skill\n").unwrap();

        let migrated = reconcile_dir_candidates(
            target.clone(),
            vec![temp.path().join("legacy").join("skills")],
        )
        .unwrap();
        assert_eq!(migrated, vec![target.clone()]);
        assert!(target.join("my-skill").join("SKILL.md").exists());
    }
}
