use anyhow::Result;
use globset::{Glob, GlobSet};
use log::warn;
use std::fs;
use std::path::Path;

/// Remove empty directories in `path` using post-order traversal.
/// `min_depth` controls the minimum depth at which directories may be removed.
/// `excludes` is a list of glob patterns (relative to `path`) to skip removal.
pub fn remove_empty_directories(
    path: &Path,
    dry_run: bool,
    min_depth: Option<usize>,
    excludes: &[String],
    git_diff: bool,
) -> Result<()> {
    if !path.exists() {
        warn!("Path {} does not exist, nothing to do", path.display());
        return Ok(());
    }
    if path.is_file() {
        warn!("Path {} is a file, nothing to do", path.display());
        return Ok(());
    }

    let globset = if excludes.is_empty() {
        None
    } else {
        let mut builder = globset::GlobSetBuilder::new();
        for pat in excludes {
            builder.add(Glob::new(pat)?);
        }
        Some(builder.build()?)
    };

    let root = path.to_path_buf();
    let min_allowed = min_depth.unwrap_or(0);

    fn helper(
        p: &Path,
        dry_run: bool,
        git_diff: bool,
        root: &Path,
        depth: usize,
        min_allowed: usize,
        excludes: &Option<GlobSet>,
    ) -> Result<bool> {
        let mut is_empty = true;
        for entry in fs::read_dir(p)? {
            let e = entry?;
            let pth = e.path();
            if pth.is_dir() {
                let child_empty = helper(&pth, dry_run, git_diff, root, depth + 1, min_allowed, excludes)?;
                if !child_empty {
                    is_empty = false;
                }
            } else {
                is_empty = false;
            }
        }

        let rel = p.strip_prefix(root).unwrap_or(Path::new(""));
        let excluded = excludes
            .as_ref()
            .map(|gs| gs.is_match(rel))
            .unwrap_or(false);

        if is_empty && !excluded && depth >= min_allowed {
            if dry_run {
                if git_diff {
                    println!("{}", crate::diff::format_remove_dir_diff(p));
                } else {
                    println!("Would remove empty directory: {}", p.display());
                }
            } else {
                if git_diff {
                    println!("{}", crate::diff::format_remove_dir_diff(p));
                } else {
                    println!("Removing empty directory: {}", p.display());
                }
                fs::remove_dir(p)?;
            }
            return Ok(true);
        }

        if excluded {
            return Ok(false);
        }
        Ok(is_empty)
    }

    // start recursion
    helper(path, dry_run, git_diff, &root, 0, min_allowed, &globset)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, File};
    use tempfile::tempdir;

    #[test]
    fn respects_min_depth_and_excludes() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        create_dir_all(root.join("a").join("b")).unwrap();
        create_dir_all(root.join("keep")).unwrap();
        create_dir_all(root.join("top_empty")).unwrap();
        File::create(root.join("keep").join("file.txt")).unwrap();
        remove_empty_directories(&root, false, Some(2), &["keep/**".to_string()], false).unwrap();
        assert!(root.join("a").exists());
        assert!(!root.join("a").join("b").exists());
        assert!(root.join("keep").exists());
        assert!(root.join("top_empty").exists());
    }
}
