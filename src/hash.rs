use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::Read;

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
    fn update_reader(&mut self, r: &mut dyn Read) -> Result<()>;
    fn finalize_hex(&self, out_len: usize) -> String; // out_len in bytes
}

