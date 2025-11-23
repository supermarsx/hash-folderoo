use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

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
