use crate::algorithms::{
    Blake2bHasher, Blake2bpHasher, Blake3Hasher, K12Hasher, ParallelHash256Hasher,
    Shake256Hasher, TurboShake256Hasher, WyHashExpander, Xxh3Expander,
};
use crate::hash::HasherImpl;
use std::io::Read;

#[cfg(test)]
mod tests {
    use super::*;
    use blake2b_simd::{blake2bp, Params};
    use blake3;
    use sha3::{digest::{ExtendableOutput, Update}, Shake256};
    use tiny_keccak::{IntoXof, ParallelHash, Xof};
    use tiny_keccak::{Hasher as TKHasher, KangarooTwelve};
    use turboshake::TurboShake256;
    use wyhash::WyHash;
    use xxhash_rust::xxh3::{xxh3_64_with_seed, Xxh3};

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

    #[test]
    fn parallelhash_matches_direct() {
        let inputs: &[&[u8]] = &[b"", b"hello", b"The quick brown fox"];
        for &inp in inputs {
            let mut h = ParallelHash256Hasher::new();
            h.update_reader(&mut &inp[..]).unwrap();
            let got = h.finalize_hex(64);

            let mut ref_hasher = ParallelHash::v256(b"", 8192);
            ref_hasher.update(inp);
            let mut xof = ref_hasher.into_xof();
            let mut out = vec![0u8; 64];
            xof.squeeze(&mut out);
            let exp = hex::encode(out);

            assert_eq!(got, exp, "parallelhash mismatch for input {:?}", inp);
        }
    }

    fn expand_seed(seed: u64, out_len: usize) -> Vec<u8> {
        if out_len == 0 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(out_len);
        let mut counter = 0u64;
        let mut tweak = seed;
        while out.len() < out_len {
            let mut block_input = [0u8; 16];
            block_input[..8].copy_from_slice(&seed.to_le_bytes());
            block_input[8..].copy_from_slice(&counter.to_le_bytes());
            let chunk = xxh3_64_with_seed(&block_input, tweak);
            out.extend_from_slice(&chunk.to_le_bytes());
            counter = counter.wrapping_add(1);
            tweak = tweak.wrapping_add(0x9E37_79B1_85EB_CA87);
        }
        out.truncate(out_len);
        out
    }

    #[test]
    fn xxh3_expander_matches_reference() {
        let inputs: &[&[u8]] = &[b"", b"hello", b"The quick brown fox"];
        for &inp in inputs {
            let mut h = Xxh3Expander::new();
            h.update_reader(&mut &inp[..]).unwrap();
            let got = h.finalize_hex(128);

            let mut ref_hasher = Xxh3::new();
            ref_hasher.update(inp);
            let seed = ref_hasher.digest();
            let expected = expand_seed(seed, 128);
            assert_eq!(got, hex::encode(expected), "xxh3 mismatch for {:?}", inp);
        }
    }

    fn wyhash_expand(seed: u64, out_len: usize) -> Vec<u8> {
        if out_len == 0 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(out_len);
        let mut counter = 0u64;
        let mut current_seed = seed;
        while out.len() < out_len {
            let mut hasher = WyHash::with_seed(current_seed);
            hasher.write(&counter.to_le_bytes());
            let chunk = hasher.finish();
            out.extend_from_slice(&chunk.to_le_bytes());
            counter = counter.wrapping_add(1);
            current_seed = current_seed.wrapping_add(0xA076_1D64_78BD_642F);
        }
        out.truncate(out_len);
        out
    }

    #[test]
    fn wyhash_expander_matches_reference() {
        let inputs: &[&[u8]] = &[b"", b"hello", b"The quick brown fox"];
        for &inp in inputs {
            let mut h = WyHashExpander::new();
            h.update_reader(&mut &inp[..]).unwrap();
            let got = h.finalize_hex(128);

            let mut ref_hasher = WyHash::with_seed(0);
            ref_hasher.write(inp);
            let seed = ref_hasher.finish();
            let expected = wyhash_expand(seed, 128);
            assert_eq!(got, hex::encode(expected), "wyhash mismatch for {:?}", inp);
        }
    }
}
