pub mod algorithms;
pub mod bench;
pub mod cli;
pub mod compare;
pub mod config;
pub mod copy;
pub mod hash;
pub mod io;
pub mod memory;
pub mod pipeline;
pub mod removempty;
pub mod renamer;
pub mod report;
pub mod utils;
pub mod walk;

pub use config::RuntimeConfig;
pub use hash::{AlgorithmInfo, HasherImpl};
pub use memory::{BufferPool, MemoryMode};
pub use pipeline::Pipeline;
pub use removempty::remove_empty_directories;
pub use renamer::rename_files;

pub use bench::run_benchmark;
pub use report::generate_report;
