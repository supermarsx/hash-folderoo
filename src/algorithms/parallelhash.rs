use crate::hash::{AlgorithmInfo, HasherImpl};
use tiny_keccak::{Hasher as TKHasher, IntoXof, ParallelHash, Xof};

const DEFAULT_BLOCK_SIZE: usize = 8192;

pub struct ParallelHash256Hasher {
    state: ParallelHash,
}

impl ParallelHash256Hasher {
    pub fn new() -> Self {
        Self {
            state: ParallelHash::v256(b"", DEFAULT_BLOCK_SIZE),
        }
    }
}

impl Default for ParallelHash256Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl HasherImpl for ParallelHash256Hasher {
    fn name(&self) -> &str {
        "parallelhash256"
    }

    fn info(&self) -> AlgorithmInfo {
        AlgorithmInfo {
            name: "parallelhash256".to_string(),
            is_cryptographic: true,
            supports_xof: true,
            output_len_default: 32,
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
        let hasher = self.state.clone();
        let mut xof = hasher.into_xof();
        let mut out = vec![0u8; out_len];
        xof.squeeze(&mut out);
        hex::encode(out)
    }
}
