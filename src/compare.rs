use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::algorithms::Algorithm;
use crate::hash::hash_path_with_pool;
use crate::io;
use crate::memory::MemoryMode;
use crate::pipeline::Pipeline;

/// Comparison report describing differences between two maps.
#[derive(Debug, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub identical: Vec<io::MapEntry>,
    /// (source, target)
    pub changed: Vec<(io::MapEntry, io::MapEntry)>,
    /// (source, target) -- same hash, different path
    pub moved: Vec<(io::MapEntry, io::MapEntry)>,
    pub missing: Vec<io::MapEntry>, // in source but not in target
    pub new: Vec<io::MapEntry>,     // in target but not in source
}

impl ComparisonReport {
    pub fn new() -> Self {
        Self {
            identical: Vec::new(),
            changed: Vec::new(),
            moved: Vec::new(),
            missing: Vec::new(),
            new: Vec::new(),
        }
    }
}

impl Default for ComparisonReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Load a map from either a file (json/csv) or by hashing a directory.
/// `input` may be a path to a file (json/csv) or a directory.
/// When hashing a directory the provided `algorithm` is used with balanced memory mode.
pub fn get_map_from_input(input: &str, algorithm: Algorithm) -> Result<Vec<io::MapEntry>> {
    let p = Path::new(input);

    if p.exists() && p.is_file() {
        // Try file extension first
        if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
            match ext.to_lowercase().as_str() {
                "json" => {
                    return io::load_map_from_json(p)
                        .with_context(|| format!("loading json {:?}", p))
                }
                "csv" => {
                    return io::load_map_from_csv(p).with_context(|| format!("loading csv {:?}", p))
                }
                _ => {}
            }
        }

        // Fallback: try json then csv
        if let Ok(m) = io::load_map_from_json(p) {
            return Ok(m);
        }
        if let Ok(m) = io::load_map_from_csv(p) {
            return Ok(m);
        }

        anyhow::bail!("unsupported or invalid map file: {:?}", p);
    }

    if p.exists() && p.is_dir() {
        // Hash the directory using pipeline similar to hashmap command.
        let alg = algorithm;
        let probe = alg.create();
        let out_len = probe.info().output_len_default;

        let pipeline = Pipeline::new(MemoryMode::Balanced);

        let entries: Arc<Mutex<Vec<io::MapEntry>>> = Arc::new(Mutex::new(Vec::new()));
        let entries_clone = entries.clone();

        let alg_for_worker = alg;
        let worker = move |path_buf: PathBuf,
                           buffer_pool: Arc<crate::memory::BufferPool>|
              -> anyhow::Result<()> {
            if !path_buf.is_file() {
                return Ok(());
            }
            let rel = path_buf.to_string_lossy().into_owned();
            let metadata = path_buf.metadata().ok();
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            let mtime = metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|dur| dur.as_secs() as i64);
            let mut hasher = alg_for_worker.create();
            hash_path_with_pool(hasher.as_mut(), &path_buf, &buffer_pool)?;
            let h = hasher.finalize_hex(out_len);
            let me = io::MapEntry {
                path: rel,
                hash: h,
                size,
                mtime,
            };
            let mut guard = entries_clone.lock().unwrap();
            guard.push(me);
            Ok(())
        };

        pipeline
            .run(p, &[], None, false, true, worker)
            .context("running pipeline to build map")?;

        let mut vec = entries.lock().unwrap().clone();
        vec.sort_by(|a, b| a.path.cmp(&b.path));
        return Ok(vec);
    }

    anyhow::bail!("input path does not exist: {}", input);
}

/// Compare two maps (source and target) and produce a ComparisonReport.
///
/// Rules:
/// - Identical: same path present in both with same hash
/// - Changed: same path present in both with different hash
/// - Moved: same hash present in both but different paths (pair source->target)
/// - Missing: entry present in source but its hash not present in target and path not present
/// - New: entry present in target but its hash not present in source and path not present
pub fn compare_maps(source: Vec<io::MapEntry>, target: Vec<io::MapEntry>) -> ComparisonReport {
    use std::collections::HashMap;

    let mut report = ComparisonReport::new();

    let mut src_by_path: HashMap<String, io::MapEntry> = HashMap::new();
    let mut tgt_by_path: HashMap<String, io::MapEntry> = HashMap::new();
    let mut src_by_hash: HashMap<String, Vec<io::MapEntry>> = HashMap::new();
    let mut tgt_by_hash: HashMap<String, Vec<io::MapEntry>> = HashMap::new();

    for e in source.into_iter() {
        src_by_path.insert(e.path.clone(), e.clone());
        src_by_hash.entry(e.hash.clone()).or_default().push(e);
    }
    for e in target.into_iter() {
        tgt_by_path.insert(e.path.clone(), e.clone());
        tgt_by_hash.entry(e.hash.clone()).or_default().push(e);
    }

    // Track which target paths have been accounted for (to avoid double counting as new)
    let mut accounted_target_paths: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    // Process source entries
    for (path, src_entry) in &src_by_path {
        if let Some(tgt_entry) = tgt_by_path.get(path) {
            if src_entry.hash == tgt_entry.hash {
                report.identical.push(src_entry.clone());
                accounted_target_paths.insert(tgt_entry.path.clone());
            } else {
                report.changed.push((src_entry.clone(), tgt_entry.clone()));
                accounted_target_paths.insert(tgt_entry.path.clone());
            }
            continue;
        }

        // No same path in target. If same hash exists somewhere in target -> moved
        if let Some(tgts) = tgt_by_hash.get(&src_entry.hash) {
            // choose the first matching target entry that hasn't been accounted for yet if possible
            let mut chosen: Option<io::MapEntry> = None;
            for te in tgts {
                if !accounted_target_paths.contains(&te.path) {
                    chosen = Some(te.clone());
                    break;
                }
            }
            if chosen.is_none() {
                chosen = tgts.first().cloned();
            }
            if let Some(te) = chosen {
                // Only mark as moved if paths differ
                if te.path != src_entry.path {
                    report.moved.push((src_entry.clone(), te.clone()));
                    accounted_target_paths.insert(te.path.clone());
                    continue;
                }
            }
        }

        // Otherwise it's missing in target
        report.missing.push(src_entry.clone());
    }

    // Process target entries to find new files that weren't matched above
    for (path, tgt_entry) in &tgt_by_path {
        if accounted_target_paths.contains(path) {
            continue;
        }

        // If target hash exists in source_by_hash then it was already handled as moved (but maybe not accounted)
        if let Some(_srcs) = src_by_hash.get(&tgt_entry.hash) {
            // If none of the source paths matched this target path, consider it moved and add pair(s)
            // We skip adding moved here to avoid duplicating; the moved pairs were added when iterating source.
            accounted_target_paths.insert(tgt_entry.path.clone());
            continue;
        }

        // Not present in source => new
        report.new.push(tgt_entry.clone());
        accounted_target_paths.insert(tgt_entry.path.clone());
    }

    report
}

