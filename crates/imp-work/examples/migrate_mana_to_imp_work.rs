use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use imp_work::{dry_run_mana_migration, migrate_mana_to_store, WorkStore};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let mut project_root = env::current_dir().map_err(|error| error.to_string())?;
    let mut write = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dry-run" => write = false,
            "--write" => write = true,
            "--project-root" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--project-root requires a path".to_string())?;
                project_root = PathBuf::from(value);
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            other => return Err(format!("unsupported argument `{other}`; use --help")),
        }
    }

    let project_root = project_root
        .canonicalize()
        .map_err(|error| format!("failed to canonicalize project root: {error}"))?;
    let mana_path = project_root.join(".mana");
    let global_root = global_work_root()?.join("projects").join(project_hash(&project_root));
    let report = if write {
        let store = WorkStore::open(&global_root);
        migrate_mana_to_store(&mana_path, &store)
            .map_err(|error| format!("failed to migrate mana into imp-work: {error}"))?
    } else {
        dry_run_mana_migration(&mana_path)
            .map_err(|error| format!("failed to dry-run mana migration: {error}"))?
    };

    println!(
        "{} mana -> imp-work migration",
        if write { "Completed" } else { "Dry-run" }
    );
    println!("project_root: {}", project_root.display());
    println!("mana_path: {}", mana_path.display());
    println!("target_global_project_store: {}", global_root.display());
    println!("imported: {}", report.imported.len());
    println!("skipped_files: {}", report.skipped_files.len());
    println!("warnings: {}", report.warnings.len());
    for warning in &report.warnings {
        println!("warning: {warning}");
    }
    for skipped in &report.skipped_files {
        println!("skipped: {}", skipped.display());
    }
    if !write {
        println!("dry-run only; rerun with --write to import into imp-work");
    }
    Ok(())
}

fn print_help() {
    println!("migrate-mana-to-imp-work [--dry-run|--write] [--project-root PATH]");
    println!("\nMigrates .mana units into the global imp-work project store.");
    println!("Defaults to --dry-run and the current directory as project root.");
}

fn global_work_root() -> Result<PathBuf, String> {
    let home = env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".imp").join("work"))
}

fn project_hash(project_root: &PathBuf) -> String {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    project_root.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
