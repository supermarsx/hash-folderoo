use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn build_globset(exclusions: &[String]) -> Result<Option<GlobSet>> {
    if exclusions.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pat in exclusions {
        let g = Glob::new(pat).with_context(|| format!("invalid glob pattern: {}", pat))?;
        builder.add(g);
    }
    Ok(Some(builder.build().context("failed to build globset")?))
}

pub struct WalkStream {
    root: PathBuf,
    walker: walkdir::IntoIter,
    globset: Option<GlobSet>,
}

impl WalkStream {
    fn new(
        root: PathBuf,
        exclusions: &[String],
        max_depth: Option<usize>,
        follow_symlinks: bool,
    ) -> Result<Self> {
        let globset = build_globset(exclusions)?;
        let mut walk_builder = WalkDir::new(&root);
        if let Some(depth) = max_depth {
            walk_builder = walk_builder.max_depth(depth);
        }
        if follow_symlinks {
            walk_builder = walk_builder.follow_links(true);
        }
        Ok(Self {
            root,
            walker: walk_builder.into_iter(),
            globset,
        })
    }
}

impl Iterator for WalkStream {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
            for entry in self.walker.by_ref() {
                match entry {
                Ok(e) => {
                    if !e.file_type().is_file() {
                        continue;
                    }
                    let path = e.into_path();
                    let rel = path.strip_prefix(&self.root).unwrap_or(&path);
                    if let Some(gs) = &self.globset {
                        if gs.is_match(rel) {
                            continue;
                        }
                    }
                    return Some(path);
                }
                Err(_) => continue,
            }
        }
        None
    }
}

/// Walk a directory and return a list of file paths, excluding patterns.
///
/// `root` - root directory to walk.
/// `exclusions` - list of glob patterns (relative to `root`) to exclude, e.g. `["target/**", "**/.git/**"]`.
/// `max_depth` - optional depth cap.
/// `follow_symlinks` - whether to follow symlinked directories.
pub fn walk_directory<P: AsRef<Path>>(
    root: P,
    exclusions: &[String],
    max_depth: Option<usize>,
    follow_symlinks: bool,
) -> Result<Vec<PathBuf>> {
    let stream = walk_directory_stream(root, exclusions, max_depth, follow_symlinks)?;
    Ok(stream.collect())
}

pub fn walk_directory_stream<P: AsRef<Path>>(
    root: P,
    exclusions: &[String],
    max_depth: Option<usize>,
    follow_symlinks: bool,
) -> Result<WalkStream> {
    let root_buf = root.as_ref().to_path_buf();
    WalkStream::new(root_buf, exclusions, max_depth, follow_symlinks)
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

        let paths = walk_directory(&root, &["target/**".to_string()], None, false).unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths.iter().any(|p| p.ends_with("a.txt")));
    }

    #[test]
    fn test_walk_directory_depth_limit() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        create_dir_all(root.join("sub")).unwrap();
        File::create(root.join("top.txt")).unwrap();
        File::create(root.join("sub").join("nested.txt")).unwrap();

        let all_paths = walk_directory(&root, &[], None, false).unwrap();
        assert_eq!(all_paths.len(), 2);

        let shallow = walk_directory(&root, &[], Some(1), false).unwrap();
        assert_eq!(shallow.len(), 1);
        assert!(shallow.iter().any(|p| p.ends_with("top.txt")));
    }
}
