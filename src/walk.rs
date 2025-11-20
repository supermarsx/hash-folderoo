use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use walkdir::WalkDir;
use globset::{Glob, GlobSetBuilder};

/// Walk a directory and return a list of file paths, excluding patterns.
///
/// `root` - root directory to walk.
/// `exclusions` - list of glob patterns (relative to `root`) to exclude, e.g. `["target/**", "**/.git/**"]`.
pub fn walk_directory<P: AsRef<Path>>(root: P, exclusions: &[String]) -> Result<Vec<PathBuf>> {
    let root = root.as_ref();

    // Build globset from exclusion patterns
    let mut builder = GlobSetBuilder::new();
    for pat in exclusions {
        let g = Glob::new(pat)
            .with_context(|| format!("invalid glob pattern: {}", pat))?;
        builder.add(g);
    }
    let globset = builder.build().context("failed to build globset")?;

    let mut files = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !entry.file_type().is_file() {
            continue;
        }

        // Prefer matching against the path relative to the root when possible.
        let rel = path.strip_prefix(root).unwrap_or(path);
        if globset.is_match(rel) {
            continue;
        }

        files.push(path.to_path_buf());
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, File};
    use tempfile::tempdir;

    #[test]
    fn test_walk_directory_excludes() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        create_dir_all(&root).unwrap();
        File::create(root.join("a.txt")).unwrap();
        create_dir_all(root.join("target")).unwrap();
        File::create(root.join("target").join("b.txt")).unwrap();

        let paths = walk_directory(&root, &["target/**".to_string()]).unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths.iter().any(|p| p.ends_with("a.txt")));
    }
}