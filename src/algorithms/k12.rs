use crate::hash::{AlgorithmInfo, HasherImpl};
use tiny_keccak::{Hasher as TKHasher, KangarooTwelve};

pub struct K12Hasher {
    hasher: KangarooTwelve<&'static [u8]>,
}

impl K12Hasher {
    pub fn new() -> Self {
        Self {
            hasher: KangarooTwelve::new(b""),
        }
    }
}

impl Default for K12Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl HasherImpl for K12Hasher {
    fn name(&self) -> &str {
        "k12"
    }

    fn info(&self) -> AlgorithmInfo {
        AlgorithmInfo {
            name: "k12".to_string(),
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
        self.hasher.update(data);
    }

    fn finalize_hex(&self, out_len: usize) -> String {
        let cloned = self.hasher.clone();
        let mut out = vec![0u8; out_len];
        cloned.finalize(&mut out);
        hex::encode(out)
    }
}
