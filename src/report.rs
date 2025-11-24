use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use crate::io;

/// Summary produced by generate_report.
#[derive(serde::Serialize)]
struct ReportSummary {
    total_files: usize,
    total_size_bytes: u64,
    duplicate_groups: usize,
    duplicate_files: usize,
    duplicate_wasted_bytes: u64,
    top_extensions: Vec<(String, usize)>,
}

#[derive(serde::Serialize)]
struct DuplicateGroupReport {
    hash: String,
    count: usize,
    total_bytes: u64,
    wasted_bytes: u64,
    paths: Vec<String>,
}

#[derive(serde::Serialize)]
struct LargeFileReport {
    path: String,
    size: u64,
    mtime: Option<i64>,
}

#[derive(Default)]
struct ReportSections {
    stats: bool,
    duplicates: bool,
    largest: bool,
}

impl ReportSections {
    fn from_includes(includes: &[String]) -> Self {
        if includes.is_empty() {
            return Self {
                stats: true,
                duplicates: true,
                largest: true,
            };
        }
        let mut sections = Self::default();
        for item in includes {
            match item.to_lowercase().as_str() {
                "stats" => sections.stats = true,
                "duplicates" => sections.duplicates = true,
                "largest" | "largest_files" => sections.largest = true,
                _ => {}
            }
        }
        sections
    }
}

