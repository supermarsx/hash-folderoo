use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// CLI definition for hash-folderoo
#[derive(Parser, Debug)]
#[command(name = "hash-folderoo", version = env!("CARGO_PKG_VERSION"), about = "Hash-based folder toolkit")]
pub struct Cli {
    /// Print supported algorithms and exit
    #[arg(long = "alg-list", global = true)]
    pub alg_list: bool,

    /// Optional configuration file path (TOML/YAML/JSON)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Subcommand to run
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a hashmap of files in a directory
    Hashmap(HashmapArgs),
    /// Compare two hashmaps or directories
    Compare(CompareArgs),
    /// Create or execute a copy plan based on diffs
    Copydiff(CopydiffArgs),
    /// Remove empty directories
    Removempty(RemovemptyArgs),
    /// Rename files according to a pattern
    Renamer(RenamerArgs),
    /// Benchmark hashing algorithms
    Benchmark(BenchmarkArgs),
    /// Generate reports from inputs
    Report(ReportArgs),
}

#[derive(Args, Debug)]
pub struct HashmapArgs {
    /// Root path to scan
    #[arg(long, short('p'))]
    pub path: Option<PathBuf>,

    /// Output file (defaults to stdout)
    #[arg(long, short('o'))]
    pub output: Option<PathBuf>,

    /// Output format (json/csv)
    #[arg(long, short('f'))]
    pub format: Option<String>,

    /// Hash algorithm to use (e.g. blake3, sha3)
    #[arg(long, short('a'))]
    pub algorithm: Option<String>,

    /// Maximum directory traversal depth
    #[arg(long)]
    pub depth: Option<usize>,

    /// Strip this prefix from recorded file paths
    #[arg(long = "strip-prefix")]
    pub strip_prefix: Option<PathBuf>,

    /// XOF output length in bytes (only for algorithms that support it)
    #[arg(long = "xof-length")]
    pub xof_length: Option<usize>,

    /// Allow requesting XOF-like output lengths for algorithms that don't natively support XOF.
    /// This enables deterministic expansion behavior (opt-in) and is intentionally required
    /// to avoid accidental non-standard output when users request large lengths for fixed-output algorithms.
    #[arg(long = "force-expand")]
    pub force_expand: bool,

    /// Exclude patterns (can be given multiple times or comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub exclude: Vec<String>,

    /// Follow symbolic links when walking directories
    #[arg(long = "follow-symlinks")]
    pub follow_symlinks: bool,

    /// Show a progress bar while hashing
    #[arg(long = "progress")]
    pub progress: bool,

    /// Perform a dry-run (hash files but skip writing output)
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Suppress non-error output
    #[arg(long)]
    pub silent: bool,

    /// Number of worker threads to use
    #[arg(long)]
    pub threads: Option<usize>,

    /// Memory mode (e.g. auto, low, high)
    #[arg(long = "mem-mode")]
    pub mem_mode: Option<String>,

    /// Maximum memory budget in bytes for hashing buffers
    #[arg(long = "max-ram")]
    pub max_ram: Option<u64>,
}

#[derive(Args, Debug)]
pub struct CompareArgs {
    /// Source path or file
    #[arg(long)]
    pub source: Option<PathBuf>,

    /// Target path or file
    #[arg(long)]
    pub target: Option<PathBuf>,

    /// Output file (defaults to stdout)
    #[arg(long, short('o'))]
    pub output: Option<PathBuf>,

    /// Output format (json/csv)
    #[arg(long)]
    pub format: Option<String>,

    /// Hash algorithm to use when hashing directories
    #[arg(long, short('a'))]
    pub algorithm: Option<String>,
}

#[derive(Args, Debug)]
pub struct CopydiffArgs {
    /// Copy plan file (input or output depending on mode)
    #[arg(long)]
    pub plan: Option<PathBuf>,

    /// Execute the planned operations
    #[arg(long)]
    pub execute: bool,

    /// Show what would be done without executing
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    /// When showing a dry-run or run summary, emit a git-style diff for each planned operation
    #[arg(long = "git-diff")]
    pub git_diff: bool,
    /// Source path or file (used to generate a plan when --plan is not provided)
    #[arg(long)]
    pub source: Option<PathBuf>,

    /// Target path or file (used to generate a plan when --plan is not provided)
    #[arg(long)]
    pub target: Option<PathBuf>,

    /// Hash algorithm to use when hashing directories
    #[arg(long, short('a'))]
    pub algorithm: Option<String>,

    /// Conflict handling strategy (overwrite, skip, rename)
    #[arg(long = "conflict", default_value = "overwrite")]
    pub conflict: String,

    /// Preserve file modification times when copying
    #[arg(long = "preserve-times")]
    pub preserve_times: bool,
}

#[derive(Args, Debug)]
pub struct RemovemptyArgs {
    /// Path to clean empty directories from
    #[arg(long, short('p'))]
    pub path: Option<PathBuf>,

    /// Don't actually remove, just show what would be removed
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Minimum depth (relative to root) before removal is allowed
    #[arg(long = "min-empty-depth")]
    pub min_empty_depth: Option<usize>,

    /// Directory exclusion patterns
    #[arg(long, value_delimiter = ',')]
    pub exclude: Vec<String>,
    /// Emit git-style diff entries for removals when performing a dry-run or run
    #[arg(long = "git-diff")]
    pub git_diff: bool,
}

#[derive(Args, Debug)]
pub struct RenamerArgs {
    /// Path containing files to rename
    #[arg(long, short('p'))]
    pub path: Option<PathBuf>,

    /// Rename pattern (implementation defined)
    #[arg(long)]
    pub pattern: Option<String>,

    /// Replacement string when using regex mode (used with --pattern which becomes the regex)
    #[arg(long = "replace")]
    pub replace: Option<String>,

    /// Path to a mapping file (CSV or JSON) describing renames as pairs; if present, mappings take precedence
    #[arg(long = "map")]
    pub map: Option<std::path::PathBuf>,

    /// Treat --pattern as a regex (use --replace for substitution)
    #[arg(long = "regex")]
    pub regex: bool,

    /// When showing a dry-run or run summary, emit a git-style diff for each planned rename
    #[arg(long = "git-diff")]
    pub git_diff: bool,

    /// Don't actually rename, just show what would be renamed
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct BenchmarkArgs {
    /// Algorithm to benchmark (e.g. blake3, sha3)
    #[arg(long)]
    pub algorithm: Option<String>,

    /// Size in bytes for the benchmark input
    #[arg(long)]
    pub size: Option<usize>,
}

#[derive(Args, Debug)]
pub struct ReportArgs {
    /// Input file (report source)
    #[arg(long)]
    pub input: Option<PathBuf>,

    /// Output format (json/csv)
    #[arg(long)]
    pub format: Option<String>,

    /// Sections to include (comma-separated: stats,duplicates,largest)
    #[arg(long, value_delimiter = ',')]
    pub include: Vec<String>,

    /// Number of entries for top lists
    #[arg(long = "top-n")]
    pub top_n: Option<usize>,
}
