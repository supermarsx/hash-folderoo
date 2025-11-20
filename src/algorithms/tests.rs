use crate::algorithms::{Blake3Hasher, Shake256Hasher};
use crate::hash::HasherImpl;
use std::io::Read;

#[cfg(test)]
mod tests {
    use super::*;
    use blake3;
    use sha3::{Shake256, digest::{Update, ExtendableOutput}};

    #[test]
    fn blake3_matches_direct() {
        // inputs to test
        let inputs: &[&[u8]] = &[b"", b"hello", b"The quick brown fox"];
        for &inp in inputs {
            let mut h = Blake3Hasher::new();
            h.update_reader(&mut &inp[..]).unwrap();
            let got = h.finalize_hex(32);

            let mut hasher = blake3::Hasher::new();
            hasher.update(inp);
            let mut reader = hasher.finalize_xof();
            let mut out = vec![0u8; 32];
            reader.read_exact(&mut out).unwrap();
            let exp = hex::encode(out);

            assert_eq!(got, exp, "blake3 mismatch for input {:?}", inp);
        }
    }

    #[test]
    fn shake256_matches_direct() {
        let inputs: &[&[u8]] = &[b"", b"hello", b"The quick brown fox"];
        for &inp in inputs {
            let mut h = Shake256Hasher::new();
            h.update_reader(&mut &inp[..]).unwrap();
            let got = h.finalize_hex(32);

            let mut hasher = Shake256::default();
            hasher.update(inp);
            let mut reader = hasher.finalize_xof();
            let mut out = vec![0u8; 32];
            reader.read_exact(&mut out).unwrap();
            let exp = hex::encode(out);

            assert_eq!(got, exp, "shake256 mismatch for input {:?}", inp);
        }
    }
}