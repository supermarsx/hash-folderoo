use crate::hash::{AlgorithmInfo, HasherImpl};
use blake2b_simd::{Params, State};

pub struct Blake2bHasher {
    state: State,
}

impl Blake2bHasher {
    pub fn new() -> Self {
        let mut params = Params::new();
        params.hash_length(64);
        Self {
            state: params.to_state(),
        }
    }
}

impl Default for Blake2bHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl HasherImpl for Blake2bHasher {
    fn name(&self) -> &str {
        "blake2b"
    }

    fn info(&self) -> AlgorithmInfo {
        AlgorithmInfo {
            name: "blake2b".to_string(),
            is_cryptographic: true,
            supports_xof: false,
            output_len_default: 64,
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
        let hash = self.state.clone().finalize();
        let bytes = hash.as_bytes();
        let take = out_len.min(bytes.len());
        hex::encode(&bytes[..take])
    }
}
