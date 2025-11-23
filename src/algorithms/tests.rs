use crate::algorithms::{
    Blake2bHasher, Blake2bpHasher, Blake3Hasher, K12Hasher, Shake256Hasher,
};
use crate::hash::HasherImpl;
use std::io::Read;

#[cfg(test)]
mod tests {
    use super::*;
    use blake2b_simd::{blake2bp, Params};
    use blake3;
    use sha3::{digest::{ExtendableOutput, Update}, Shake256};
    use tiny_keccak::{Hasher as TKHasher, KangarooTwelve};
    use turboshake::TurboShake256;

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

    #[test]
    fn blake2b_matches_direct() {
        let inputs: &[&[u8]] = &[b"", b"hello", b"The quick brown fox"];
        for &inp in inputs {
            let mut h = Blake2bHasher::new();
            h.update_reader(&mut &inp[..]).unwrap();
            let got = h.finalize_hex(64);

            let mut params = Params::new();
            params.hash_length(64);
            let mut state = params.to_state();
            state.update(inp);
            let hash = state.finalize();
            let exp = hex::encode(hash.as_bytes());

            assert_eq!(got, exp, "blake2b mismatch for input {:?}", inp);
        }
    }

    #[test]
    fn blake2bp_matches_direct() {
        let inputs: &[&[u8]] = &[b"", b"hello", b"The quick brown fox"];
        for &inp in inputs {
            let mut h = Blake2bpHasher::new();
            h.update_reader(&mut &inp[..]).unwrap();
            let got = h.finalize_hex(64);

            let expected = blake2bp::Params::new().hash(inp);
            assert_eq!(got, expected.to_hex().as_str(), "blake2bp mismatch for {:?}", inp);
        }
    }

    #[test]
    fn k12_matches_direct() {
        let inputs: &[&[u8]] = &[b"", b"hello", b"The quick brown fox"];
        for &inp in inputs {
            let mut h = K12Hasher::new();
            h.update_reader(&mut &inp[..]).unwrap();
            let got = h.finalize_hex(64);

            let mut hasher = KangarooTwelve::new();
            hasher.update(inp);
            let mut out = vec![0u8; 64];
            hasher.finalize(&mut out);
            let exp = hex::encode(out);

            assert_eq!(got, exp, "k12 mismatch for input {:?}", inp);
        }
    }

    #[test]
    fn turboshake_matches_direct() {
        let inputs: &[&[u8]] = &[b"", b"hello", b"The quick brown fox"];
        for &inp in inputs {
            let mut h = TurboShake256Hasher::new();
            h.update_reader(&mut &inp[..]).unwrap();
            let got = h.finalize_hex(64);

            let mut ref_hasher = TurboShake256::default();
            ref_hasher.absorb(inp).unwrap();
            ref_hasher
                .finalize::<{ TurboShake256::DEFAULT_DOMAIN_SEPARATOR }>()
                .unwrap();
            let mut out = vec![0u8; 64];
            ref_hasher.squeeze(&mut out).unwrap();
            let exp = hex::encode(out);

            assert_eq!(got, exp, "turboshake mismatch for input {:?}", inp);
        }
    }
}
