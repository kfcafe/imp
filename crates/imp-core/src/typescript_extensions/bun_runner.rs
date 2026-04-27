use std::process::Command;

use crate::error::{Error, Result};

use super::discovery::TypeScriptExtension;
use super::pi_compat::BUN_BRIDGE;

pub(super) fn run_bun_bridge(
    extension: &TypeScriptExtension,
    action: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value> {
    ensure_bun_available()?;

    let bridge = std::env::temp_dir().join("imp-ts-extension-bridge.ts");
    std::fs::write(&bridge, BUN_BRIDGE).map_err(Error::from)?;

    let output = Command::new("bun")
        .arg(&bridge)
        .arg(action)
        .arg(&extension.entrypoint)
        .arg(serde_json::to_string(&payload)?)
        .current_dir(&extension.root)
        .output()
        .map_err(Error::from)?;

    if !output.status.success() {
        return Err(Error::Tool(format!(
            "TypeScript extension '{}' failed: {}",
            extension.name,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    serde_json::from_slice(&output.stdout).map_err(Error::from)
}

pub(super) fn ensure_bun_available() -> Result<()> {
    Command::new("bun")
        .arg("--version")
        .output()
        .map(|_| ())
        .map_err(|_| {
            Error::Tool(
                "TypeScript extensions require Bun. Install Bun and ensure `bun` is on PATH."
                    .to_string(),
            )
        })
}
