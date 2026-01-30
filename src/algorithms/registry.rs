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

    pub fn from_name(name: &str) -> Option<Algorithm> {
        name.parse().ok()
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

    /// Whether this algorithm supports eXtendable-Output (XOF) semantics.
    pub fn is_xof(&self) -> bool {
        match self {
            Algorithm::Shake256 => true,
            Algorithm::TurboShake256 => true,
            Algorithm::K12 => true,
            Algorithm::ParallelHash256 => true,
            // The remaining algorithms are fixed-output
            Algorithm::Blake2b
            | Algorithm::Blake2bp
            | Algorithm::Blake3
            | Algorithm::Xxh3_1024
            | Algorithm::Wyhash1024 => false,
        }
    }
}

impl std::str::FromStr for Algorithm {
    type Err = ();
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name.to_lowercase().as_str() {
            "blake2b" | "blake2b-512" => Ok(Algorithm::Blake2b),
            "blake2bp" => Ok(Algorithm::Blake2bp),
            "blake3" => Ok(Algorithm::Blake3),
            "shake256" => Ok(Algorithm::Shake256),
            "k12" | "kangarootwelve" | "kangaroo12" => Ok(Algorithm::K12),
            "turboshake" | "turboshake256" => Ok(Algorithm::TurboShake256),
            "parallelhash" | "parallelhash256" => Ok(Algorithm::ParallelHash256),
            "xxh3" | "xxh3-1024" => Ok(Algorithm::Xxh3_1024),
            "wyhash" | "wyhash-1024" => Ok(Algorithm::Wyhash1024),
            _ => Err(()),
        }
    }
}
