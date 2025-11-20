use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::io::Write;

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

/// Generate a report summary from a map file (JSON or CSV).
///
/// - `input_path` is a path to a JSON or CSV map file.
/// - `format` controls output: case-insensitive "json" will emit pretty JSON to stdout,
///   any other value prints a human-readable textual summary.
///
/// The function returns Ok(()) on success or an anyhow error on failure.
/// Files with no extension are treated with an empty-string extension.
pub fn generate_report(input_path: &str, format: &str) -> Result<()> {
    let p = Path::new(input_path);

    // Load entries based on extension if possible, otherwise try json then csv.
    let entries: Vec<io::MapEntry> = if p.exists() && p.is_file() {
        if let Some(ext) = p.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()) {
            match ext.as_str() {
                "json" => io::load_map_from_json(p).with_context(|| format!("loading json {:?}", p))?,
                "csv" => io::load_map_from_csv(p).with_context(|| format!("loading csv {:?}", p))?,
                _ => {
                    // try json then csv
                    match io::load_map_from_json(p) {
                        Ok(v) => v,
                        Err(_) => io::load_map_from_csv(p).with_context(|| format!("loading csv {:?}", p))?,
                    }
                }
            }
        } else {
            // no extension, try json then csv
            match io::load_map_from_json(p) {
                Ok(v) => v,
                Err(_) => io::load_map_from_csv(p).with_context(|| format!("loading csv {:?}", p))?,
            }
        }
    } else {
        anyhow::bail!("input path does not exist or is not a file: {}", input_path);
    };

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

    for (_h, group) in &by_hash {
        if group.len() > 1 {
            duplicate_groups += 1;
            // count duplicates beyond the first
            duplicate_files += group.len() - 1;
            for dup in group.iter().skip(1) {
                duplicate_wasted_bytes = duplicate_wasted_bytes.saturating_add(dup.size);
            }
        }
    }

    // Count file extensions (lowercased); empty string for missing ext
    let mut ext_counts: HashMap<String, usize> = HashMap::new();
    for e in &entries {
        let ext = Path::new(&e.path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "".to_string());
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

    // Output
    if format.to_lowercase() == "json" {
        let out = serde_json::to_vec_pretty(&summary).context("serialize summary to json")?;
        std::io::stdout().write_all(&out).context("write summary to stdout")?;
    } else {
        // Human-readable
        println!("Report summary for: {}", input_path);
        println!("  Total files: {}", summary.total_files);
        println!("  Total size: {} bytes", summary.total_size_bytes);
        println!("  Duplicate groups (same hash): {}", summary.duplicate_groups);
        println!("  Duplicate files (beyond first): {}", summary.duplicate_files);
        println!("  Duplicate wasted space: {} bytes", summary.duplicate_wasted_bytes);
        println!("  Top extensions:");
        for (ext, cnt) in &summary.top_extensions {
            let display_ext = if ext.is_empty() { "<none>" } else { ext };
            println!("    {:>6}  {}", display_ext, cnt);
        }
    }

    Ok(())
}
