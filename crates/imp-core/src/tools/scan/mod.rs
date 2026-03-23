//! Scan tool — extract code structure using tree-sitter AST parsing.
//!
//! Dispatches to language-specific parsers based on file extension.
//! Produces rich output: visibility, signatures, fields, variants, trait impls.

pub mod go;
pub mod python;
pub mod rust;
pub mod types;
pub mod typescript;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::json;

use super::{truncate_head, truncate_line, Tool, ToolContext, ToolOutput, TruncationResult};
use crate::error::{Error, Result};
use types::*;

const MAX_OUTPUT_LINES: usize = 2000;
const MAX_OUTPUT_BYTES: usize = 50 * 1024;
const MAX_LINE_CHARS: usize = 500;

pub struct ScanTool;

#[async_trait]
impl Tool for ScanTool {
    fn name(&self) -> &str {
        "scan"
    }

    fn label(&self) -> &str {
        "Scan Code Structure"
    }

    fn description(&self) -> &str {
        "Extract code structure (types, functions, impls) via tree-sitter. Shows visibility, signatures, fields, variants."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["extract", "build", "scan"] },
                "files": { "type": "array", "items": { "type": "string" } },
                "directory": { "type": "string" },
                "task": { "type": "string", "description": "Context for relevance hints" }
            },
            "required": ["action"]
        })
    }

    fn is_readonly(&self) -> bool {
        true
    }

    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let action = match params["action"].as_str() {
            Some(a) => a,
            None => return Ok(ToolOutput::error("missing 'action' parameter")),
        };

        let mut files = match action {
            "extract" | "build" => {
                let files = match params["files"].as_array() {
                    Some(f) if !f.is_empty() => f,
                    _ => {
                        return Ok(ToolOutput::error(
                            "'files' array required for extract/build actions",
                        ))
                    }
                };
                let mut resolved = Vec::with_capacity(files.len());
                for file in files {
                    match file.as_str() {
                        Some(f) => resolved.push(ctx.cwd.join(f)),
                        None => return Ok(ToolOutput::error("'files' must contain strings")),
                    }
                }
                resolved
            }
            "scan" => {
                let dir = params["directory"]
                    .as_str()
                    .map(|d| ctx.cwd.join(d))
                    .unwrap_or_else(|| ctx.cwd.clone());
                collect_source_files(&dir)?
            }
            _ => return Ok(ToolOutput::error(format!("unknown action: {action}"))),
        };

        files.sort();
        files.dedup();

        if files.is_empty() {
            return Ok(ToolOutput::text("No supported source files found."));
        }

        let result = extract_files(&files, &ctx.cwd);
        let task = params["task"].as_str();
        let output = format_result(&result, &files, &ctx.cwd, action, task);

        Ok(ToolOutput::text(truncate_output(output)))
    }
}

// ── extraction dispatch ─────────────────────────────────────────────

fn extract_files(files: &[PathBuf], cwd: &Path) -> ScanResult {
    let mut result = ScanResult::default();

    for file in files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Skip binary files
        if source.as_bytes().contains(&0) {
            continue;
        }

        let rel = file
            .strip_prefix(cwd)
            .unwrap_or(file)
            .to_string_lossy()
            .to_string();

        let ext = file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();

        match ext {
            "rs" => rust::parse(&source, &rel, &mut result),
            "ts" => {
                if !rel.ends_with(".d.ts") {
                    typescript::parse(&source, &rel, false, &mut result);
                }
            }
            "tsx" => typescript::parse(&source, &rel, true, &mut result),
            "py" => python::parse(&source, &rel, &mut result),
            "go" => go::parse(&source, &rel, &mut result),
            // TODO: add more languages as tree-sitter grammars are added
            _ => {}
        }
    }

    result
}

// ── file collection ─────────────────────────────────────────────────

fn collect_source_files(root: &Path) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(if is_supported(root) {
            vec![root.to_path_buf()]
        } else {
            Vec::new()
        });
    }

    if !root.exists() {
        return Err(Error::Tool(format!(
            "scan path not found: {}",
            root.display()
        )));
    }

    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| !is_skip_dir(e.path()))
    {
        if is_supported(entry.path()) {
            files.push(entry.path().to_path_buf());
        }
    }

    Ok(files)
}

fn is_supported(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("rs" | "ts" | "tsx" | "py" | "go")
    )
}

fn is_skip_dir(path: &Path) -> bool {
    const SKIP: &[&str] = &[
        "target",
        "node_modules",
        ".git",
        "__pycache__",
        ".venv",
        "venv",
        "vendor",
        "dist",
        "build",
        ".next",
        "coverage",
    ];
    path.components().any(|c| {
        if let std::path::Component::Normal(name) = c {
            SKIP.contains(&name.to_string_lossy().as_ref())
        } else {
            false
        }
    })
}

