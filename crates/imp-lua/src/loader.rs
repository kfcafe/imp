use std::path::{Path, PathBuf};

use crate::sandbox::{LuaError, LuaRuntime};

/// Discovered Lua extension.
#[derive(Debug, Clone)]
pub struct LuaExtension {
    pub name: String,
    pub path: PathBuf,
}

/// Discover Lua extensions from user and project directories.
pub fn discover_extensions(
    user_config_dir: &Path,
    project_dir: Option<&Path>,
) -> Vec<LuaExtension> {
    let mut extensions = Vec::new();

    let mut dirs = vec![user_config_dir.join("lua")];
    if let Some(project) = project_dir {
        dirs.push(project.join(".imp").join("lua"));
    }

    for dir in &dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Direct .lua file
                if path.extension().is_some_and(|e| e == "lua") {
                    let name = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    extensions.push(LuaExtension { name, path });
                    continue;
                }

                // Directory with init.lua
                if path.is_dir() {
                    let init = path.join("init.lua");
                    if init.exists() {
                        let name = path
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        extensions.push(LuaExtension { name, path: init });
                    }
                }
            }
        }
    }

    extensions
}

/// Load all discovered extensions into a Lua runtime.
pub fn load_extensions(
    runtime: &LuaRuntime,
    extensions: &[LuaExtension],
) -> Vec<(String, Result<(), LuaError>)> {
    extensions
        .iter()
        .map(|ext| {
            let result = runtime.exec_file(&ext.path);
            (ext.name.clone(), result)
        })
        .collect()
}

/// Hot reload: drop old state, create new runtime, re-load extensions.
pub fn reload(
    user_config_dir: &Path,
    project_dir: Option<&Path>,
) -> Result<(LuaRuntime, Vec<LuaExtension>), LuaError> {
    let extensions = discover_extensions(user_config_dir, project_dir);
    let runtime = LuaRuntime::new()?;
    crate::bridge::setup_host_api(&runtime)?;
    load_extensions(&runtime, &extensions);
    Ok((runtime, extensions))
}
