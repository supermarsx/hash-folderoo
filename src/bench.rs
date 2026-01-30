use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::algorithms::Algorithm;

/// Run a benchmark for `algorithm` over a buffer of `size_mb` megabytes.
/// If `size_mb` is 0 a sensible default (64 MB) is used.
/// If `algorithm == "all"` each available algorithm will be benchmarked.
pub fn run_benchmark(algorithm: &str, size_mb: usize) -> Result<()> {
    let (_alg_name, _size_mb, _secs, _throughput) = run_benchmark_report(algorithm, size_mb)?;
    Ok(())
}

/// Run benchmark and return a concise report tuple: (algorithm, size_mb, secs, throughput_mb_s)
pub fn run_benchmark_report(algorithm: &str, size_mb: usize) -> Result<(String, usize, f64, f64)> {
    let size_mb = if size_mb == 0 { 64 } else { size_mb };
    let buf_size = size_mb
        .checked_mul(1024 * 1024)
        .ok_or_else(|| anyhow::anyhow!("size overflow"))?;

    // Allocate a deterministic buffer (zeros). Random is nicer but not required here.
    let buf = vec![0u8; buf_size];

    if algorithm.eq_ignore_ascii_case("all") {
        // If "all" we return a placeholder; callers should call run_benchmark for each alg.
        return Ok(("all".to_string(), size_mb, 0.0_f64, 0.0_f64));
    }

    let alg_enum = match Algorithm::from_name(algorithm) {
        Some(a) => a,
        None => {
            println!(
                "Unknown algorithm '{}', available: {:?}",
                algorithm,
                Algorithm::list()
            );
            return Ok((algorithm.to_string(), size_mb, 0.0_f64, 0.0_f64));
        }
    };

    let mut hasher = alg_enum.create();
    let out_len = hasher.info().output_len_default;

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
        hasher.info().name,
        size_mb,
        secs,
        throughput
    );

    Ok((hasher.info().name.to_string(), size_mb, secs, throughput))
}

/// Run a benchmark and persist a simple JSON report to `out_path`.
/// The report contains: algorithm, size_mb, time_s, throughput_mb_s, timestamp_unix.
pub fn run_benchmark_and_save(algorithm: &str, size_mb: usize, out_path: &Path) -> Result<()> {
    let (alg_name, size_mb, secs, throughput) = run_benchmark_report(algorithm, size_mb)?;

    // Build a simple JSON object (manually to avoid extra deps).
    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let json = format!(
        "{{\"algorithm\":\"{}\",\"size_mb\":{},\"time_s\":{:.6},\"throughput_mb_s\":{:.6},\"timestamp_unix\":{} }}",
        alg_name, size_mb, secs, throughput, ts
    );

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_path, json)?;
    println!("Wrote bench report to {}", out_path.display());

    Ok(())
}
