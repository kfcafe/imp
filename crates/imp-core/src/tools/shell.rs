use std::path::Path;

use crate::error::Result;
use crate::tools::ToolRegistry;

/// TOML-defined shell tool definition.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ShellToolDef {
    pub name: String,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub readonly: bool,
    #[serde(default)]
    pub params: std::collections::HashMap<String, ShellParamDef>,
    pub exec: ShellExecDef,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ShellParamDef {
    #[serde(rename = "type")]
    pub param_type: String,
    pub description: String,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ShellExecDef {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout: u32,
    #[serde(default = "default_truncate")]
    pub truncate: String,
    pub install_hint: Option<String>,
}

fn default_timeout() -> u32 { 30 }
fn default_truncate() -> String { "head".into() }

/// Load shell tools from a directory of TOML definitions.
pub fn load_shell_tools(_dir: &Path, _registry: &mut ToolRegistry) -> Result<()> {
    // TODO: Walk dir, parse TOML files, create ShellTool instances, register them
    Ok(())
}
