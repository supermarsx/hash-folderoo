use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Atomically write bytes to `path`.
/// Writes to a temporary file in the same directory and then renames it into place.
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent dir {:?}", parent))?;
    }

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("tempfile");
    // temp file hidden in same dir
    let tmp_name = format!(".{}.tmp", file_name);
    let tmp_path = path.with_file_name(tmp_name);

    {
        let mut tmp = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp_path)
            .with_context(|| format!("open temp file {:?}", tmp_path))?;
        tmp.write_all(data)
            .with_context(|| format!("write to temp file {:?}", tmp_path))?;
        tmp.sync_all()
            .with_context(|| format!("sync temp file {:?}", tmp_path))?;
    }

    // On Windows rename fails if target exists — remove first if present
    if path.exists() {
        fs::remove_file(path).with_context(|| format!("remove existing target file {:?}", path))?;
    }

    fs::rename(&tmp_path, path)
        .with_context(|| format!("rename temp file {:?} -> {:?}", tmp_path, path))?;

    Ok(())
}

/// Serialize `value` as pretty JSON and atomically write to `path`.
pub fn write_json<T: ?Sized + Serialize>(path: &Path, value: &T) -> Result<()> {
    let data = serde_json::to_vec_pretty(value).context("serialize json")?;
    atomic_write(path, &data)
}

/// Serialize `records` to CSV and atomically write to `path`.
pub fn write_csv<T: Serialize>(path: &Path, records: &[T]) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(vec![]);
    for rec in records {
        wtr.serialize(rec).context("serialize csv record")?;
    }
    let data = wtr.into_inner().context("finalize csv writer")?;
    atomic_write(path, &data)
}

/// MapEntry used for persistent maps (json/csv) and for in-memory comparisons.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MapEntry {
    pub path: String,
    pub hash: String,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtime: Option<i64>,
}

/// Load a map from a JSON file. Accepts either:
/// - an object with an "entries" field containing an array of MapEntry
/// - a top-level array of MapEntry
pub fn load_map_from_json(path: &Path) -> Result<Vec<MapEntry>> {
    let s = std::fs::read_to_string(path).with_context(|| format!("read json {:?}", path))?;
    let v: serde_json::Value = serde_json::from_str(&s).context("parse json")?;

    // Try object with entries first
    if let Some(entries) = v.get("entries") {
        let entries_parsed: Vec<MapEntry> =
            serde_json::from_value(entries.clone()).context("deserialize entries")?;
        return Ok(entries_parsed);
    }

    // If top-level array
    if v.is_array() {
        let entries_parsed: Vec<MapEntry> =
            serde_json::from_value(v).context("deserialize array")?;
        return Ok(entries_parsed);
    }

    // Try to deserialize into a wrapper that matches older formats
    // Fallback: attempt to deserialize whole file as Vec<MapEntry>
    let entries_parsed: Vec<MapEntry> =
        serde_json::from_str(&s).context("deserialize as Vec<MapEntry>")?;
    Ok(entries_parsed)
}