// ── formatting ──────────────────────────────────────────────────────

fn format_result(
    result: &ScanResult,
    files: &[PathBuf],
    cwd: &Path,
    action: &str,
    task: Option<&str>,
) -> String {
    let mut sections = Vec::new();
    sections.push(format!("Action: {action}"));
    if let Some(task) = task {
        sections.push(format!("Task: {task}"));
    }
    sections.push(format!("Files analyzed: {}", files.len()));

    // Group types and functions by source file
    let mut file_types: BTreeMap<&str, Vec<&TypeInfo>> = BTreeMap::new();
    let mut file_functions: BTreeMap<&str, Vec<&FunctionInfo>> = BTreeMap::new();

    for t in result.types.values() {
        let file = source_file(&t.source);
        file_types.entry(file).or_default().push(t);
    }

    for f in result.functions.values() {
        let file = source_file(&f.source);
        file_functions.entry(file).or_default().push(f);
    }

    let all_files: BTreeSet<&str> = file_types
        .keys()
        .chain(file_functions.keys())
        .copied()
        .collect();

    for file in &all_files {
        let rel = display_path(file, cwd);
        let mut lines = vec![rel];

        if let Some(types) = file_types.get(file) {
            lines.push(format!("  Types ({}):", types.len()));
            for t in types {
                lines.push(format!("    - {}", format_type(t)));
            }
        }

        if let Some(funcs) = file_functions.get(file) {
            // Standalone functions only (not Type::method — those show under Types)
            let standalone: Vec<_> = funcs
                .iter()
                .filter(|f| !f.name.contains("::") && !is_qualified_name(&f.name))
                .filter(|f| !f.is_test)
                .collect();
            if !standalone.is_empty() {
                lines.push(format!("  Functions ({}):", standalone.len()));
                for f in standalone {
                    lines.push(format!("    - {}", format_function(f)));
                }
            }
        }

        if lines.len() > 1 {
            sections.push(lines.join("\n"));
        }
    }

    sections.join("\n\n")
}

fn format_type(t: &TypeInfo) -> String {
    let vis = format_visibility(&t.visibility);
    let kind = match t.kind {
        TypeKind::Struct => "struct",
        TypeKind::Enum => "enum",
        TypeKind::Trait => "trait",
        TypeKind::Interface => "interface",
        TypeKind::Class => "class",
        TypeKind::TypeAlias => "type",
        TypeKind::Union => "union",
        TypeKind::Protocol => "protocol",
    };

    let mut out = format!("{vis}{kind} {}", t.name);

    match t.kind {
        TypeKind::Struct | TypeKind::Class => {
            if !t.fields.is_empty() {
                let names: Vec<&str> = t.fields.iter().map(|f| f.name.as_str()).collect();
                if names.len() <= 6 {
                    out.push_str(&format!(" {{ {} }}", names.join(", ")));
                } else {
                    let shown = &names[..5];
                    out.push_str(&format!(
                        " {{ {}, ... +{} }}",
                        shown.join(", "),
                        names.len() - 5
                    ));
                }
            }
        }
        TypeKind::Enum => {
            if !t.variants.is_empty() {
                if t.variants.len() <= 6 {
                    out.push_str(&format!(" {{ {} }}", t.variants.join(", ")));
                } else {
                    let shown: Vec<&str> = t.variants[..5].iter().map(|s| s.as_str()).collect();
                    out.push_str(&format!(
                        " {{ {}, ... +{} }}",
                        shown.join(", "),
                        t.variants.len() - 5
                    ));
                }
            }
        }
        TypeKind::Trait | TypeKind::Interface | TypeKind::Protocol => {
            if !t.methods.is_empty() {
                if t.methods.len() <= 6 {
                    out.push_str(&format!(" {{ {} }}", t.methods.join(", ")));
                } else {
                    let shown: Vec<&str> = t.methods[..5].iter().map(|s| s.as_str()).collect();
                    out.push_str(&format!(
                        " {{ {}, ... +{} }}",
                        shown.join(", "),
                        t.methods.len() - 5
                    ));
                }
            }
        }
        _ => {}
    }

    if !t.implements.is_empty() {
        out.push_str(&format!(" [{}]", t.implements.join(", ")));
    }

    out
}

fn format_function(f: &FunctionInfo) -> String {
    let vis = format_visibility(&f.visibility);
    if !f.signature.is_empty() {
        format!("{vis}{}", f.signature)
    } else {
        format!("{vis}fn {}", f.name)
    }
}

