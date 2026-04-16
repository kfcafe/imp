use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_TARGET_DIR");
    println!("cargo:rerun-if-env-changed=UU_INSTALL");

    if !cfg!(target_os = "macos") {
        return;
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| Path::new(&manifest_dir).join("target"));
    let binary_path = target_dir.join("debug/imp");

    if !binary_path.exists() {
        return;
    }

    let status = Command::new("codesign")
        .args(["--force", "--sign", "-"])
        .arg(&binary_path)
        .status();

    match status {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!(
            "warning: post-build codesign for {} exited with status {status}",
            binary_path.display()
        ),
        Err(err) => eprintln!(
            "warning: failed to invoke codesign for {}: {err}",
            binary_path.display()
        ),
    }
}
