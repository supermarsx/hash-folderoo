use std::io::Cursor;
use std::time::Instant;

use anyhow::Result;

use crate::algorithms::Algorithm;

/// Run a benchmark for `algorithm` over a buffer of `size_mb` megabytes.
/// If `size_mb` is 0 a sensible default (64 MB) is used.
/// If `algorithm == "all"` each available algorithm will be benchmarked.
pub fn run_benchmark(algorithm: &str, size_mb: usize) -> Result<()> {
    let size_mb = if size_mb == 0 { 64 } else { size_mb };
    let buf_size = size_mb
        .checked_mul(1024 * 1024)
        .ok_or_else(|| anyhow::anyhow!("size overflow"))?;

    // Allocate a deterministic buffer (zeros). Random is nicer but not required here.
    let buf = vec![0u8; buf_size];

    if algorithm.eq_ignore_ascii_case("all") {
        for alg_name in Algorithm::list() {
            // call the same function for each algorithm name
            run_benchmark(alg_name, size_mb)?;
        }
        return Ok(());
    }

    let alg_enum = match Algorithm::from_name(algorithm) {
        Some(a) => a,
        None => {
            println!(
                "Unknown algorithm '{}', available: {:?}",
                algorithm,
                Algorithm::list()
            );
            return Ok(());
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

    Ok(())
}
