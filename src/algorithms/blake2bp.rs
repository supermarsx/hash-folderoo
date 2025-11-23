use crate::hash::{AlgorithmInfo, HasherImpl};
use blake2b_simd::blake2bp;

pub struct Blake2bpHasher {
    state: blake2bp::State,
}

impl Blake2bpHasher {
    pub fn new() -> Self {
        Self {
            state: blake2bp::State::new(),
        }
    }
}

impl Default for Blake2bpHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl HasherImpl for Blake2bpHasher {
    fn name(&self) -> &str {
        "blake2bp"
    }

    fn info(&self) -> AlgorithmInfo {
        AlgorithmInfo {
            name: "blake2bp".to_string(),
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
