use std::path::{Path, PathBuf};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use anyhow::{Context, Result};
use serde::{Serialize, Deserialize};

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
        fs::remove_file(path)
            .with_context(|| format!("remove existing target file {:?}", path))?;
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
}

/// Load a map from a JSON file. Accepts either:
/// - an object with an "entries" field containing an array of MapEntry
/// - a top-level array of MapEntry
pub fn load_map_from_json(path: &Path) -> Result<Vec<MapEntry>> {
    let s = std::fs::read_to_string(path).with_context(|| format!("read json {:?}", path))?;
    let v: serde_json::Value = serde_json::from_str(&s).context("parse json")?;

    // Try object with entries first
    if let Some(entries) = v.get("entries") {
        let entries_parsed: Vec<MapEntry> = serde_json::from_value(entries.clone()).context("deserialize entries")?;
        return Ok(entries_parsed);
    }

    // If top-level array
    if v.is_array() {
        let entries_parsed: Vec<MapEntry> = serde_json::from_value(v).context("deserialize array")?;
        return Ok(entries_parsed);
    }

    // Try to deserialize into a wrapper that matches older formats
    // Fallback: attempt to deserialize whole file as Vec<MapEntry>
    let entries_parsed: Vec<MapEntry> = serde_json::from_str(&s).context("deserialize as Vec<MapEntry>")?;
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
    use std::fs::write;

    #[test]
    fn roundtrip_json_array() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("m.json");
        let v = vec![
            MapEntry { path: "a".into(), hash: "h1".into(), size: 1 },
            MapEntry { path: "b".into(), hash: "h2".into(), size: 2 },
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
            MapEntry { path: "a".into(), hash: "h1".into(), size: 1 },
            MapEntry { path: "b".into(), hash: "h2".into(), size: 2 },
        ];
        write_csv(&p, &v).unwrap();
        let loaded = load_map_from_csv(&p).unwrap();
        assert_eq!(loaded, v);
    }
}