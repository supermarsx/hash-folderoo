use anyhow::Result;
use std::io::Write;
use log::{info, warn};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Rename files under `path` according to a simple pattern.
/// Pattern format: "old->new" (replace occurrences of `old` in filenames with `new`).
/// If pattern does not contain "->", treat it as `old` and replace with empty string.
/// If `dry_run` is true, only print the planned renames.
/// Backward-compatible wrapper that calls the extended renamer with basic parameters.
pub fn rename_files(path: &Path, pattern: &str, dry_run: bool) -> Result<()> {
    // default git_diff_context = 3 for wrapper convenience
    rename_files_with_options(path, Some(pattern), None, None, false, dry_run, false, false, 3, None)
}


/// Advanced renamer that supports:
/// - `pattern` (+ optional `replace` if regex==true)
/// - `map` file (CSV or JSON) containing mapping pairs {src,dst} or two-column CSV
/// - `regex` flag: treat pattern as a regex and apply `replace` substitution on filenames
/// - `dry_run` and `git_diff` output options
pub fn rename_files_with_options(
    path: &Path,
    pattern: Option<&str>,
    replace: Option<&str>,
    map: Option<&Path>,
    regex: bool,
    dry_run: bool,
    git_diff: bool,
    git_diff_body: bool,
    git_diff_context: usize,
    git_diff_output: Option<&Path>,
) -> Result<()> {
    if !path.exists() {
        warn!("Path {} does not exist, nothing to do", path.display());
        return Ok(());
    }
    if path.is_file() {
        warn!("Path {} is a file; nothing to do", path.display());
        return Ok(());
    }

    // parse mappings
    let mut plan: Vec<(PathBuf, PathBuf)> = Vec::new();

    if let Some(map_path) = map {
        // support CSV or JSON mapping file
        if let Some(ext) = map_path.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()) {
            match ext.as_str() {
                "csv" => {
                    let mut rdr = csv::Reader::from_path(map_path)?;
                    for result in rdr.records() {
                        let rec = result?;
                        if rec.len() < 2 {
                            continue;
                        }
                        let src = path.join(rec.get(0).unwrap()).to_path_buf();
                        let dst = path.join(rec.get(1).unwrap()).to_path_buf();
                        plan.push((src, dst));
                    }
                }
                "json" => {
                    let json_val: serde_json::Value = serde_json::from_reader(std::fs::File::open(map_path)?)?;
                    if let Some(arr) = json_val.as_array() {
                        for obj in arr {
                            let s = obj.get("src").and_then(|v| v.as_str());
                            let d = obj.get("dst").and_then(|v| v.as_str());
                            if let (Some(s), Some(d)) = (s, d) {
                                plan.push((path.join(s), path.join(d)));
                            }
                        }
                    }
                }
                _ => {
                    warn!("Unsupported mapping file extension: {}", ext);
                }
            }
        }
    } else if let Some(pat) = pattern {
        if regex {
            let re = regex::Regex::new(pat).map_err(|e| anyhow::anyhow!(e))?;
            if replace.is_none() {
                anyhow::bail!("--replace must be provided when using --regex");
            }
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.is_file() {
                    if let Some(fname) = p.file_name().and_then(|s| s.to_str()) {
                        let new = re.replace_all(fname, replace.unwrap()).into_owned();
                        if new != fname {
                            let dst = p.parent().unwrap_or(Path::new("")).join(&new);
                            plan.push((p.to_path_buf(), dst));
                        }
                    }
                }
            }
        } else {
            // fallback to old substring replacement
            let (from, to) = if let Some(idx) = pat.find("->") {
                let (a, b) = pat.split_at(idx);
                (a.to_string(), b[2..].to_string())
            } else {
                (pat.to_string(), String::new())
            };

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
        }
    } else {
        warn!("No mapping / pattern provided for renamer; nothing to do");
        return Ok(());
    }

    if plan.is_empty() {
        info!("No files to rename");
        return Ok(());
    }

    println!("Planned renames:");
    for (s, d) in &plan {
        if git_diff {
            let diff = crate::diff::format_rename_diff(s, d, git_diff_body, git_diff_context);
            if let Some(out_path) = git_diff_output {
                if let Err(e) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(out_path)
                    .and_then(|mut f| f.write_all(diff.as_bytes()))
                {
                    let _ = writeln!(std::io::stderr(), "warning: failed writing diff to {}: {}", out_path.display(), e);
                }
            } else {
                println!("{}", diff);
            }
        } else {
            println!("{} -> {}", s.display(), d.display());
        }
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
                if let Err(e) = std::fs::create_dir_all(parent) {
                    warn!("Failed creating parent {}: {}", parent.display(), e);
                    continue;
                }
            }
        }
        match std::fs::rename(&s, &d) {
            Ok(_) => {
                info!("Renamed {} -> {}", s.display(), d.display());
                if git_diff {
                    let diff = crate::diff::format_rename_diff(&s, &d, git_diff_body, git_diff_context);
                    if let Some(out_path) = git_diff_output {
                        if let Err(e) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(out_path)
                            .and_then(|mut f| f.write_all(diff.as_bytes()))
                        {
                            let _ = writeln!(std::io::stderr(), "warning: failed writing diff to {}: {}", out_path.display(), e);
                        }
                    } else {
                        println!("{}", diff);
                    }
                }
            }
            Err(e) => warn!("Failed renaming {} -> {}: {}", s.display(), d.display(), e),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::{create_dir_all, write};

    #[test]
    fn regex_rename_dry_run_no_change_files() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        create_dir_all(&root).unwrap();
        write(root.join("file1.txt"), b"hello").unwrap();
        write(root.join("file2.txt"), b"world").unwrap();

        // regex replace digits with X
        let res = rename_files_with_options(&root, Some("file(\\d)"), Some("fileX"), None, true, true, true, true, 3, None);
        assert!(res.is_ok());

        // Dry-run should not have renamed files
        assert!(root.join("file1.txt").exists());
        assert!(root.join("file2.txt").exists());
    }

    #[test]
    fn map_file_rename_dry_run() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        create_dir_all(&root).unwrap();
        write(root.join("a.txt"), b"1").unwrap();

        // Prepare CSV mapping file
        let map_file = dir.path().join("map.csv");
        std::fs::write(&map_file, "a.txt,b.txt\n").unwrap();

        let res = rename_files_with_options(&root, None, None, Some(&map_file), false, true, true, true, 3, None);
        assert!(res.is_ok());
        // still unchanged after dry-run false? Wait dry_run true -> no change, we passed true so unchanged.
        assert!(root.join("a.txt").exists());
        assert!(!root.join("b.txt").exists());
    }
}
