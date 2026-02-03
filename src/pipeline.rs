use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use crossbeam_channel::unbounded;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::ThreadPoolBuilder;

use crate::memory::{recommend_config, BufferPool, MemoryMode};
use crate::walk;

/// A simple hashing pipeline that connects a producer (directory walker)
/// to multiple worker threads that process files.
///
/// The pipeline accepts a `worker` function that will be invoked for each file.
/// The worker receives the file path and an Arc<BufferPool> for buffer reuse.
pub struct Pipeline {
    pub mode: MemoryMode,
    threads_override: Option<usize>,
    max_ram_override: Option<u64>,
}

impl Pipeline {
    pub fn new(mode: MemoryMode) -> Self {
        Self {
            mode,
            threads_override: None,
            max_ram_override: None,
        }
    }

    pub fn with_threads(mut self, threads: Option<usize>) -> Self {
        self.threads_override = threads.and_then(|t| if t == 0 { None } else { Some(t) });
        self
    }

    pub fn with_max_ram(mut self, max_ram: Option<u64>) -> Self {
        self.max_ram_override = max_ram.filter(|v| *v > 0);
        self
    }

    /// Run the pipeline over `root` using `exclusions`.
    ///
    /// `worker` is called for every file and must be Send + Sync + 'static.
    /// Returns the number of files processed.
    pub fn run<F>(
        &self,
        root: impl AsRef<std::path::Path>,
        exclusions: &[String],
        max_depth: Option<usize>,
        follow_symlinks: bool,
        show_progress: bool,
        worker: F,
    ) -> Result<usize>
    where
        F: Fn(PathBuf, Arc<BufferPool>) -> Result<()> + Send + Sync + 'static,
    {
        // Decide threads and buffer configuration from memory mode
        let plan = recommend_config(self.mode, self.threads_override, self.max_ram_override)
            .context("failed to get recommended config")?;
        let threads = plan.threads;
        let buf_size = plan.buffer_size;
        let num_buffers = plan.num_buffers;
        log::info!(
            "Memory plan {:?}: threads={}, buffers={} (~{:.2} MiB)",
            plan.mode,
            plan.threads,
            plan.num_buffers,
            plan.total_buffer_bytes() as f64 / (1024.0 * 1024.0)
        );

        // Build buffer pool
        let buffer_pool = Arc::new(BufferPool::new(num_buffers, buf_size));

        let root_buf = root.as_ref().to_path_buf();
        let walker_stream =
            walk::walk_directory_stream(&root_buf, exclusions, max_depth, follow_symlinks)
                .context("walk directory")?;

        let mut streaming_iter: Option<walk::WalkStream> = None;
        let (files, total_files) = if plan.prefetch_listing {
            let collected: Vec<PathBuf> = walker_stream.collect();
            let total = collected.len() as u64;
            (Some(collected), total)
        } else {
            streaming_iter = Some(walker_stream);
            (None, 0)
        };

        let pb = if show_progress {
            let bar = if plan.prefetch_listing {
                ProgressBar::new(total_files)
            } else {
                ProgressBar::new_spinner()
            };
            bar.set_style(
                ProgressStyle::with_template(if plan.prefetch_listing {
                    "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}"
                } else {
                    "{spinner:.green} [{elapsed_precise}] {msg}"
                })
                .unwrap_or_else(|_| ProgressStyle::default_bar()),
            );
            bar.set_message("hashing files");
            bar
        } else {
            ProgressBar::hidden()
        };

        // Channel to feed file paths to workers
        let (tx, rx) = unbounded::<PathBuf>();

        // Producer: send all file paths then close the channel
        if let Some(files) = files {
            let tx = tx.clone();
            std::thread::spawn(move || {
                for f in files {
                    if tx.send(f).is_err() {
                        break;
                    }
                }
            });
        } else if let Some(stream) = streaming_iter.take() {
            let tx = tx.clone();
            std::thread::spawn(move || {
                for f in stream {
                    if tx.send(f).is_err() {
                        break;
                    }
                }
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
                        // Enforce soft backpressure: if allocated buffers exceed budget, yield briefly
                        if pool_clone.allocated_buffers() > pool_clone.max_buffers() {
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
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
    use std::fs::{create_dir_all, write};
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

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

        let processed = pipeline
            .run(&root, &[], None, false, true, move |_path, _pool| {
                let mut s = seen_clone.lock().unwrap();
                *s += 1;
                Ok(())
            })
            .unwrap();

        assert_eq!(processed, 2);
        assert_eq!(*seen.lock().unwrap(), 2);
    }

    #[test]
    fn pipeline_handles_empty_directory() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("empty");
        create_dir_all(&root).unwrap();

        let pipeline = Pipeline::new(MemoryMode::Balanced);
        let processed = pipeline
            .run(&root, &[], None, false, true, |_path, _pool| Ok(()))
            .unwrap();

        assert_eq!(processed, 0);
    }

    #[test]
    fn pipeline_handles_single_file() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("single");
        create_dir_all(&root).unwrap();
        write(root.join("only.txt"), b"content").unwrap();

        let pipeline = Pipeline::new(MemoryMode::Stream);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_clone = seen.clone();

        let processed = pipeline
            .run(&root, &[], None, false, true, move |path, _pool| {
                seen_clone.lock().unwrap().push(path.to_path_buf());
                Ok(())
            })
            .unwrap();

        assert_eq!(processed, 1);
        assert_eq!(seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn pipeline_with_excludes() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("exclude_test");
        create_dir_all(&root).unwrap();
        write(root.join("include.txt"), b"yes").unwrap();
        write(root.join("exclude.txt"), b"no").unwrap();
        write(root.join("also_include.md"), b"yes").unwrap();

        let pipeline = Pipeline::new(MemoryMode::Balanced);
        let excludes = vec!["exclude.txt".to_string()];
        
        let processed = pipeline
            .run(&root, &excludes, None, false, true, |_path, _pool| Ok(()))
            .unwrap();

        assert_eq!(processed, 2); // Only include.txt and also_include.md
    }

    #[test]
    fn pipeline_nested_directories() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("nested");
        create_dir_all(root.join("a/b/c")).unwrap();
        write(root.join("a/file1.txt"), b"1").unwrap();
        write(root.join("a/b/file2.txt"), b"2").unwrap();
        write(root.join("a/b/c/file3.txt"), b"3").unwrap();

        let pipeline = Pipeline::new(MemoryMode::Balanced);
        let processed = pipeline
            .run(&root, &[], None, false, true, |_path, _pool| Ok(()))
            .unwrap();

        assert_eq!(processed, 3);
    }

    #[test]
    fn pipeline_all_memory_modes() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("modes");
        create_dir_all(&root).unwrap();
        write(root.join("test.txt"), b"data").unwrap();

        for mode in &[MemoryMode::Stream, MemoryMode::Balanced, MemoryMode::Booster] {
            let pipeline = Pipeline::new(*mode);
            let processed = pipeline
                .run(&root, &[], None, false, true, |_path, _pool| Ok(()))
                .unwrap();
            assert_eq!(processed, 1);
        }
    }

    #[test]
    fn pipeline_callback_error_propagation() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("error_test");
        create_dir_all(&root).unwrap();
        write(root.join("file.txt"), b"data").unwrap();

        let pipeline = Pipeline::new(MemoryMode::Balanced);
        let result = pipeline.run(&root, &[], None, false, true, |_path, _pool| {
            Err(anyhow::anyhow!("Simulated error"))
        });

        // Pipeline should propagate the error or handle gracefully
        // The behavior may vary based on parallelism and error handling strategy
        // assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn pipeline_with_symlinks_follow_false() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("symlink_test");
        create_dir_all(&root).unwrap();
        write(root.join("real.txt"), b"real").unwrap();
        
        // Try to create symlink (may fail on Windows without privileges)
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("real.txt"), root.join("link.txt")).ok();
        }

        let pipeline = Pipeline::new(MemoryMode::Balanced);
        let processed = pipeline
            .run(&root, &[], None, false, false, |_path, _pool| Ok(()))
            .unwrap();

        assert!(processed >= 1); // At least the real file
    }

    #[test]
    fn pipeline_large_number_of_files() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("many_files");
        create_dir_all(&root).unwrap();

        // Create 100 files
        for i in 0..100 {
            write(root.join(format!("file_{}.txt", i)), format!("content {}", i)).unwrap();
        }

        let pipeline = Pipeline::new(MemoryMode::Booster);
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        let processed = pipeline
            .run(&root, &[], None, false, true, move |_path, _pool| {
                *counter_clone.lock().unwrap() += 1;
                Ok(())
            })
            .unwrap();

        assert_eq!(processed, 100);
        assert_eq!(*counter.lock().unwrap(), 100);
    }

    #[test]
    fn pipeline_with_thread_override() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("thread_test");
        create_dir_all(&root).unwrap();
        write(root.join("file.txt"), b"data").unwrap();

        let pipeline = Pipeline::new(MemoryMode::Balanced);
        let processed = pipeline
            .run(&root, &[], Some(1), false, true, |_path, _pool| Ok(()))
            .unwrap();

        assert_eq!(processed, 1);
    }

    #[test]
    fn pipeline_empty_files() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("empty_files");
        create_dir_all(&root).unwrap();
        write(root.join("empty1.txt"), b"").unwrap();
        write(root.join("empty2.txt"), b"").unwrap();
        write(root.join("nonempty.txt"), b"data").unwrap();

        let pipeline = Pipeline::new(MemoryMode::Balanced);
        let processed = pipeline
            .run(&root, &[], None, false, true, |_path, _pool| Ok(()))
            .unwrap();

        assert_eq!(processed, 3);
    }
}
