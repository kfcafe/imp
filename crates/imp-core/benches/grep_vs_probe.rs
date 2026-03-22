//! Benchmark: imp native grep vs probe CLI
//!
//! Run with: cargo bench -p imp-core --bench grep_vs_probe
//!
//! Compares:
//! 1. Line search: imp grep vs probe search (text matching)
//! 2. Block search: imp grep blocks=true vs probe search (tree-sitter extraction)
//! 3. Extract: imp grep extract vs probe extract (block by location)
//!
//! Uses the imp-core source tree as the benchmark corpus.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

fn imp_crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn tower_dir() -> PathBuf {
    imp_crate_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn has_probe() -> bool {
    Command::new("probe")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn has_rg() -> bool {
    Command::new("rg")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

struct BenchResult {
    name: String,
    iterations: usize,
    total: Duration,
    min: Duration,
    max: Duration,
    avg: Duration,
    result_lines: usize,
}

impl std::fmt::Display for BenchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:<40} avg {:>8.2}ms  min {:>8.2}ms  max {:>8.2}ms  ({} iters, ~{} result lines)",
            self.name,
            self.avg.as_secs_f64() * 1000.0,
            self.min.as_secs_f64() * 1000.0,
            self.max.as_secs_f64() * 1000.0,
            self.iterations,
            self.result_lines,
        )
    }
}

fn bench<F>(name: &str, iterations: usize, mut f: F) -> BenchResult
where
    F: FnMut() -> usize, // returns result line count
{
    // Warmup
    let _ = f();

    let mut times = Vec::with_capacity(iterations);
    let mut last_lines = 0;

    for _ in 0..iterations {
        let start = Instant::now();
        last_lines = f();
        times.push(start.elapsed());
    }

    let total: Duration = times.iter().sum();
    let min = *times.iter().min().unwrap();
    let max = *times.iter().max().unwrap();
    let avg = total / iterations as u32;

    BenchResult {
        name: name.to_string(),
        iterations,
        total,
        min,
        max,
        avg,
        result_lines: last_lines,
    }
}

// ── imp native grep ─────────────────────────────────────────────────

fn imp_line_search(pattern: &str, path: &Path) -> usize {
    let re = regex::Regex::new(pattern).unwrap();
    let mut count = 0;

    let walker = ignore::WalkBuilder::new(path)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let bytes = match std::fs::read(entry.path()) {
            Ok(b) => b,
            Err(_) => continue,
        };
        if bytes.contains(&0) {
            continue;
        }
        let content = String::from_utf8_lossy(&bytes);
        for line in content.lines() {
            if re.is_match(line) {
                count += 1;
            }
        }
    }
    count
}

fn imp_block_search(pattern: &str, path: &Path) -> usize {
    let re = regex::Regex::new(pattern).unwrap();
    let mut block_count = 0;

    let walker = ignore::WalkBuilder::new(path)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let file_path = entry.path();
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let language: Option<tree_sitter::Language> = match ext {
            "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
            "ts" | "tsx" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            "py" => Some(tree_sitter_python::LANGUAGE.into()),
            "go" => Some(tree_sitter_go::LANGUAGE.into()),
            _ => None,
        };

        let bytes = match std::fs::read(file_path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        if bytes.contains(&0) {
            continue;
        }
        let content = String::from_utf8_lossy(&bytes);

        let match_lines: Vec<usize> = content
            .lines()
            .enumerate()
            .filter(|(_, line)| re.is_match(line))
            .map(|(idx, _)| idx)
            .collect();

        if match_lines.is_empty() {
            continue;
        }

        if let Some(lang) = language {
            let mut parser = tree_sitter::Parser::new();
            if parser.set_language(&lang).is_ok() {
                if let Some(tree) = parser.parse(content.as_ref(), None) {
                    let mut seen = std::collections::HashSet::new();
                    for &line_idx in &match_lines {
                        if let Some(node) = find_enclosing_block(tree.root_node(), line_idx) {
                            let range = (node.start_position().row, node.end_position().row);
                            if seen.insert(range) {
                                block_count += 1;
                            }
                        }
                    }
                }
            }
        } else {
            block_count += match_lines.len();
        }
    }
    block_count
}

const BLOCK_KINDS: &[&str] = &[
    "function_item",
    "impl_item",
    "struct_item",
    "enum_item",
    "trait_item",
    "mod_item",
    "function_declaration",
    "method_definition",
    "class_declaration",
    "interface_declaration",
    "type_alias_declaration",
    "function_definition",
    "class_definition",
    "decorated_definition",
    "method_declaration",
    "type_declaration",
];

fn find_enclosing_block(root: tree_sitter::Node, target_line: usize) -> Option<tree_sitter::Node> {
    let mut best: Option<tree_sitter::Node> = None;
    find_recursive(root, target_line, &mut best);
    best
}

