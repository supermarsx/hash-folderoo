use thiserror::Error;
use std::env;
use indicatif::ProgressBar;

/// Initialize logging (simple wrapper around env_logger)
pub fn setup_logging() {
    if env::var("RUST_LOG").is_err() {
        // Default to info if user hasn't set RUST_LOG
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
}

/// Simple progress-bar helper (placeholder for later phases)
pub fn new_progress(len: u64) -> ProgressBar {
    ProgressBar::new(len)
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Generic error: {0}")]
    Anyhow(#[from] anyhow::Error),
}