/// Generate a report summary from a map file (JSON or CSV).
///
/// - `input_path` is a path to a JSON or CSV map file.
/// - `format` controls output: case-insensitive "json" will emit pretty JSON to stdout,
///   any other value prints a human-readable textual summary.
///
/// The function returns Ok(()) on success or an anyhow error on failure.
/// Files with no extension are treated with an empty-string extension.
pub fn generate_report(
    input_path: &str,
    format: &str,
    includes: &[String],
    top_n: usize,
) -> Result<()> {
    let p = Path::new(input_path);

    // Load entries based on extension if possible, otherwise try json then csv.
    let entries: Vec<io::MapEntry> = if p.exists() && p.is_file() {
        if let Some(ext) = p
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
        {
            match ext.as_str() {
                "json" => {
                    io::load_map_from_json(p).with_context(|| format!("loading json {:?}", p))?
                }
                "csv" => {
                    io::load_map_from_csv(p).with_context(|| format!("loading csv {:?}", p))?
                }
                _ => {
                    // try json then csv
                    match io::load_map_from_json(p) {
                        Ok(v) => v,
                        Err(_) => io::load_map_from_csv(p)
                            .with_context(|| format!("loading csv {:?}", p))?,
                    }
                }
            }
        } else {
            // no extension, try json then csv
            match io::load_map_from_json(p) {
                Ok(v) => v,
                Err(_) => {
                    io::load_map_from_csv(p).with_context(|| format!("loading csv {:?}", p))?
                }
            }
        }
    } else {
        anyhow::bail!("input path does not exist or is not a file: {}", input_path);
    };

    let sections = ReportSections::from_includes(includes);

    // Compute totals
    let total_files = entries.len();
    let total_size_bytes: u64 = entries.iter().map(|e| e.size).sum();

    // Group by hash to find duplicates
    let mut by_hash: HashMap<String, Vec<&io::MapEntry>> = HashMap::new();
    for e in &entries {
        by_hash.entry(e.hash.clone()).or_default().push(e);
    }

    let mut duplicate_groups = 0usize;
    let mut duplicate_files = 0usize;
    let mut duplicate_wasted_bytes: u64 = 0;
    let mut duplicate_rows: Vec<DuplicateGroupReport> = Vec::new();

    for (hash, group) in &by_hash {
        if group.len() > 1 {
            duplicate_groups += 1;
            duplicate_files += group.len() - 1;
            let mut total_bytes = 0u64;
            let mut wasted_bytes = 0u64;
            for (idx, dup) in group.iter().enumerate() {
                total_bytes = total_bytes.saturating_add(dup.size);
                if idx > 0 {
                    wasted_bytes = wasted_bytes.saturating_add(dup.size);
                    duplicate_wasted_bytes = duplicate_wasted_bytes.saturating_add(dup.size);
                }
            }
            if sections.duplicates {
                duplicate_rows.push(DuplicateGroupReport {
                    hash: hash.clone(),
                    count: group.len(),
                    total_bytes,
                    wasted_bytes,
                    paths: group.iter().map(|e| e.path.clone()).collect(),
                });
            }
        }
    }

    if sections.duplicates {
        duplicate_rows.sort_by(|a, b| b.wasted_bytes.cmp(&a.wasted_bytes));
        duplicate_rows.truncate(top_n);
    }

    // Count file extensions (lowercased); empty string for missing ext
    let mut ext_counts: HashMap<String, usize> = HashMap::new();
    for e in &entries {
        let ext = Path::new(&e.path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        *ext_counts.entry(ext).or_default() += 1;
    }

    // Build top 5 extensions sorted by count desc
    let mut top_extensions: Vec<(String, usize)> = ext_counts.into_iter().collect();
    top_extensions.sort_by(|a, b| b.1.cmp(&a.1));
    if top_extensions.len() > 5 {
        top_extensions.truncate(5);
    }

    let summary = ReportSummary {
        total_files,
        total_size_bytes,
        duplicate_groups,
        duplicate_files,
        duplicate_wasted_bytes,
        top_extensions,
    };

    let mut largest_files: Vec<LargeFileReport> = Vec::new();
    if sections.largest {
        let mut sorted = entries.clone();
        sorted.sort_by(|a, b| b.size.cmp(&a.size));
        for entry in sorted.into_iter().take(top_n) {
            largest_files.push(LargeFileReport {
                path: entry.path,
                size: entry.size,
                mtime: entry.mtime,
            });
        }
    }

    #[derive(serde::Serialize)]
    struct ReportOutput {
        #[serde(skip_serializing_if = "Option::is_none")]
        stats: Option<ReportSummary>,
        #[serde(skip_serializing_if = "Option::is_none")]
        duplicates: Option<Vec<DuplicateGroupReport>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        largest_files: Option<Vec<LargeFileReport>>,
    }

    let output = ReportOutput {
        stats: sections.stats.then_some(summary),
        duplicates: if sections.duplicates {
            Some(duplicate_rows)
        } else {
            None
        },
        largest_files: if sections.largest {
            Some(largest_files)
        } else {
            None
        },
    };

    if format.to_lowercase() == "json" {
        let out = serde_json::to_vec_pretty(&output).context("serialize summary to json")?;
        std::io::stdout()
            .write_all(&out)
            .context("write summary to stdout")?;
    } else {
        println!("Report summary for: {}", input_path);
        if let Some(stats) = &output.stats {
            println!("  Total files: {}", stats.total_files);
            println!("  Total size: {} bytes", stats.total_size_bytes);
            println!("  Duplicate groups: {}", stats.duplicate_groups);
            println!("  Duplicate files: {}", stats.duplicate_files);
            println!(
                "  Duplicate wasted space: {} bytes",
                stats.duplicate_wasted_bytes
            );
            println!("  Top extensions:");
            for (ext, cnt) in &stats.top_extensions {
                let display_ext = if ext.is_empty() { "<none>" } else { ext };
                println!("    {:>6}  {}", display_ext, cnt);
            }
        }
        if let Some(dups) = &output.duplicates {
            println!("\nTop duplicate groups:");
            for group in dups {
                println!(
                    "  hash {} -> {} files, wasted {} bytes",
                    group.hash, group.count, group.wasted_bytes
                );
                for path in &group.paths {
                    println!("    - {}", path);
                }
            }
        }
        if let Some(largest) = &output.largest_files {
            println!("\nLargest files:");
            for file in largest {
                println!("  {:>12} bytes  {}", file.size, file.path);
            }
        }
    }

    Ok(())
}