fn find_recursive<'a>(
    node: tree_sitter::Node<'a>,
    target_line: usize,
    best: &mut Option<tree_sitter::Node<'a>>,
) {
    let start = node.start_position().row;
    let end = node.end_position().row;
    if target_line < start || target_line > end {
        return;
    }
    if BLOCK_KINDS.contains(&node.kind()) {
        *best = Some(node);
    }
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    for child in children {
        find_recursive(child, target_line, best);
    }
}

// ── probe CLI ───────────────────────────────────────────────────────

fn probe_search(pattern: &str, path: &Path) -> usize {
    let output = Command::new("probe")
        .args([
            "search",
            pattern,
            &path.display().to_string(),
            "--format",
            "json",
            "--max-results",
            "100",
        ])
        .output()
        .expect("probe not found");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Count results
    stdout.matches("\"file\"").count()
}

fn probe_extract(path: &Path, target: &str) -> usize {
    let output = Command::new("probe")
        .args(["extract", target, "--format", "json"])
        .current_dir(path)
        .output()
        .expect("probe not found");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.matches("\"file\"").count()
}

// ── rg CLI ──────────────────────────────────────────────────────────

fn rg_search(pattern: &str, path: &Path) -> usize {
    let output = Command::new("rg")
        .args([
            "--no-heading",
            "--line-number",
            "--color=never",
            "--max-count",
            "100",
            "--",
            pattern,
            &path.display().to_string(),
        ])
        .output()
        .expect("rg not found");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().count()
}

// ── main ────────────────────────────────────────────────────────────

fn main() {
    let search_dir = imp_crate_dir().join("src");
    let iters = 3;

    println!("=== Grep Benchmark: imp native vs external tools ===");
    println!("Search directory: {}", search_dir.display());
    println!("Iterations: {iters}");
    println!();

    // ── Line search ──
    println!("── Line Search ──────────────────────────────────────");

    let patterns = &[("simple", "ToolOutput"), ("common", "pub")];

    for (label, pattern) in patterns {
        println!("\nPattern: {pattern} ({label})");

        let imp = bench(&format!("  imp grep (native)"), iters, || {
            imp_line_search(pattern, &search_dir)
        });
        println!("{imp}");

        if has_rg() {
            let rg = bench(&format!("  rg (CLI)"), iters, || {
                rg_search(pattern, &search_dir)
            });
            println!("{rg}");

            let speedup = rg.avg.as_secs_f64() / imp.avg.as_secs_f64();
            if speedup > 1.0 {
                println!("  → imp is {speedup:.1}x faster than rg");
            } else {
                println!("  → rg is {:.1}x faster than imp", 1.0 / speedup);
            }
        }

        if has_probe() {
            let probe = bench(&format!("  probe search (CLI)"), iters, || {
                probe_search(pattern, &search_dir)
            });
            println!("{probe}");

            let speedup = probe.avg.as_secs_f64() / imp.avg.as_secs_f64();
            if speedup > 1.0 {
                println!("  → imp is {speedup:.1}x faster than probe");
            } else {
                println!("  → probe is {:.1}x faster than imp", 1.0 / speedup);
            }
        }
    }

    // ── Block search ──
    println!("\n── Block Search (tree-sitter) ───────────────────────");

    for (label, pattern) in patterns {
        println!("\nPattern: {pattern} ({label})");

        let imp = bench(&format!("  imp grep blocks=true (native)"), iters, || {
            imp_block_search(pattern, &search_dir)
        });
        println!("{imp}");

        if has_probe() {
            let probe = bench(&format!("  probe search (CLI)"), iters, || {
                probe_search(pattern, &search_dir)
            });
            println!("{probe}");

            let speedup = probe.avg.as_secs_f64() / imp.avg.as_secs_f64();
            if speedup > 1.0 {
                println!("  → imp is {speedup:.1}x faster than probe");
            } else {
                println!("  → probe is {:.1}x faster than imp", 1.0 / speedup);
            }
        }
    }

    // ── Extract ──
    println!("\n── Extract (block at location) ─────────────────────");

    let extract_target = "src/tools/grep.rs:50";
    let full_target = format!("{}/tools/grep.rs:50", search_dir.display());
    println!("\nTarget: {extract_target}");

    let imp_extract = bench("  imp grep extract (native)", iters, || {
        // Simulate extract: read file, parse, find block
        let path = search_dir.join("tools/grep.rs");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        if let Some(tree) = parser.parse(&content, None) {
            if find_enclosing_block(tree.root_node(), 49).is_some() {
                1
            } else {
                0
            }
        } else {
            0
        }
    });
    println!("{imp_extract}");

    if has_probe() {
        let probe_ext = bench("  probe extract (CLI)", iters, || {
            probe_extract(&search_dir, &full_target)
        });
        println!("{probe_ext}");

        let speedup = probe_ext.avg.as_secs_f64() / imp_extract.avg.as_secs_f64();
        if speedup > 1.0 {
            println!("  → imp is {speedup:.1}x faster than probe");
        } else {
            println!("  → probe is {:.1}x faster than imp", 1.0 / speedup);
        }
    }

    println!("\n=== Done ===");
}
