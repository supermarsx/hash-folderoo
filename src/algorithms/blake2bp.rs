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
        if out_len <= bytes.len() {
            let take = out_len.min(bytes.len());
            return hex::encode(&bytes[..take]);
        }

        // Deterministic expansion using blake2bp hashing of seed || counter
        fn expand_seed(seed: &[u8], out_len: usize) -> Vec<u8> {
            if out_len == 0 {
                return vec![];
            }
            let mut out = Vec::with_capacity(out_len);
            let mut counter: u32 = 0;
            while out.len() < out_len {
                let mut input = Vec::with_capacity(seed.len() + 4);
                input.extend_from_slice(seed);
                input.extend_from_slice(&counter.to_le_bytes());
                let chunk = blake2bp::Params::new().hash(&input);
                out.extend_from_slice(chunk.as_bytes());
                counter = counter.wrapping_add(1);
            }
            out.truncate(out_len);
            out
        }

        let expanded = expand_seed(bytes, out_len);
        hex::encode(expanded)
    }
}
