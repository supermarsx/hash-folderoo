use crate::hash::{AlgorithmInfo, HasherImpl};
use sha3::{
    digest::{ExtendableOutput, Update},
    Shake256,
};
use std::io::Read;

pub struct Shake256Hasher {
    hasher: Shake256,
}

impl Shake256Hasher {
    pub fn new() -> Self {
        Self {
            hasher: Shake256::default(),
        }
    }
}

impl Default for Shake256Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl HasherImpl for Shake256Hasher {
    fn name(&self) -> &str {
        "shake256"
    }

    fn info(&self) -> AlgorithmInfo {
        AlgorithmInfo {
            name: "shake256".to_string(),
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
        // Clone hasher to avoid consuming original state when creating XOF reader
        let mut reader = {
            let cloned = self.hasher.clone();
            cloned.finalize_xof()
        };
        let mut out = vec![0u8; out_len];
        Read::read_exact(&mut reader, &mut out).unwrap();
        hex::encode(out)
    }
}
