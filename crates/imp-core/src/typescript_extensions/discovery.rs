use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeScriptExtension {
    pub name: String,
    pub root: PathBuf,
    pub entrypoint: PathBuf,
    pub source: TypeScriptExtensionSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeScriptExtensionSource {
    Local,
    Imported(String),
}

#[derive(Debug, Deserialize)]
struct PackageJson {
    pi: Option<PiPackage>,
}

#[derive(Debug, Deserialize)]
struct PiPackage {
    extensions: Option<Vec<String>>,
}

pub fn discover_typescript_extensions(project_dir: &Path) -> Vec<TypeScriptExtension> {
    discover_typescript_extensions_in(&project_dir.join(".imp").join("extensions"))
}

pub fn discover_typescript_extensions_in(root: &Path) -> Vec<TypeScriptExtension> {
    let mut extensions = Vec::new();
    discover_extension_entries(root, TypeScriptExtensionSource::Local, &mut extensions);
    extensions.sort_by(|a, b| a.name.cmp(&b.name));
    extensions
}

fn discover_extension_entries(
    dir: &Path,
    source: TypeScriptExtensionSource,
    extensions: &mut Vec<TypeScriptExtension>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if path.extension().and_then(|ext| ext.to_str()) == Some("ts") {
                if let Some(name) = file_stem_string(&path) {
                    extensions.push(TypeScriptExtension {
                        name,
                        root: dir.to_path_buf(),
                        entrypoint: path,
                        source: source.clone(),
                    });
                }
            }
            continue;
        }

        if !path.is_dir() {
            continue;
        }

        if let Some(extension) = detect_package_dir(&path, source.clone()) {
            extensions.push(extension);
        } else if matches!(source, TypeScriptExtensionSource::Local) {
            let Some(namespace) = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
            else {
                continue;
            };
            discover_extension_entries(
                &path,
                TypeScriptExtensionSource::Imported(namespace),
                extensions,
            );
        }
    }
}

fn detect_package_dir(
    dir: &Path,
    source: TypeScriptExtensionSource,
) -> Option<TypeScriptExtension> {
    let name = dir.file_name()?.to_string_lossy().to_string();
    let package_json = dir.join("package.json");

    if package_json.exists() {
        if let Ok(content) = std::fs::read_to_string(&package_json) {
            if let Ok(package) = serde_json::from_str::<PackageJson>(&content) {
                if let Some(entrypoint) = package
                    .pi
                    .and_then(|pi| pi.extensions)
                    .and_then(|mut extensions| extensions.drain(..).next())
                {
                    return Some(TypeScriptExtension {
                        name,
                        root: dir.to_path_buf(),
                        entrypoint: dir.join(entrypoint),
                        source,
                    });
                }
            }
        }
    }

    let entrypoint = dir.join("index.ts");
    entrypoint.exists().then_some(TypeScriptExtension {
        name,
        root: dir.to_path_buf(),
        entrypoint,
        source,
    })
}

fn file_stem_string(path: &Path) -> Option<String> {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .filter(|stem| !stem.is_empty())
}
