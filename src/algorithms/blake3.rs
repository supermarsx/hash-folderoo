use crate::hash::{AlgorithmInfo, HasherImpl};
use blake3::{Hasher, OutputReader};
use std::io::Read;

pub struct Blake3Hasher {
    hasher: Hasher,
}

impl Blake3Hasher {
    pub fn new() -> Self {
        Self {
            hasher: Hasher::new(),
        }
    }
}

impl Default for Blake3Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl HasherImpl for Blake3Hasher {
    fn name(&self) -> &str {
        "blake3"
    }

    fn info(&self) -> AlgorithmInfo {
        AlgorithmInfo {
            name: "blake3".to_string(),
            is_cryptographic: true,
            supports_xof: true,
            output_len_default: 32, // 256-bit default
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
        // Use XOF output reader to produce arbitrary length
        let mut reader: OutputReader = self.hasher.finalize_xof();
        let mut out = vec![0u8; out_len];
        // XOF reader requires mutable reader; clone hasher state by re-finalizing
        // Note: blake3::Hasher::finalize_xof consumes &self; using finalize_xof above is fine
        // but OutputReader implements read
        Read::read_exact(&mut reader, &mut out).unwrap();
        hex::encode(out)
    }
}
