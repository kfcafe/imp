fn main() {
    let mut command = std::process::Command::new("imp");

    // If this shim is installed as `mcp` beside an imp binary that is also an
    // mcp shim (for example during local cargo installs), blindly resolving
    // `imp` through PATH can recurse forever. Prefer the workspace imp binary
    // in Cargo's target dir when running from `cargo run -p mcp-shim`, then
    // fall back to PATH for normal installs.
    if let Some(imp_path) = sibling_imp_binary() {
        command = std::process::Command::new(imp_path);
    }

    command.arg("mcp");
    command.args(std::env::args_os().skip(1));

    match command.status() {
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),
        Err(err) => {
            eprintln!("failed to run imp mcp: {err}");
            std::process::exit(127);
        }
    }
}

fn sibling_imp_binary() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let candidate = dir.join("imp");
    if candidate == exe || !candidate.is_file() {
        return None;
    }
    Some(candidate)
}
