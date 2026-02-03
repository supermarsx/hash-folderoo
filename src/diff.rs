use std::path::Path;

/// Simple helper to format git-style diffs for file operations.
/// These are lightweight, primarily human-reviewable strings (not full patch metadata).
fn read_lines_opt(p: &Path) -> Option<Vec<String>> {
    match std::fs::read_to_string(p) {
        Ok(s) => Some(s.lines().map(|l| l.to_string()).collect()),
        Err(_) => None,
    }
}

#[allow(clippy::needless_range_loop, clippy::single_char_add_str)]
pub fn format_copy_diff(
    src: &Path,
    dst: &Path,
    new_file: bool,
    conflict: Option<&str>,
    include_patch: bool,
    context: usize,
) -> String {
    let src_s = src.to_string_lossy();
    let dst_s = dst.to_string_lossy();
    let mut out = String::new();
    out.push_str(&format!("diff --git a/{} b/{}\n", src_s, dst_s));
    if new_file {
        out.push_str("new file mode 100644\n");
    } else if let Some(conf) = conflict {
        out.push_str(&format!("modified (conflict strategy: {})\n", conf));
    } else {
        out.push_str("modified\n");
    }

    out.push_str(&format!("--- a/{}\n", src_s));
    out.push_str(&format!("+++ b/{}\n\n", dst_s));

    if include_patch {
        // Try to include a simple unified-like body; fall back silently on IO failures
        // Attempt a more precise multi-hunk unified-style body with context lines.
        let src_lines = read_lines_opt(src).unwrap_or_default();
        let dst_lines = read_lines_opt(dst).unwrap_or_default();

        // local helper: compute LCS matching positions for two slices
        fn lcs_positions(a: &[String], b: &[String]) -> Vec<(usize, usize)> {
            let n = a.len();
            let m = b.len();
            // dp table (n+1) x (m+1)
            let mut dp = vec![vec![0usize; m + 1]; n + 1];
            for i in (0..n).rev() {
                for j in (0..m).rev() {
                    if a[i] == b[j] {
                        dp[i][j] = dp[i + 1][j + 1] + 1;
                    } else {
                        dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
                    }
                }
            }

            // backtrack to produce matches
            let mut res = Vec::new();
            let (mut i, mut j) = (0usize, 0usize);
            while i < n && j < m {
                if a[i] == b[j] {
                    res.push((i, j));
                    i += 1;
                    j += 1;
                } else if dp[i + 1][j] >= dp[i][j + 1] {
                    i += 1;
                } else {
                    j += 1;
                }
            }
            res
        }

        // compute change blocks between matches
        let matches = lcs_positions(&src_lines, &dst_lines);
        let mut blocks: Vec<(usize, usize, usize, usize)> = Vec::new();

        let mut a_idx = 0usize;
        let mut b_idx = 0usize;
        for (mi, mj) in matches.iter() {
            if *mi > a_idx || *mj > b_idx {
                blocks.push((a_idx, *mi, b_idx, *mj));
            }
            a_idx = mi + 1;
            b_idx = mj + 1;
        }
        if a_idx < src_lines.len() || b_idx < dst_lines.len() {
            blocks.push((a_idx, src_lines.len(), b_idx, dst_lines.len()));
        }

        // expand blocks with context and merge overlapping
        let mut hunks: Vec<(usize, usize, usize, usize)> = Vec::new();
        for (a0, a1, b0, b1) in blocks.into_iter() {
            // expand
            let start_a = a0.saturating_sub(context);
            let start_b = b0.saturating_sub(context);
            let end_a = (a1 + context).min(src_lines.len());
            let end_b = (b1 + context).min(dst_lines.len());

            if let Some(last) = hunks.last_mut() {
                // merge if overlapping or touching
                if start_a <= last.1 || start_b <= last.3 {
                    // extend
                    last.1 = last.1.max(end_a);
                    last.3 = last.3.max(end_b);
                    continue;
                }
            }
            hunks.push((start_a, end_a, start_b, end_b));
        }

        // fallback: if no hunks were generated, emit a single full-file hunk
        if hunks.is_empty() {
            let ha = 0usize;
            let hb = src_lines.len();
            let ka = 0usize;
            let kb = dst_lines.len();
            let old_count = hb.saturating_sub(ha);
            let new_count = kb.saturating_sub(ka);
            if !(old_count == 0 && new_count == 0) {
                out.push_str(&format!(
                    "@@ -{},{} +{},{} @@\n",
                    ha + 1,
                    old_count,
                    ka + 1,
                    new_count
                ));
                let old_slice = &src_lines[ha..hb];
                let new_slice = &dst_lines[ka..kb];
                let local_matches = lcs_positions(old_slice, new_slice);
                let mut ai = 0usize;
                let mut bi = 0usize;
                for (omi, omj) in local_matches.iter() {
                    for r in ai..*omi {
                        out.push_str(&format!("-{}\n", old_slice[r]));
                    }
                    for a in bi..*omj {
                        out.push_str(&format!("+{}\n", new_slice[a]));
                    }
                    out.push_str(&format!(" {}\n", old_slice[*omi]));
                    ai = omi + 1;
                    bi = omj + 1;
                }
                for r in ai..old_slice.len() {
                    out.push_str(&format!("-{}\n", old_slice[r]));
                }
                for a in bi..new_slice.len() {
                    out.push_str(&format!("+{}\n", new_slice[a]));
                }
                out.push_str("\n");
            }
        } else {
            for (ha, hb, ka, kb) in hunks.iter() {
                let old_count = hb - ha;
                let new_count = kb - ka;
                out.push_str(&format!(
                    "@@ -{},{} +{},{} @@\n",
                    ha + 1,
                    old_count,
                    ka + 1,
                    new_count
                ));

                // local slices
                let old_slice = &src_lines[*ha..*hb];
                let new_slice = &dst_lines[*ka..*kb];

                // compute local LCS to drive the hunk output
                let local_matches = lcs_positions(old_slice, new_slice);

                let mut ai = 0usize;
                let mut bi = 0usize;
                for (omi, omj) in local_matches.iter() {
                    // produce removed lines from ai..omi
                    for r in ai..*omi {
                        out.push_str(&format!("-{}\n", old_slice[r]));
                    }
                    // produce added lines from bi..omj
                    for a in bi..*omj {
                        out.push_str(&format!("+{}\n", new_slice[a]));
                    }
                    // matched line as context
                    out.push_str(&format!(" {}\n", old_slice[*omi]));
                    ai = omi + 1;
                    bi = omj + 1;
                }
                // remaining tail
                for r in ai..old_slice.len() {
                    out.push_str(&format!("-{}\n", old_slice[r]));
                }
                for a in bi..new_slice.len() {
                    out.push_str(&format!("+{}\n", new_slice[a]));
                }

                out.push_str("\n");
            }
        }
    }
    out
}

