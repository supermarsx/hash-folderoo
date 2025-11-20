use std::fs;
use std::path::Path;
use anyhow::Result;
use log::{info, warn};

/// Remove empty directories in `path` using post-order traversal.
/// If `dry_run` is true, only print what would be removed.
pub fn remove_empty_directories(path: &Path, dry_run: bool) -> Result<()> {
    if !path.exists() {
        warn!("Path {} does not exist, nothing to do", path.display());
        return Ok(());
    }
    if path.is_file() {
        warn!("Path {} is a file, nothing to do", path.display());
        return Ok(());
    }

    fn helper(p: &Path, dry_run: bool) -> Result<bool> {
        let mut is_empty = true;
        for entry in fs::read_dir(p)? {
            let e = entry?;
            let pth = e.path();
            if pth.is_dir() {
                let child_empty = helper(&pth, dry_run)?;
                if !child_empty {
                    is_empty = false;
                }
            } else {
                is_empty = false;
            }
        }

        if is_empty {
            if dry_run {
                println!("Would remove empty directory: {}", p.display());
            } else {
                println!("Removing empty directory: {}", p.display());
                fs::remove_dir(p)?;
            }
            return Ok(true);
        }

        Ok(false)
    }

    // start recursion
    helper(path, dry_run)?;
    Ok(())
}