use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use crate::algorithms::Algorithm;
use crate::memory::BufferPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmInfo {
    pub name: String,
    pub is_cryptographic: bool,
    pub supports_xof: bool,
    pub output_len_default: usize, // bytes
}

pub trait HasherImpl: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn info(&self) -> AlgorithmInfo;
    fn new_boxed() -> Box<dyn HasherImpl>
    where
        Self: Sized;
    fn update(&mut self, data: &[u8]);
    fn update_reader(&mut self, r: &mut dyn Read) -> Result<()> {
        let mut buf = [0u8; 8192];
        loop {
            let n = r.read(&mut buf)?;
            if n == 0 {
                break;
            }
            self.update(&buf[..n]);
        }
        Ok(())
    }
    fn finalize_hex(&self, out_len: usize) -> String; // out_len in bytes
}

/// Stream file contents located at `path` into the provided hasher using buffers
/// sourced from `buffer_pool`.
pub fn hash_path_with_pool(
    hasher: &mut dyn HasherImpl,
    path: &Path,
    buffer_pool: &Arc<BufferPool>,
) -> Result<()> {
    let mut file = File::open(path)?;
    let mut pooled = buffer_pool.get();
    loop {
        let buf = pooled.as_mut();
        let read = file.read(buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(())
}

/// Deterministic expansion for algorithms.
/// - For XOF algorithms, produce `out_len` bytes via the algorithm's XOF interface.
/// - For fixed-output algorithms, produce output by computing
///   H(input || 0x00 || counter) || H(input || 0x00 || counter+1) ...
///   where H is the algorithm's native fixed-output hash function.
pub fn expand_digest(alg: &Algorithm, input: &[u8], out_len: usize) -> Vec<u8> {
    if out_len == 0 {
        return Vec::new();
    }

    if alg.is_xof() {
        let mut hasher = alg.create();
        hasher.update(input);
        let hex = hasher.finalize_hex(out_len);
        return hex::decode(hex).expect("hex decode");
    }

    // Fixed-output deterministic expansion
    // Use the algorithm's native output length as the chunk size
    let native_len = {
        let h = alg.create();
        h.info().output_len_default
    };

    let mut out = Vec::with_capacity(out_len);
    let mut counter: u32 = 0;
    while out.len() < out_len {
        let mut h = alg.create();
        h.update(input);
        h.update(&[0u8]);
        h.update(&counter.to_le_bytes());
        let hex = h.finalize_hex(native_len);
        let chunk = hex::decode(hex).expect("hex decode");
        out.extend_from_slice(&chunk);
        counter = counter.wrapping_add(1);
    }
    out.truncate(out_len);
    out
}
