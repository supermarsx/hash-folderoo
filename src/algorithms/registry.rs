use crate::hash::HasherImpl;
use crate::algorithms::Blake3Hasher;

pub enum Algorithm {
    Blake3,
}

impl Algorithm {
    pub fn list() -> Vec<&'static str> {
        vec!["blake3"]
    }

    pub fn from_str(name: &str) -> Option<Algorithm> {
        match name.to_lowercase().as_str() {
            "blake3" => Some(Algorithm::Blake3),
            _ => None,
        }
    }

    pub fn create(&self) -> Box<dyn HasherImpl> {
        match self {
            Algorithm::Blake3 => Blake3Hasher::new_boxed(),
        }
    }
}