fn format_visibility(vis: &Visibility) -> &'static str {
    match vis {
        Visibility::Public => "pub ",
        Visibility::Internal => "pub(crate) ",
        Visibility::Private => "",
    }
}

fn source_file(source: &str) -> &str {
    // "src/lib.rs:42" → "src/lib.rs"
    source.rsplit_once(':').map(|(f, _)| f).unwrap_or(source)
}

fn display_path(path: &str, cwd: &Path) -> String {
    let cwd_str = cwd.to_string_lossy();
    path.strip_prefix(cwd_str.as_ref())
        .map(|p| p.strip_prefix('/').unwrap_or(p))
        .unwrap_or(path)
        .to_string()
}

fn is_qualified_name(name: &str) -> bool {
    // "Type::method" or "module.function" patterns
    name.contains("::")
}

fn truncate_output(text: String) -> String {
    if text.is_empty() {
        return text;
    }

    let truncated_lines = text
        .lines()
        .map(|line| truncate_line(line, MAX_LINE_CHARS))
        .collect::<Vec<_>>()
        .join("\n");

    let TruncationResult {
        content,
        truncated,
        output_lines,
        total_lines,
        temp_file,
        ..
    } = truncate_head(&truncated_lines, MAX_OUTPUT_LINES, MAX_OUTPUT_BYTES);

    if !truncated {
        return content;
    }

    let mut result = content;
    result.push_str(&format!(
        "\n[Output truncated: showing first {output_lines} of {total_lines} lines{}]",
        temp_file
            .as_ref()
            .map(|p| format!(". Full output saved to {}", p.display()))
            .unwrap_or_default()
    ));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_rust_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("sample.rs");
        std::fs::write(
            &file,
            r#"
pub struct User {
    pub name: String,
    pub age: u32,
}

pub enum Status { Active, Inactive }

pub trait Validate {
    fn validate(&self) -> bool;
}

impl Validate for User {
    fn validate(&self) -> bool { true }
}

pub async fn load_user(id: &str) -> Result<User> { todo!() }
fn internal_helper() {}
"#,
        )
        .unwrap();

        let result = extract_files(&[file], tmp.path());

        // Types extracted
        assert!(result.types.contains_key("User"));
        assert!(result.types.contains_key("Status"));
        assert!(result.types.contains_key("Validate"));

        // User has fields
        let user = &result.types["User"];
        assert_eq!(user.fields.len(), 2);
        assert_eq!(user.visibility, Visibility::Public);

        // Status has variants
        let status = &result.types["Status"];
        assert_eq!(status.variants, vec!["Active", "Inactive"]);

        // Validate has methods
        let validate = &result.types["Validate"];
        assert!(validate.methods.contains(&"validate".to_string()));

        // User implements Validate
        assert!(user.implements.contains(&"Validate".to_string()));

        // Functions extracted with signatures
        let load = &result.functions["load_user"];
        assert!(load.is_async);
        assert!(load.signature.contains("-> Result<User>"));
        assert_eq!(load.visibility, Visibility::Public);

        let helper = &result.functions["internal_helper"];
        assert_eq!(helper.visibility, Visibility::Private);
    }

    #[test]
    fn extract_typescript_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("models.ts");
        std::fs::write(
            &file,
            r#"
export interface User {
    name: string;
    email: string;
}

export enum Status {
    Active = "active",
    Inactive = "inactive",
}

export async function fetchUser(id: string): Promise<User> {
    return {} as User;
}

function internalHelper(): void {}
"#,
        )
        .unwrap();

        let result = extract_files(&[file], tmp.path());
        assert!(result.types.contains_key("User"));
        assert!(result.types.contains_key("Status"));
        assert_eq!(result.types["User"].visibility, Visibility::Public);
        assert_eq!(result.types["Status"].variants, vec!["Active", "Inactive"]);
        assert!(result.functions["fetchUser"].is_async);
        assert_eq!(
            result.functions["internalHelper"].visibility,
            Visibility::Private
        );
    }

    #[test]
    fn format_output_shows_rich_info() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("lib.rs");
        std::fs::write(
            &file,
            r#"
pub struct Config { pub host: String, pub port: u16 }
pub enum Mode { Debug, Release }
pub fn start(config: &Config) -> Result<()> { todo!() }
"#,
        )
        .unwrap();

        let result = extract_files(&[file.clone()], tmp.path());
        let output = format_result(&result, &[file], tmp.path(), "extract", None);

        assert!(output.contains("pub struct Config { host, port }"));
        assert!(output.contains("pub enum Mode { Debug, Release }"));
        assert!(output.contains("pub fn start"));
        assert!(output.contains("-> Result<()>"));
    }
}