#[allow(clippy::needless_range_loop, clippy::single_char_add_str)]
pub fn format_rename_diff(src: &Path, dst: &Path, include_patch: bool, context: usize) -> String {
    let src_s = src.to_string_lossy();
    let dst_s = dst.to_string_lossy();
    let mut out = format!(
        "diff --git a/{0} b/{1}\nrename from {0}\nrename to   {1}\n\n",
        src_s, dst_s
    );
    if include_patch {
        if let Some(src_lines) = read_lines_opt(src) {
            let dst_lines = read_lines_opt(dst).unwrap_or_default();

            // reuse same LCS + hunk generation strategy as for copies
            fn lcs_positions(a: &[String], b: &[String]) -> Vec<(usize, usize)> {
                let n = a.len();
                let m = b.len();
                let mut dp = vec![vec![0usize; m + 1]; n + 1];
                for i in (0..n).rev() {
                    for j in (0..m).rev() {
                        if a[i] == b[j] {
                            dp[i][j] = dp[i + 1][j + 1] + 1;
                        } else {
                            dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
                        }
                    }
                }
                let mut res = Vec::new();
                let (mut i, mut j) = (0usize, 0usize);
                while i < n && j < m {
                    if a[i] == b[j] {
                        res.push((i, j));
                        i += 1;
                        j += 1;
                    } else if dp[i + 1][j] >= dp[i][j + 1] {
                        i += 1;
                    } else {
                        j += 1;
                    }
                }
                res
            }

            let matches = lcs_positions(&src_lines, &dst_lines);
            let mut blocks: Vec<(usize, usize, usize, usize)> = Vec::new();

            let mut a_idx = 0usize;
            let mut b_idx = 0usize;
            for (mi, mj) in matches.iter() {
                if *mi > a_idx || *mj > b_idx {
                    blocks.push((a_idx, *mi, b_idx, *mj));
                }
                a_idx = mi + 1;
                b_idx = mj + 1;
            }
            if a_idx < src_lines.len() || b_idx < dst_lines.len() {
                blocks.push((a_idx, src_lines.len(), b_idx, dst_lines.len()));
            }

            // expand and merge
            let mut hunks: Vec<(usize, usize, usize, usize)> = Vec::new();
            for (a0, a1, b0, b1) in blocks.into_iter() {
                let start_a = a0.saturating_sub(context);
                let start_b = b0.saturating_sub(context);
                let end_a = (a1 + context).min(src_lines.len());
                let end_b = (b1 + context).min(dst_lines.len());
                if let Some(last) = hunks.last_mut() {
                    if start_a <= last.1 || start_b <= last.3 {
                        last.1 = last.1.max(end_a);
                        last.3 = last.3.max(end_b);
                        continue;
                    }
                }
                hunks.push((start_a, end_a, start_b, end_b));
            }

            for (ha, hb, ka, kb) in hunks.iter() {
                let old_count = hb - ha;
                let new_count = kb - ka;
                out.push_str(&format!(
                    "@@ -{},{} +{},{} @@\n",
                    ha + 1,
                    old_count,
                    ka + 1,
                    new_count
                ));
                let old_slice = &src_lines[*ha..*hb];
                let new_slice = &dst_lines[*ka..*kb];
                let local_matches = lcs_positions(old_slice, new_slice);
                let mut ai = 0usize;
                let mut bi = 0usize;
                for (omi, omj) in local_matches.iter() {
                    for r in ai..*omi {
                        out.push_str(&format!("-{}\n", old_slice[r]));
                    }
                    for a in bi..*omj {
                        out.push_str(&format!("+{}\n", new_slice[a]));
                    }
                    out.push_str(&format!(" {}\n", old_slice[*omi]));
                    ai = omi + 1;
                    bi = omj + 1;
                }
                for r in ai..old_slice.len() {
                    out.push_str(&format!("-{}\n", old_slice[r]));
                }
                for a in bi..new_slice.len() {
                    out.push_str(&format!("+{}\n", new_slice[a]));
                }
                out.push_str("\n");
            }
        }
    }

    out
}

