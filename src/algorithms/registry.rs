use crate::algorithms::{Blake3Hasher, Shake256Hasher};
use crate::hash::HasherImpl;

#[derive(Clone, Copy)]
pub enum Algorithm {
    Blake3,
    Shake256,
}

impl Algorithm {
    pub fn all() -> &'static [Algorithm] {
        &[Algorithm::Blake3, Algorithm::Shake256]
    }

    pub fn list() -> Vec<&'static str> {
        Self::all().iter().map(|alg| alg.name()).collect()
    }

    pub fn from_str(name: &str) -> Option<Algorithm> {
        match name.to_lowercase().as_str() {
            "blake3" => Some(Algorithm::Blake3),
            "shake256" => Some(Algorithm::Shake256),
            _ => None,
        }
    }

    pub fn create(&self) -> Box<dyn HasherImpl> {
        match self {
            Algorithm::Blake3 => Blake3Hasher::new_boxed(),
            Algorithm::Shake256 => Shake256Hasher::new_boxed(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Algorithm::Blake3 => "blake3",
            Algorithm::Shake256 => "shake256",
        }
    }
}
