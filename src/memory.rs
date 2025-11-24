use anyhow::Result;
use log::warn;
use std::sync::{Arc, Mutex};
use sysinfo::{System, SystemExt};

/// Memory usage modes for the hashing engine.
#[derive(Debug, Clone, Copy)]
pub enum MemoryMode {
    Stream,
    Balanced,
    Booster,
}

impl MemoryMode {
    pub fn from_name(s: &str) -> Self {
        s.parse().unwrap_or(MemoryMode::Balanced)
    }
}

impl std::str::FromStr for MemoryMode {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stream" => Ok(MemoryMode::Stream),
            "booster" => Ok(MemoryMode::Booster),
            "balanced" => Ok(MemoryMode::Balanced),
            _ => Err(()),
        }
    }
}

/// A pool of reusable byte buffers to reduce allocation churn.
///
/// The pool stores Vec<u8> buffers and hands out a PooledBuffer wrapper which
/// returns the buffer to the pool on Drop.
#[derive(Clone)]
pub struct BufferPool {
    inner: Arc<Mutex<Vec<Vec<u8>>>>,
    buf_size: usize,
}

impl BufferPool {
    /// Create a new pool with `num_buffers` buffers preallocated to `buf_size`.
    pub fn new(num_buffers: usize, buf_size: usize) -> Self {
        let mut v = Vec::with_capacity(num_buffers);
        for _ in 0..num_buffers {
            v.push(vec![0u8; buf_size]);
        }
        Self {
            inner: Arc::new(Mutex::new(v)),
            buf_size,
        }
    }

    /// Get a buffer from the pool. If none are available, allocate a fresh one.
    pub fn get(&self) -> PooledBuffer {
        if let Ok(mut guard) = self.inner.lock() {
            if let Some(mut b) = guard.pop() {
                // ensure capacity
                b.resize(self.buf_size, 0u8);
                return PooledBuffer {
                    buf: Some(b),
                    pool: Some(self.inner.clone()),
                };
            }
        }
        // fallback: allocate
        PooledBuffer {
            buf: Some(vec![0u8; self.buf_size]),
            pool: Some(self.inner.clone()),
        }
    }

    /// Return a buffer to the pool manually.
    pub fn put(&self, mut buf: Vec<u8>) {
        // Normalize buffer size to configured buf_size
        buf.resize(self.buf_size, 0u8);
        if let Ok(mut guard) = self.inner.lock() {
            guard.push(buf);
        }
    }

    /// Get configured buffer size.
    pub fn buf_size(&self) -> usize {
        self.buf_size
    }
}

/// A wrapper that returns its buffer to the pool when dropped.
pub struct PooledBuffer {
    buf: Option<Vec<u8>>,
    pool: Option<Arc<Mutex<Vec<Vec<u8>>>>>,
}

impl PooledBuffer {
    /// Take ownership of the inner Vec<u8>.
    pub fn into_inner(mut self) -> Vec<u8> {
        self.buf.take().unwrap_or_default()
    }

    /// Get a mutable reference to the underlying buffer.
    ///
    /// Named `as_mut_slice` to avoid colliding with `AsMut::as_mut` trait method.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.buf.as_mut().map(|b| &mut b[..]).unwrap_or(&mut [])
    }

    /// Get a shared slice.
    pub fn as_slice(&self) -> &[u8] {
        self.buf.as_ref().map(|b| &b[..]).unwrap_or(&[])
    }
}

impl AsMut<[u8]> for PooledBuffer {
    fn as_mut(&mut self) -> &mut [u8] {
        self.buf.as_mut().map(|b| &mut b[..]).unwrap_or(&mut [])
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let (Some(b), Some(pool)) = (self.buf.take(), self.pool.take()) {
            // reset length to configured size for predictability
            // Note: we can't access buf_size here, so just push as-is.
            if let Ok(mut guard) = pool.lock() {
                guard.push(b);
            }
        }
    }
}

