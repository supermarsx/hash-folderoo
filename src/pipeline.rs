use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use crossbeam_channel::{unbounded, Receiver};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::ThreadPoolBuilder;

use crate::walk;
use crate::memory::{BufferPool, MemoryMode, recommend_config};

/// A simple hashing pipeline that connects a producer (directory walker)
/// to multiple worker threads that process files.
///
/// The pipeline accepts a `worker` function that will be invoked for each file.
/// The worker receives the file path and an Arc<BufferPool> for buffer reuse.
pub struct Pipeline {
    pub mode: MemoryMode,
}

impl Pipeline {
    pub fn new(mode: MemoryMode) -> Self {
        Self { mode }
    }

    /// Run the pipeline over `root` using `exclusions`.
    ///
    /// `worker` is called for every file and must be Send + Sync + 'static.
    /// Returns the number of files processed.
    pub fn run<F>(&self, root: impl AsRef<std::path::Path>, exclusions: &[String], worker: F) -> Result<usize>
    where
        F: Fn(PathBuf, Arc<BufferPool>) -> Result<()> + Send + Sync + 'static,
    {
        // Decide threads and buffer configuration from memory mode
        let (threads, buf_size, num_buffers) = recommend_config(self.mode)
            .context("failed to get recommended config")?;

        // Build buffer pool
        let buffer_pool = Arc::new(BufferPool::new(num_buffers, buf_size));

        // Enumerate files first so we can show a determinate progress bar
        let files = walk::walk_directory(root, exclusions).context("walk directory")?;
        let total = files.len() as u64;

        let pb = ProgressBar::new(total);
        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar()));
        pb.set_message("hashing files");

        // Channel to feed file paths to workers
        let (tx, rx) = unbounded::<PathBuf>();

        // Producer: send all file paths then close the channel
        {
            let tx = tx.clone();
            std::thread::spawn(move || {
                for f in files {
                    if tx.send(f).is_err() {
                        // consumers gone; stop
                        break;
                    }
                }
                // drop tx to close channel
            });
        }

        // Drop the original sender so the channel is closed once the producer finishes.
        drop(tx);

        // Wrap worker in Arc so it can be cloned into threads
        let worker = Arc::new(worker);

        // Build rayon thread pool with configured number of threads
        let pool = ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|i| format!("hash-worker-{}", i))
            .build()
            .context("build rayon thread pool")?;

        // Start workers inside the rayon pool
        pool.install(|| {
            // spawn worker tasks equal to the number of threads
            let mut handles = Vec::with_capacity(threads);
            for _ in 0..threads {
                let rx = rx.clone();
                let worker = worker.clone();
                let pool_clone = buffer_pool.clone();
                let pb = pb.clone();
                // Each rayon task loops over the shared receiver
                handles.push(std::thread::spawn(move || {
                    // Iterate until channel closes
                    for path in rx.iter() {
                        if let Err(e) = (worker)(path, pool_clone.clone()) {
                            log::warn!("worker error: {:?}", e);
                        }
                        pb.inc(1);
                    }
                }));
            }

            // Wait for all spawned threads to finish
            for h in handles {
                let _ = h.join();
            }
        });

        pb.finish_with_message("done");

        Ok(pb.position() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::{create_dir_all, File, write};
    use std::sync::{Arc, Mutex};

    #[test]
    fn pipeline_runs_basic() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        create_dir_all(&root).unwrap();
        write(root.join("a.txt"), b"hello").unwrap();
        write(root.join("b.txt"), b"world").unwrap();

        let pipeline = Pipeline::new(MemoryMode::Balanced);
        let seen = Arc::new(Mutex::new(0usize));
        let seen_clone = seen.clone();

        let processed = pipeline.run(&root, &[], move |_path, _pool| {
            let mut s = seen_clone.lock().unwrap();
            *s += 1;
            Ok(())
        }).unwrap();

        assert_eq!(processed, 2);
        assert_eq!(*seen.lock().unwrap(), 2);
    }
}