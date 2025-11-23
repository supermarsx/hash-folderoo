use crate::algorithms::{
    Blake2bHasher, Blake2bpHasher, Blake3Hasher, K12Hasher, ParallelHash256Hasher, Shake256Hasher,
    TurboShake256Hasher, WyHashExpander, Xxh3Expander,
};
use crate::hash::HasherImpl;

#[derive(Clone, Copy)]
pub enum Algorithm {
    Blake2b,
    Blake2bp,
    Blake3,
    Shake256,
    K12,
    TurboShake256,
    ParallelHash256,
    Xxh3_1024,
    Wyhash1024,
}

impl Algorithm {
    pub fn all() -> &'static [Algorithm] {
        &[
            Algorithm::Blake2b,
            Algorithm::Blake2bp,
            Algorithm::Blake3,
            Algorithm::Shake256,
            Algorithm::K12,
            Algorithm::TurboShake256,
            Algorithm::ParallelHash256,
            Algorithm::Xxh3_1024,
            Algorithm::Wyhash1024,
        ]
    }

    pub fn list() -> Vec<&'static str> {
        Self::all().iter().map(|alg| alg.name()).collect()
    }

    pub fn from_str(name: &str) -> Option<Algorithm> {
        match name.to_lowercase().as_str() {
            "blake2b" | "blake2b-512" => Some(Algorithm::Blake2b),
            "blake2bp" => Some(Algorithm::Blake2bp),
            "blake3" => Some(Algorithm::Blake3),
            "shake256" => Some(Algorithm::Shake256),
            "k12" | "kangarootwelve" | "kangaroo12" => Some(Algorithm::K12),
            "turboshake" | "turboshake256" => Some(Algorithm::TurboShake256),
            "parallelhash" | "parallelhash256" => Some(Algorithm::ParallelHash256),
            "xxh3" | "xxh3-1024" => Some(Algorithm::Xxh3_1024),
            "wyhash" | "wyhash-1024" => Some(Algorithm::Wyhash1024),
            _ => None,
        }
    }

    pub fn create(&self) -> Box<dyn HasherImpl> {
        match self {
            Algorithm::Blake2b => Blake2bHasher::new_boxed(),
            Algorithm::Blake2bp => Blake2bpHasher::new_boxed(),
            Algorithm::Blake3 => Blake3Hasher::new_boxed(),
            Algorithm::Shake256 => Shake256Hasher::new_boxed(),
            Algorithm::K12 => K12Hasher::new_boxed(),
            Algorithm::TurboShake256 => TurboShake256Hasher::new_boxed(),
            Algorithm::ParallelHash256 => ParallelHash256Hasher::new_boxed(),
            Algorithm::Xxh3_1024 => Xxh3Expander::new_boxed(),
            Algorithm::Wyhash1024 => WyHashExpander::new_boxed(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Algorithm::Blake2b => "blake2b",
            Algorithm::Blake2bp => "blake2bp",
            Algorithm::Blake3 => "blake3",
            Algorithm::Shake256 => "shake256",
            Algorithm::K12 => "k12",
            Algorithm::TurboShake256 => "turboshake256",
            Algorithm::ParallelHash256 => "parallelhash256",
            Algorithm::Xxh3_1024 => "xxh3-1024",
            Algorithm::Wyhash1024 => "wyhash-1024",
        }
    }
}