/// Load a map from CSV file. Expects headers matching MapEntry fields.
pub fn load_map_from_csv(path: &Path) -> Result<Vec<MapEntry>> {
    let mut rdr = csv::Reader::from_path(path).with_context(|| format!("open csv {:?}", path))?;
    let mut out = Vec::new();
    for result in rdr.deserialize() {
        let rec: MapEntry = result.context("deserialize csv record")?;
        out.push(rec);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_json_array() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("m.json");
        let v = vec![
            MapEntry {
                path: "a".into(),
                hash: "h1".into(),
                size: 1,
                mtime: None,
            },
            MapEntry {
                path: "b".into(),
                hash: "h2".into(),
                size: 2,
                mtime: None,
            },
        ];
        write_json(&p, &v).unwrap();
        let loaded = load_map_from_json(&p).unwrap();
        assert_eq!(loaded, v);
    }

    #[test]
    fn roundtrip_csv() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("m.csv");
        let v = vec![
            MapEntry {
                path: "a".into(),
                hash: "h1".into(),
                size: 1,
                mtime: None,
            },
            MapEntry {
                path: "b".into(),
                hash: "h2".into(),
                size: 2,
                mtime: None,
            },
        ];
        write_csv(&p, &v).unwrap();
        let loaded = load_map_from_csv(&p).unwrap();
        assert_eq!(loaded, v);
    }

    #[test]
    fn json_handles_empty_array() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("empty.json");
        let v: Vec<MapEntry> = vec![];
        write_json(&p, &v).unwrap();
        let loaded = load_map_from_json(&p).unwrap();
        assert_eq!(loaded.len(), 0);
    }

    #[test]
    fn csv_handles_empty_array() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("empty.csv");
        let v: Vec<MapEntry> = vec![];
        write_csv(&p, &v).unwrap();
        let loaded = load_map_from_csv(&p).unwrap();
        assert_eq!(loaded.len(), 0);
    }

    #[test]
    fn json_handles_special_characters() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("special.json");
        let v = vec![MapEntry {
            path: "file with spaces & \"quotes\" and 'apostrophes'.txt".into(),
            hash: "abc123".into(),
            size: 100,
            mtime: Some(1234567890),
        }];
        write_json(&p, &v).unwrap();
        let loaded = load_map_from_json(&p).unwrap();
        assert_eq!(loaded, v);
    }

    #[test]
    fn csv_handles_special_characters() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("special.csv");
        let v = vec![MapEntry {
            path: "file,with,commas.txt".into(),
            hash: "hash\"with\"quotes".into(),
            size: 999,
            mtime: Some(9999999),
        }];
        write_csv(&p, &v).unwrap();
        let loaded = load_map_from_csv(&p).unwrap();
        assert_eq!(loaded, v);
    }

    #[test]
    fn json_handles_unicode() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("unicode.json");
        let v = vec![MapEntry {
            path: "文件名.txt".into(),
            hash: "🔥hash🔥".into(),
            size: 42,
            mtime: None,
        }];
        write_json(&p, &v).unwrap();
        let loaded = load_map_from_json(&p).unwrap();
        assert_eq!(loaded, v);
    }

    #[test]
    fn csv_handles_unicode() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("unicode.csv");
        let v = vec![MapEntry {
            path: "файл.txt".into(),
            hash: "хеш".into(),
            size: 777,
            mtime: Some(1000),
        }];
        write_csv(&p, &v).unwrap();
        let loaded = load_map_from_csv(&p).unwrap();
        assert_eq!(loaded, v);
    }

    #[test]
    fn json_handles_large_dataset() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("large.json");
        let v: Vec<MapEntry> = (0..1000)
            .map(|i| MapEntry {
                path: format!("file_{}.txt", i),
                hash: format!("hash_{}", i),
                size: i as u64,
                mtime: Some(i as i64),
            })
            .collect();
        write_json(&p, &v).unwrap();
        let loaded = load_map_from_json(&p).unwrap();
        assert_eq!(loaded.len(), 1000);
        assert_eq!(loaded[0].path, "file_0.txt");
        assert_eq!(loaded[999].path, "file_999.txt");
    }

    #[test]
    fn csv_handles_large_dataset() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("large.csv");
        let v: Vec<MapEntry> = (0..1000)
            .map(|i| MapEntry {
                path: format!("file_{}.txt", i),
                hash: format!("hash_{}", i),
                size: i as u64,
                mtime: Some(i as i64),
            })
            .collect();
        write_csv(&p, &v).unwrap();
        let loaded = load_map_from_csv(&p).unwrap();
        assert_eq!(loaded.len(), 1000);
    }

    #[test]
    fn json_with_none_mtime() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("no_mtime.json");
        let v = vec![MapEntry {
            path: "test.txt".into(),
            hash: "hash123".into(),
            size: 100,
            mtime: None,
        }];
        write_json(&p, &v).unwrap();
        let loaded = load_map_from_json(&p).unwrap();
        assert_eq!(loaded[0].mtime, None);
    }

    #[test]
    fn csv_with_none_mtime() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("no_mtime.csv");
        let v = vec![MapEntry {
            path: "test.txt".into(),
            hash: "hash123".into(),
            size: 100,
            mtime: None,
        }];
        write_csv(&p, &v).unwrap();
        let loaded = load_map_from_csv(&p).unwrap();
        assert_eq!(loaded[0].mtime, None);
    }

    #[test]
    fn json_handles_very_long_paths() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("long_paths.json");
        let long_path = "a/".repeat(100) + "file.txt";
        let v = vec![MapEntry {
            path: long_path.clone(),
            hash: "hash".into(),
            size: 1,
            mtime: None,
        }];
        write_json(&p, &v).unwrap();
        let loaded = load_map_from_json(&p).unwrap();
        assert_eq!(loaded[0].path, long_path);
    }

    #[test]
    fn json_handles_very_long_hashes() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("long_hash.json");
        let long_hash = "a".repeat(10000);
        let v = vec![MapEntry {
            path: "file.txt".into(),
            hash: long_hash.clone(),
            size: 1,
            mtime: None,
        }];
        write_json(&p, &v).unwrap();
        let loaded = load_map_from_json(&p).unwrap();
        assert_eq!(loaded[0].hash, long_hash);
    }

    #[test]
    fn json_handles_zero_size() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("zero_size.json");
        let v = vec![MapEntry {
            path: "empty.txt".into(),
            hash: "empty_hash".into(),
            size: 0,
            mtime: None,
        }];
        write_json(&p, &v).unwrap();
        let loaded = load_map_from_json(&p).unwrap();
        assert_eq!(loaded[0].size, 0);
    }

    #[test]
    fn json_handles_max_u64_size() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("max_size.json");
        let v = vec![MapEntry {
            path: "huge.txt".into(),
            hash: "hash".into(),
            size: u64::MAX,
            mtime: None,
        }];
        write_json(&p, &v).unwrap();
        let loaded = load_map_from_json(&p).unwrap();
        assert_eq!(loaded[0].size, u64::MAX);
    }

    #[test]
    fn load_nonexistent_json_fails() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("nonexistent.json");
        let result = load_map_from_json(&p);
        assert!(result.is_err());
    }

    #[test]
    fn load_nonexistent_csv_fails() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("nonexistent.csv");
        let result = load_map_from_csv(&p);
        assert!(result.is_err());
    }

    #[test]
    fn load_malformed_json_fails() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("malformed.json");
        std::fs::write(&p, b"{ this is not valid json }").unwrap();
        let result = load_map_from_json(&p);
        assert!(result.is_err());
    }
}