/// Save or print a comparison report.
/// If `output` is Some(path) the report is written to that file, otherwise printed to stdout.
/// `format` is "json" or "csv".
pub fn write_report(report: &ComparisonReport, output: Option<&Path>, format: &str) -> Result<()> {
    let fmt = format.to_lowercase();
    match fmt.as_str() {
        "json" => {
            if let Some(p) = output {
                // write full report as json
                io::write_json(p, report).with_context(|| format!("write json {:?}", p))?;
            } else {
                let data = serde_json::to_vec_pretty(report).context("serialize report to json")?;
                std::io::stdout().write_all(&data)?;
            }
            Ok(())
        }
        "csv" => {
            // Emit a flat CSV with rows describing each observed change.
            #[derive(Serialize)]
            struct Row<'a> {
                status: &'a str,
                source_path: Option<&'a str>,
                source_hash: Option<&'a str>,
                source_size: Option<u64>,
                target_path: Option<&'a str>,
                target_hash: Option<&'a str>,
                target_size: Option<u64>,
            }

            let mut rows: Vec<Row> = Vec::new();
            for r in &report.identical {
                rows.push(Row {
                    status: "identical",
                    source_path: Some(&r.path),
                    source_hash: Some(&r.hash),
                    source_size: Some(r.size),
                    target_path: Some(&r.path),
                    target_hash: Some(&r.hash),
                    target_size: Some(r.size),
                });
            }
            for (s, t) in &report.changed {
                rows.push(Row {
                    status: "changed",
                    source_path: Some(&s.path),
                    source_hash: Some(&s.hash),
                    source_size: Some(s.size),
                    target_path: Some(&t.path),
                    target_hash: Some(&t.hash),
                    target_size: Some(t.size),
                });
            }
            for (s, t) in &report.moved {
                rows.push(Row {
                    status: "moved",
                    source_path: Some(&s.path),
                    source_hash: Some(&s.hash),
                    source_size: Some(s.size),
                    target_path: Some(&t.path),
                    target_hash: Some(&t.hash),
                    target_size: Some(t.size),
                });
            }
            for s in &report.missing {
                rows.push(Row {
                    status: "missing",
                    source_path: Some(&s.path),
                    source_hash: Some(&s.hash),
                    source_size: Some(s.size),
                    target_path: None,
                    target_hash: None,
                    target_size: None,
                });
            }
            for t in &report.new {
                rows.push(Row {
                    status: "new",
                    source_path: None,
                    source_hash: None,
                    source_size: None,
                    target_path: Some(&t.path),
                    target_hash: Some(&t.hash),
                    target_size: Some(t.size),
                });
            }

            if let Some(p) = output {
                io::write_csv(p, &rows).with_context(|| format!("write csv {:?}", p))?;
            } else {
                let mut wtr = csv::Writer::from_writer(std::io::stdout());
                for row in rows {
                    wtr.serialize(row)?;
                }
                wtr.flush()?;
            }
            Ok(())
        }
        other => anyhow::bail!("unsupported format: {}", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_basic() {
        let a = vec![
            io::MapEntry {
                path: "a.txt".into(),
                hash: "h1".into(),
                size: 1,
                mtime: None,
            },
            io::MapEntry {
                path: "b.txt".into(),
                hash: "h2".into(),
                size: 2,
                mtime: None,
            },
            io::MapEntry {
                path: "c.txt".into(),
                hash: "h3".into(),
                size: 3,
                mtime: None,
            },
        ];
        let b = vec![
            io::MapEntry {
                path: "a.txt".into(),
                hash: "h1".into(),
                size: 1,
                mtime: None,
            }, // identical
            io::MapEntry {
                path: "b.txt".into(),
                hash: "h2b".into(),
                size: 2,
                mtime: None,
            }, // changed
            io::MapEntry {
                path: "d.txt".into(),
                hash: "h3".into(),
                size: 3,
                mtime: None,
            }, // moved (c -> d)
            io::MapEntry {
                path: "e.txt".into(),
                hash: "h4".into(),
                size: 4,
                mtime: None,
            }, // new
        ];

        let r = compare_maps(a, b);
        assert_eq!(r.identical.len(), 1);
        assert_eq!(r.changed.len(), 1);
        assert_eq!(r.moved.len(), 1);
        assert_eq!(r.missing.len(), 0);
        assert_eq!(r.new.len(), 1);
    }
}
