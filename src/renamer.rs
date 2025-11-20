use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;
use log::{info, warn};
use walkdir::WalkDir;

/// Rename files under `path` according to a simple pattern.
/// Pattern format: "old->new" (replace occurrences of `old` in filenames with `new`).
/// If pattern does not contain "->", treat it as `old` and replace with empty string.
/// If `dry_run` is true, only print the planned renames.
pub fn rename_files(path: &Path, pattern: &str, dry_run: bool) -> Result<()> {
    if !path.exists() {
        warn!("Path {} does not exist, nothing to do", path.display());
        return Ok(());
    }
    if path.is_file() {
        warn!("Path {} is a file; nothing to do", path.display());
        return Ok(());
    }

    // Parse pattern
    let (from, to) = if let Some(idx) = pattern.find("->") {
        let (a, b) = pattern.split_at(idx);
        let b = &b[2..];
        (a.to_string(), b.to_string())
    } else {
        (pattern.to_string(), String::new())
    };

    let mut plan: Vec<(PathBuf, PathBuf)> = Vec::new();

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_file() {
            if let Some(fname) = p.file_name().and_then(|s| s.to_str()) {
                let new_fname = fname.replace(&from, &to);
                if new_fname != fname {
                    let dst = p.parent().unwrap_or(Path::new("")).join(&new_fname);
                    plan.push((p.to_path_buf(), dst));
                }
            }
        }
    }

    if plan.is_empty() {
        info!("No files to rename for pattern '{}'", pattern);
        return Ok(());
    }

    println!("Planned renames:");
    for (s, d) in &plan {
        println!("{} -> {}", s.display(), d.display());
    }

    if dry_run {
        println!("Dry-run mode; not performing renames");
        return Ok(());
    }

    for (s, d) in plan {
        if d.exists() {
            warn!("Target exists, skipping: {}", d.display());
            continue;
        }
        if let Some(parent) = d.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    warn!("Failed creating parent {}: {}", parent.display(), e);
                    continue;
                }
            }
        }
        match fs::rename(&s, &d) {
            Ok(_) => info!("Renamed {} -> {}", s.display(), d.display()),
            Err(e) => warn!("Failed renaming {} -> {}: {}", s.display(), d.display(), e),
        }
    }

    Ok(())
}