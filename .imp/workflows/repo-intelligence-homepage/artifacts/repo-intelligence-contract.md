# Repo intelligence contract

Repo intelligence is a lightweight, best-effort project snapshot used by the imp homepage, scan tool, and agent context assembly. It should stay local, fast, explainable, and swappable: the UI and tools consume a stable summary contract rather than depending on scanner internals.

## Version 1 scope

In scope:

- source inventory from the vendored whatlang-style scanner
- primary language, total nonblank LOC, and source file count
- tree-sitter-backed symbol and test counts using existing scan extractors
- safe launch states for home directory, non-repo cwd, empty repo, and failed scan
- compact homepage display
- reusable core entry point for scan/tool/context callers

Out of scope for v1:

- embeddings
- LSP lifecycle/integration
- daemon or watcher
- complex persistent cache
- exact type-aware reference graph
- exact comment-stripped LOC parity with tokei

## Current implemented contract

```rust
pub struct RepoStats {
    pub primary_language: String,
    pub code_lines: u64,
    pub files: u64,
    pub symbols: Option<usize>,
    pub tests: Option<usize>,
}
```

`RepoStats` currently lives in `imp-tui` because it feeds the homepage directly. It is backed by the vendored scanner plus `imp_core::repo_intelligence`.

```rust
pub struct RepoIndexSummary {
    pub symbols: usize,
    pub tests: usize,
}

pub fn index_repo(root: &Path) -> crate::Result<RepoIndexSummary>;
pub fn index_files(files: &[PathBuf], root: &Path) -> RepoIndexSummary;
```

`RepoIndexSummary` lives in `imp-core` and is backed by the existing tree-sitter scan extractors. This avoids a parallel parser path and gives the scan tool and repo intelligence a common foundation.

## Target contract

As repo intelligence grows, promote the TUI-local `RepoStats` into a shared core snapshot:

```rust
pub struct RepoSnapshot {
    pub root: PathBuf,
    pub inventory: RepoInventory,
    pub index: Option<RepoIndexSummary>,
    pub rules: Vec<PathBuf>,
    pub packages: Vec<PackageSummary>,
}

pub struct RepoInventory {
    pub primary_language: String,
    pub code_lines: u64,
    pub files: u64,
    pub languages: Vec<LanguageStats>,
    pub source_files: Vec<FileSummary>,
}

pub struct LanguageStats {
    pub language: String,
    pub files: u64,
    pub code_lines: u64,
}

pub struct FileSummary {
    pub path: PathBuf,
    pub language: String,
    pub role: FileRole,
    pub code_lines: u64,
}

pub enum FileRole {
    Source,
    Test,
    Docs,
    Config,
    Manifest,
    Lockfile,
    Generated,
    Other,
}

pub struct PackageSummary {
    pub name: String,
    pub kind: PackageKind,
    pub root: PathBuf,
}

pub enum PackageKind {
    RustCrate,
    NodePackage,
    PythonPackage,
    GoModule,
    Other,
}
```

The homepage should only need a compact display projection:

```rust
pub struct RepoDisplaySummary {
    pub repo_line: String,  // "Rust · 14.3k loc · 120 files"
    pub index_line: Option<String>, // "480 symbols · 32 tests · 4 packages"
}
```

## State model

The TUI should preserve these states:

- `Scanning` — scan/index is running in the background
- `Ready` — snapshot is available
- `HomeDirectory` — cwd or repo root is the user's home directory; do not scan
- `NoRepo` — cwd is not inside a Git repo; do not scan arbitrary trees
- `Empty` — repo has no supported source files
- `Failed` — scan failed; do not block launch

## Related/impact graph direction

The next useful contract should derive approximate related/impact data from the same snapshot:

```rust
pub struct RelatedContext {
    pub files: Vec<RelatedFile>,
    pub tests: Vec<PathBuf>,
    pub packages: Vec<PackageSummary>,
    pub verification: Vec<String>,
}

pub struct RelatedFile {
    pub path: PathBuf,
    pub reason: String,
}
```

Initial edges can be approximate and explainable:

- file contains symbol
- file belongs to package
- test likely covers file/symbol
- manifest owns package
- docs/rules apply to repo

Imports/exports and call relationships can be added after the snapshot contract is stable.

## Design constraints

- Do not block TUI launch on repo intelligence.
- Keep homepage output compact; detailed related/impact data belongs in tools and agent context.
- Prefer existing tree-sitter scan extractors over duplicate parser logic.
- Keep scanner/index implementations replaceable behind the snapshot contract.
- Treat all repo intelligence as best-effort evidence, not an authoritative compiler/typechecker.
