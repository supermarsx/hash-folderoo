use clap::{Parser, Subcommand, Args};
use std::path::PathBuf;

/// CLI definition for hash-folderoo
#[derive(Parser, Debug)]
#[command(name = "hash-folderoo", version = env!("CARGO_PKG_VERSION"), about = "Hash-based folder toolkit")]
pub struct Cli {
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

    /// Exclude patterns (can be given multiple times or comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub exclude: Vec<String>,

    /// Suppress non-error output
    #[arg(long)]
    pub silent: bool,

    /// Number of worker threads to use
    #[arg(long)]
    pub threads: Option<usize>,

    /// Memory mode (e.g. auto, low, high)
    #[arg(long = "mem-mode")]
    pub mem_mode: Option<String>,
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
    /// Source path or file (used to generate a plan when --plan is not provided)
    #[arg(long)]
    pub source: Option<PathBuf>,

    /// Target path or file (used to generate a plan when --plan is not provided)
    #[arg(long)]
    pub target: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct RemovemptyArgs {
    /// Path to clean empty directories from
    #[arg(long, short('p'))]
    pub path: Option<PathBuf>,

    /// Don't actually remove, just show what would be removed
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct RenamerArgs {
    /// Path containing files to rename
    #[arg(long, short('p'))]
    pub path: Option<PathBuf>,

    /// Rename pattern (implementation defined)
    #[arg(long)]
    pub pattern: Option<String>,

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
}