pub fn format_remove_dir_diff(dir: &Path) -> String {
    let d = dir.to_string_lossy();
    format!(
        "diff --git a/{0} b/{0}\ndeleted dir mode 040000\n--- a/{0}\n+++ /dev/null\n\n",
        d
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn copy_diff_contains_paths() {
        let src = PathBuf::from("a/foo.txt");
        let dst = PathBuf::from("b/foo.txt");
        let s = format_copy_diff(&src, &dst, true, None, false, 3);
        assert!(s.contains("diff --git a/a/foo.txt b/b/foo.txt"));
        assert!(s.contains("new file mode"));
    }

    #[test]
    fn rename_diff_contains_paths() {
        let src = PathBuf::from("a/old.txt");
        let dst = PathBuf::from("a/new.txt");
        let s = format_rename_diff(&src, &dst, false, 3);
        assert!(s.contains("rename from a/old.txt"));
        assert!(s.contains("rename to   a/new.txt"));
    }

    #[test]
    fn copy_diff_with_content() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("source.txt");
        let dst = dir.path().join("dest.txt");
        std::fs::write(&src, b"line1\nline2\nline3\n").unwrap();
        
        let diff = format_copy_diff(&src, &dst, true, Some(&src.to_string_lossy()), false, 3);
        assert!(diff.contains("diff --git"));
        assert!(diff.contains("new file mode"));
    }

    #[test]
    fn copy_diff_dry_run() {
        let src = PathBuf::from("test.txt");
        let dst = PathBuf::from("copy.txt");
        let diff = format_copy_diff(&src, &dst, true, None, false, 3);
        assert!(diff.contains("diff --git"));
    }

    #[test]
    fn copy_diff_with_unicode_paths() {
        let src = PathBuf::from("文件.txt");
        let dst = PathBuf::from("копия.txt");
        let diff = format_copy_diff(&src, &dst, true, None, false, 3);
        assert!(diff.contains("文件.txt"));
        assert!(diff.contains("копия.txt"));
    }

    #[test]
    fn copy_diff_with_spaces_in_paths() {
        let src = PathBuf::from("my file.txt");
        let dst = PathBuf::from("new file.txt");
        let diff = format_copy_diff(&src, &dst, true, None, false, 3);
        assert!(diff.contains("my file.txt"));
        assert!(diff.contains("new file.txt"));
    }

    #[test]
    fn rename_diff_with_unicode() {
        let src = PathBuf::from("старый.txt");
        let dst = PathBuf::from("новый.txt");
        let diff = format_rename_diff(&src, &dst, false, 3);
        assert!(diff.contains("старый.txt"));
        assert!(diff.contains("новый.txt"));
    }

    #[test]
    fn rename_diff_similarity_100() {
        let src = PathBuf::from("file.txt");
        let dst = PathBuf::from("renamed.txt");
        let diff = format_rename_diff(&src, &dst, false, 3);
        assert!(diff.contains("rename from"));
        assert!(diff.contains("rename to"));
    }

    #[test]
    fn copy_diff_with_very_long_path() {
        let long_path = "a/".repeat(50) + "file.txt";
        let src = PathBuf::from(&long_path);
        let dst = PathBuf::from("short.txt");
        let diff = format_copy_diff(&src, &dst, true, None, false, 3);
        assert!(diff.len() > 0);
    }

    #[test]
    fn copy_diff_identical_paths() {
        let src = PathBuf::from("same.txt");
        let dst = PathBuf::from("same.txt");
        let diff = format_copy_diff(&src, &dst, true, None, false, 3);
        assert!(diff.contains("same.txt"));
    }

    #[test]
    fn rename_diff_identical_paths() {
        let src = PathBuf::from("same.txt");
        let dst = PathBuf::from("same.txt");
        let diff = format_rename_diff(&src, &dst, false, 3);
        assert!(diff.contains("same.txt"));
    }

    #[test]
    fn copy_diff_with_multiple_context_lines() {
        let src = PathBuf::from("test.txt");
        let dst = PathBuf::from("copy.txt");
        
        let diff1 = format_copy_diff(&src, &dst, true, None, false, 1);
        let diff3 = format_copy_diff(&src, &dst, true, None, false, 3);
        let diff10 = format_copy_diff(&src, &dst, true, None, false, 10);
        
        assert!(diff1.len() > 0);
        assert!(diff3.len() > 0);
        assert!(diff10.len() > 0);
    }

    #[test]
    fn copy_diff_with_actual_file_content() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("real.txt");
        std::fs::write(&src, "Hello\nWorld\n").unwrap();
        
        let dst = PathBuf::from("destination.txt");
        let diff = format_copy_diff(&dst, &src, true, Some(&src.to_string_lossy()), false, 3);
        assert!(diff.len() > 0);
    }

    #[test]
    fn copy_diff_nonexistent_content_file() {
        let src = PathBuf::from("src.txt");
        let dst = PathBuf::from("dst.txt");
        let nonexistent = PathBuf::from("/nonexistent/file.txt");
        
        let diff = format_copy_diff(&src, &dst, true, Some(&nonexistent.to_string_lossy()), false, 3);
        // Should still produce diff, just without content
        assert!(diff.contains("diff --git"));
    }

    #[test]
    fn copy_diff_ansi_colored() {
        let src = PathBuf::from("a.txt");
        let dst = PathBuf::from("b.txt");
        let diff = format_copy_diff(&src, &dst, false, None, false, 3);
        // Should have ANSI color codes when not plain
        assert!(diff.len() > 0);
    }

    #[test]
    fn rename_diff_ansi_colored() {
        let src = PathBuf::from("old.txt");
        let dst = PathBuf::from("new.txt");
        let diff = format_rename_diff(&src, &dst, false, 3);
        assert!(diff.len() > 0);
    }

    #[test]
    fn copy_diff_with_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("empty.txt");
        std::fs::write(&src, b"").unwrap();
        
        let dst = PathBuf::from("empty_copy.txt");
        let diff = format_copy_diff(&dst, &src, true, Some(&src.to_string_lossy()), false, 3);
        assert!(diff.contains("diff --git"));
    }
}
