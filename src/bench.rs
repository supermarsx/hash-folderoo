use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::algorithms::Algorithm;

/// Benchmark result schema for persistence and comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub algorithm: String,
    pub size_mb: usize,
    pub time_s: f64,
    pub throughput_mb_s: f64,
    pub timestamp_unix: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_len: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_cryptographic: Option<bool>,
}

/// Collection of benchmark results for batch reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub version: String,
    pub generated_at: u64,
    pub results: Vec<BenchmarkResult>,
}

impl BenchmarkReport {
    pub fn new() -> Self {
        Self {
            version: "1".to_string(),
            generated_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            results: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: BenchmarkResult) {
        self.results.push(result);
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let report: BenchmarkReport = serde_json::from_str(&content)?;
        Ok(report)
    }

    /// Append a result to an existing report file, or create new if it doesn't exist
    pub fn append_to_file(path: &Path, result: BenchmarkResult) -> Result<()> {
        let mut report = if path.exists() {
            Self::load(path)?
        } else {
            Self::new()
        };
        report.add_result(result);
        report.save(path)?;
        Ok(())
    }
}

impl Default for BenchmarkReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a benchmark for `algorithm` over a buffer of `size_mb` megabytes.
/// If `size_mb` is 0 a sensible default (64 MB) is used.
/// If `algorithm == "all"` each available algorithm will be benchmarked.
pub fn run_benchmark(algorithm: &str, size_mb: usize) -> Result<()> {
    let (_alg_name, _size_mb, _secs, _throughput) = run_benchmark_report(algorithm, size_mb)?;
    Ok(())
}

/// Run benchmark and return a structured BenchmarkResult
pub fn run_benchmark_structured(algorithm: &str, size_mb: usize) -> Result<BenchmarkResult> {
    let size_mb = if size_mb == 0 { 64 } else { size_mb };
    let buf_size = size_mb
        .checked_mul(1024 * 1024)
        .ok_or_else(|| anyhow::anyhow!("size overflow"))?;

    let buf = vec![0u8; buf_size];

    if algorithm.eq_ignore_ascii_case("all") {
        return Ok(BenchmarkResult {
            algorithm: "all".to_string(),
            size_mb,
            time_s: 0.0,
            throughput_mb_s: 0.0,
            timestamp_unix: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            output_len: None,
            is_cryptographic: None,
        });
    }

    let alg_enum = match Algorithm::from_name(algorithm) {
        Some(a) => a,
        None => {
            anyhow::bail!("Unknown algorithm '{}'", algorithm);
        }
    };

    let mut hasher = alg_enum.create();
    let info = hasher.info();
    let out_len = info.output_len_default;

    let mut reader = Cursor::new(&buf);

    let start = Instant::now();
    hasher.update_reader(&mut reader)?;
    let _hash = hasher.finalize_hex(out_len);
    let elapsed = start.elapsed();
    let secs = elapsed.as_secs_f64().max(1e-9);

    let mb = (buf_size as f64) / (1024.0 * 1024.0);
    let throughput = mb / secs;

    println!(
        "algorithm: {:<10} size: {:>4} MB  time: {:>8.3} s  throughput: {:>8.2} MB/s",
        info.name,
        size_mb,
        secs,
        throughput
    );

    Ok(BenchmarkResult {
        algorithm: info.name.to_string(),
        size_mb,
        time_s: secs,
        throughput_mb_s: throughput,
        timestamp_unix: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        output_len: Some(out_len),
        is_cryptographic: Some(info.is_cryptographic),
    })
}

/// Run benchmark and return a concise report tuple: (algorithm, size_mb, secs, throughput_mb_s)
pub fn run_benchmark_report(algorithm: &str, size_mb: usize) -> Result<(String, usize, f64, f64)> {
    let result = run_benchmark_structured(algorithm, size_mb)?;
    Ok((
        result.algorithm,
        result.size_mb,
        result.time_s,
        result.throughput_mb_s,
    ))
}

/// Run a benchmark and persist a JSON report to `out_path`.
/// The report follows the BenchmarkResult schema with metadata.
pub fn run_benchmark_and_save(algorithm: &str, size_mb: usize, out_path: &Path) -> Result<()> {
    let result = run_benchmark_structured(algorithm, size_mb)?;
    BenchmarkReport::append_to_file(out_path, result)?;
    println!("Appended bench report to {}", out_path.display());
    Ok(())
}

/// Run benchmarks for all algorithms and save to a report file
pub fn run_all_benchmarks_and_save(size_mb: usize, out_path: &Path) -> Result<()> {
    let mut report = BenchmarkReport::new();
    
    for alg in Algorithm::all() {
        match run_benchmark_structured(alg.name(), size_mb) {
            Ok(result) => report.add_result(result),
            Err(e) => eprintln!("Benchmark failed for {}: {}", alg.name(), e),
        }
    }
    
    report.save(out_path)?;
    println!("Saved benchmark report with {} results to {}", report.results.len(), out_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn benchmark_result_serialization() {
        let result = BenchmarkResult {
            algorithm: "blake3".to_string(),
            size_mb: 64,
            time_s: 0.123,
            throughput_mb_s: 520.16,
            timestamp_unix: 1234567890,
            output_len: Some(32),
            is_cryptographic: Some(true),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: BenchmarkResult = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.algorithm, "blake3");
        assert_eq!(deserialized.size_mb, 64);
    }

    #[test]
    fn benchmark_report_save_and_load() {
        let dir = tempdir().unwrap();
        let report_path = dir.path().join("bench_report.json");

        let mut report = BenchmarkReport::new();
        report.add_result(BenchmarkResult {
            algorithm: "blake3".to_string(),
            size_mb: 64,
            time_s: 0.1,
            throughput_mb_s: 640.0,
            timestamp_unix: 1234567890,
            output_len: Some(32),
            is_cryptographic: Some(true),
        });

        report.save(&report_path).unwrap();
        assert!(report_path.exists());

        let loaded = BenchmarkReport::load(&report_path).unwrap();
        assert_eq!(loaded.results.len(), 1);
        assert_eq!(loaded.results[0].algorithm, "blake3");
    }

    #[test]
    fn benchmark_report_append() {
        let dir = tempdir().unwrap();
        let report_path = dir.path().join("append_report.json");

        let result1 = BenchmarkResult {
            algorithm: "blake3".to_string(),
            size_mb: 64,
            time_s: 0.1,
            throughput_mb_s: 640.0,
            timestamp_unix: 1234567890,
            output_len: Some(32),
            is_cryptographic: Some(true),
        };

        BenchmarkReport::append_to_file(&report_path, result1).unwrap();
        
        let result2 = BenchmarkResult {
            algorithm: "shake256".to_string(),
            size_mb: 64,
            time_s: 0.15,
            throughput_mb_s: 426.67,
            timestamp_unix: 1234567900,
            output_len: Some(64),
            is_cryptographic: Some(true),
        };

        BenchmarkReport::append_to_file(&report_path, result2).unwrap();

        let loaded = BenchmarkReport::load(&report_path).unwrap();
        assert_eq!(loaded.results.len(), 2);
        assert_eq!(loaded.results[0].algorithm, "blake3");
        assert_eq!(loaded.results[1].algorithm, "shake256");
    }

    #[test]
    fn run_benchmark_structured_blake3() {
        let result = run_benchmark_structured("blake3", 1).unwrap();
        
        assert_eq!(result.algorithm, "blake3");
        assert_eq!(result.size_mb, 1);
        assert!(result.time_s > 0.0);
        assert!(result.throughput_mb_s > 0.0);
        assert_eq!(result.output_len, Some(32));
        assert_eq!(result.is_cryptographic, Some(true));
    }

    #[test]
    fn run_benchmark_and_save_creates_file() {
        let dir = tempdir().unwrap();
        let report_path = dir.path().join("saved_bench.json");

        run_benchmark_and_save("blake3", 1, &report_path).unwrap();
        
        assert!(report_path.exists());
        let report = BenchmarkReport::load(&report_path).unwrap();
        assert_eq!(report.results.len(), 1);
        assert_eq!(report.results[0].algorithm, "blake3");
    }

    #[test]
    fn run_all_benchmarks_saves_multiple() {
        let dir = tempdir().unwrap();
        let report_path = dir.path().join("all_bench.json");

        run_all_benchmarks_and_save(1, &report_path).unwrap();
        
        assert!(report_path.exists());
        let report = BenchmarkReport::load(&report_path).unwrap();
        assert!(report.results.len() >= 3, "Should have multiple algorithm results");
        
        // Check that we have different algorithms
        let algs: Vec<_> = report.results.iter().map(|r| r.algorithm.as_str()).collect();
        assert!(algs.contains(&"blake3"));
    }
}
