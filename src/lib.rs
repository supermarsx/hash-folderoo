pub mod hash;
pub mod algorithms;
pub mod config;
pub mod cli;
pub mod utils;
pub mod walk;
pub mod io;
pub mod memory;
pub mod pipeline;
pub mod compare;
pub mod copy;
pub mod removempty;
pub mod renamer;
pub mod bench;
pub mod report;

pub use hash::{HasherImpl, AlgorithmInfo};
pub use config::RuntimeConfig;
pub use memory::{MemoryMode, BufferPool};
pub use pipeline::Pipeline;
pub use removempty::remove_empty_directories;
pub use renamer::rename_files;

pub use bench::run_benchmark;
pub use report::generate_report;
