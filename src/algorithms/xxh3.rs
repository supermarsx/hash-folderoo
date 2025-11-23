use crate::hash::{AlgorithmInfo, HasherImpl};
use xxhash_rust::xxh3::{xxh3_64_with_seed, Xxh3};

/// XXH3-based expander that produces deterministic 1024-bit (or arbitrary length) output.
/// Non-cryptographic: meant for fast comparisons/benchmarks only.
pub struct Xxh3Expander {
    state: Xxh3,
}

impl Xxh3Expander {
    pub fn new() -> Self {
        Self { state: Xxh3::new() }
    }

    fn expand_from_seed(seed: u64, out_len: usize) -> Vec<u8> {
        if out_len == 0 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(out_len);
        let mut counter = 0u64;
        let mut tweak = seed;
        while out.len() < out_len {
            let mut block_input = [0u8; 16];
            block_input[..8].copy_from_slice(&seed.to_le_bytes());
            block_input[8..].copy_from_slice(&counter.to_le_bytes());
            let chunk = xxh3_64_with_seed(&block_input, tweak);
            out.extend_from_slice(&chunk.to_le_bytes());
            counter = counter.wrapping_add(1);
            tweak = tweak.wrapping_add(0x9E37_79B1_85EB_CA87);
        }
        out.truncate(out_len);
        out
    }
}

impl Default for Xxh3Expander {
    fn default() -> Self {
        Self::new()
    }
}

impl HasherImpl for Xxh3Expander {
    fn name(&self) -> &str {
        "xxh3-1024"
    }

    fn info(&self) -> AlgorithmInfo {
        AlgorithmInfo {
            name: "xxh3-1024".to_string(),
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
        self.state.update(data);
    }

    fn finalize_hex(&self, out_len: usize) -> String {
        let seed = self.state.clone().digest();
        let bytes = Self::expand_from_seed(seed, out_len);
        hex::encode(bytes)
    }
}
