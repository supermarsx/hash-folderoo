use crate::hash::{AlgorithmInfo, HasherImpl};
use anyhow::Result;
use sha3::{Shake256, digest::{Update, ExtendableOutput, XofReader}};
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

    fn update_reader(&mut self, r: &mut dyn Read) -> Result<()> {
        let mut buf = [0u8; 8192];
        loop {
            let n = r.read(&mut buf)?;
            if n == 0 {
                break;
            }
            self.hasher.update(&buf[..n]);
        }
        Ok(())
    }

    fn finalize_hex(&self, out_len: usize) -> String {
        // Clone hasher to avoid consuming original state when creating XOF reader
        let mut reader = {
            let mut cloned = self.hasher.clone();
            cloned.finalize_xof()
        };
        let mut out = vec![0u8; out_len];
        use std::io::Read as _;
        let _ = reader.read_exact(&mut out);
        hex::encode(out)
    }
}