/// Detect total system RAM in bytes. Uses sysinfo.
pub fn detect_system_ram_bytes() -> Result<u64> {
    let mut sys = System::new();
    // Refresh memory to get up-to-date values.
    sys.refresh_memory();
    // sys.total_memory() returns KB according to sysinfo docs; convert to bytes.
    let kb = sys.total_memory();
    Ok(kb * 1024)
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryPlan {
    pub mode: MemoryMode,
    pub threads: usize,
    pub buffer_size: usize,
    pub num_buffers: usize,
    pub prefetch_listing: bool,
}

impl MemoryPlan {
    pub fn total_buffer_bytes(&self) -> u64 {
        (self.buffer_size as u64).saturating_mul(self.num_buffers as u64)
    }
}

/// Recommend configuration (threads, buffer_size, num_buffers) based on RAM and MemoryMode.
pub fn recommend_config(
    mode: MemoryMode,
    threads_override: Option<usize>,
    max_ram_override: Option<u64>,
) -> Result<MemoryPlan> {
    let detected_ram = detect_system_ram_bytes().unwrap_or(2 * 1024 * 1024 * 1024);
    let ram_budget = max_ram_override.unwrap_or(detected_ram).max(64 * 1024) as u128;

    // Determine number of logical CPUs available
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    // base heuristics
    let (mut threads, buf_size, buffers_per_thread) = match mode {
        MemoryMode::Stream => {
            let threads = std::cmp::max(1, cpus / 2);
            let buf_size = 64 * 1024; // 64KB
            let buffers_per_thread = 2;
            (threads, buf_size, buffers_per_thread)
        }
        MemoryMode::Balanced => {
            let threads = cpus;
            let buf_size = 256 * 1024; // 256KB
            let buffers_per_thread = 4;
            (threads, buf_size, buffers_per_thread)
        }
        MemoryMode::Booster => {
            let threads = std::cmp::max(1, cpus * 2);
            let buf_size = 1024 * 1024; // 1MB
            let buffers_per_thread = 6;
            (threads, buf_size, buffers_per_thread)
        }
    };

    if let Some(t_override) = threads_override {
        if t_override > 0 {
            threads = t_override;
        }
    }

    let desired_total_buffers = threads.saturating_mul(buffers_per_thread).max(1);
    let desired_memory = (desired_total_buffers as u128) * (buf_size as u128);

    let max_allowed = ram_budget.max(buf_size as u128);
    let mut num_buffers = desired_total_buffers;
    let mut scaled = false;
    if desired_memory > max_allowed {
        scaled = true;
        let scale = max_allowed as f64 / desired_memory as f64;
        let scaled_buffers = ((desired_total_buffers as f64) * scale).floor() as usize;
        num_buffers = std::cmp::max(1, scaled_buffers);
    }

    if num_buffers < threads {
        threads = num_buffers.max(1);
    }

    let prefetch_listing = !matches!(mode, MemoryMode::Stream);

    let plan = MemoryPlan {
        mode,
        threads: threads.max(1),
        buffer_size: buf_size,
        num_buffers: num_buffers.max(1),
        prefetch_listing,
    };

    if scaled {
        warn!(
            "memory plan scaled down due to budget (buffers {} -> {} totaling {:.2} MiB)",
            desired_total_buffers,
            plan.num_buffers,
            plan.total_buffer_bytes() as f64 / (1024.0 * 1024.0)
        );
    }

    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommend_config_runs() {
        let plan = recommend_config(MemoryMode::Balanced, None, None).unwrap();
        assert!(plan.threads >= 1);
        assert!(plan.buffer_size >= 64 * 1024);
        assert!(plan.num_buffers >= 1);
    }

    #[test]
    fn test_buffer_pool_basic() {
        let pool = BufferPool::new(2, 1024);
        {
            let mut p1 = pool.get();
            let _p2 = pool.get();
            let s1 = p1.as_mut_slice();
            if !s1.is_empty() {
                s1[0] = 42;
            }
            // p1 and p2 dropped here and returned
        }
        // after drops, we should be able to get buffers again
        let _ = pool.get();
        let _ = pool.get();
    }
    #[test]
    fn plan_respects_max_ram() {
        let plan = recommend_config(MemoryMode::Booster, None, Some(2 * 1024 * 1024)).unwrap();
        assert!(plan.total_buffer_bytes() <= 2 * 1024 * 1024);
        assert!(plan.num_buffers >= 1);
    }
}
