use crate::hash::{AlgorithmInfo, HasherImpl};
use std::hash::Hasher;
use wyhash::WyHash;

/// WyHash-based expander for fast, non-cryptographic 1024-bit digests.
pub struct WyHashExpander {
    state: WyHash,
}

impl WyHashExpander {
    pub fn new() -> Self {
        Self {
            state: WyHash::with_seed(0),
        }
    }

    fn expand_from_seed(seed: u64, out_len: usize) -> Vec<u8> {
        if out_len == 0 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(out_len);
        let mut counter = 0u64;
        let mut current_seed = seed;
        while out.len() < out_len {
            let mut hasher = WyHash::with_seed(current_seed);
            hasher.write(&counter.to_le_bytes());
            let chunk = hasher.finish();
            out.extend_from_slice(&chunk.to_le_bytes());
            counter = counter.wrapping_add(1);
            current_seed = current_seed.wrapping_add(0xA076_1D64_78BD_642F);
        }
        out.truncate(out_len);
        out
    }
}

impl Default for WyHashExpander {
    fn default() -> Self {
        Self::new()
    }
}

impl HasherImpl for WyHashExpander {
    fn name(&self) -> &str {
        "wyhash-1024"
    }

    fn info(&self) -> AlgorithmInfo {
        AlgorithmInfo {
            name: "wyhash-1024".to_string(),
            is_cryptographic: false,
            supports_xof: true,
            output_len_default: 128,
        }
    }

    fn new_boxed() -> Box<dyn HasherImpl>
    where
        Self: Sized,
    {
        Box::new(Self::new())
    }

    fn update(&mut self, data: &[u8]) {
        self.state.write(data);
    }

    fn finalize_hex(&self, out_len: usize) -> String {
        let seed = self.state.finish();
        let bytes = Self::expand_from_seed(seed, out_len);
        hex::encode(bytes)
    }
}
