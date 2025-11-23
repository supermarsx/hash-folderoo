use crate::hash::{AlgorithmInfo, HasherImpl};
use turboshake::TurboShake256;

pub struct TurboShake256Hasher {
    state: TurboShake256,
}

impl TurboShake256Hasher {
    pub fn new() -> Self {
        Self {
            state: TurboShake256::default(),
        }
    }
}

impl Default for TurboShake256Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl HasherImpl for TurboShake256Hasher {
    fn name(&self) -> &str {
        "turboshake256"
    }

    fn info(&self) -> AlgorithmInfo {
        AlgorithmInfo {
            name: "turboshake256".to_string(),
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
        self.state
            .absorb(data)
            .expect("TurboSHAKE absorb should not fail before finalize");
    }

    fn finalize_hex(&self, out_len: usize) -> String {
        let mut clone = self.state.clone();
        clone
            .finalize::<{ TurboShake256::DEFAULT_DOMAIN_SEPARATOR }>()
            .expect("TurboSHAKE finalize should not fail");
        let mut out = vec![0u8; out_len];
        clone
            .squeeze(&mut out)
            .expect("TurboSHAKE squeeze should not fail");
        hex::encode(out)
    }
}
