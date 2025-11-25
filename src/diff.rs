use std::path::Path;

/// Simple helper to format git-style diffs for file operations.
/// These are lightweight, primarily human-reviewable strings (not full patch metadata).

fn read_lines_opt(p: &Path) -> Option<Vec<String>> {
    match std::fs::read_to_string(p) {
        Ok(s) => Some(s.lines().map(|l| l.to_string()).collect()),
        Err(_) => None,
    }
}

pub fn format_copy_diff(
    src: &Path,
    dst: &Path,
    new_file: bool,
    conflict: Option<&str>,
    include_patch: bool,
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
        if let Some(src_lines) = read_lines_opt(src) {
            let dst_lines = read_lines_opt(dst).unwrap_or_default();
            let src_len = src_lines.len();
            let dst_len = dst_lines.len();
            let max = std::cmp::max(src_len, dst_len);
            out.push_str(&format!("@@ -1,{} +1,{} @@\n", src_len, dst_len));
            for i in 0..max {
                match (src_lines.get(i), dst_lines.get(i)) {
                    (Some(sv), Some(dv)) => {
                        if sv == dv {
                            out.push_str(&format!(" {}\n", sv));
                        } else {
                            out.push_str(&format!("-{}\n", sv));
                            out.push_str(&format!("+{}\n", dv));
                        }
                    }
                    (Some(sv), None) => out.push_str(&format!("-{}\n", sv)),
                    (None, Some(dv)) => out.push_str(&format!("+{}\n", dv)),
                    (None, None) => {}
                }
            }
            out.push_str("\n");
        }
    }
    out
}

pub fn format_rename_diff(src: &Path, dst: &Path, include_patch: bool) -> String {
    let src_s = src.to_string_lossy();
    let dst_s = dst.to_string_lossy();
    let mut out = format!(
        "diff --git a/{0} b/{1}\nrename from {0}\nrename to   {1}\n\n",
        src_s, dst_s
    );
    if include_patch {
        if let Some(src_lines) = read_lines_opt(src) {
            let dst_lines = read_lines_opt(dst).unwrap_or_default();
            let src_len = src_lines.len();
            let dst_len = dst_lines.len();
            let max = std::cmp::max(src_len, dst_len);
            out.push_str(&format!("@@ -1,{} +1,{} @@\n", src_len, dst_len));
            for i in 0..max {
                match (src_lines.get(i), dst_lines.get(i)) {
                    (Some(sv), Some(dv)) => {
                        if sv == dv {
                            out.push_str(&format!(" {}\n", sv));
                        } else {
                            out.push_str(&format!("-{}\n", sv));
                            out.push_str(&format!("+{}\n", dv));
                        }
                    }
                    (Some(sv), None) => out.push_str(&format!("-{}\n", sv)),
                    (None, Some(dv)) => out.push_str(&format!("+{}\n", dv)),
                    (None, None) => {}
                }
            }
            out.push_str("\n");
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
        let s = format_copy_diff(&src, &dst, true, None, false);
        assert!(s.contains("diff --git a/a/foo.txt b/b/foo.txt"));
        assert!(s.contains("new file mode"));
    }

    #[test]
    fn rename_diff_contains_paths() {
        let src = PathBuf::from("a/old.txt");
        let dst = PathBuf::from("a/new.txt");
        let s = format_rename_diff(&src, &dst, false);
        assert!(s.contains("rename from a/old.txt"));
        assert!(s.contains("rename to   a/new.txt"));
    }
}
