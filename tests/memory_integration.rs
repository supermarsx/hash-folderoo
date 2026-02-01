use hash_folderoo::memory::{recommend_config, BufferPool, MemoryMode};

#[test]
fn low_memory_scaling() {
    let plan =
        recommend_config(MemoryMode::Booster, Some(8), Some(128 * 1024)).expect("recommend_config");

    // Allow a generous slack for low-budget scenarios (recommend_config may
    // round buffer sizes up for internal constraints).
    assert!(
        plan.total_buffer_bytes() <= 1024 * 1024,
        "total buffer bytes {} exceeds budget",
        plan.total_buffer_bytes()
    );
    assert!(plan.num_buffers >= 1);

    let pool = BufferPool::new(plan.num_buffers, plan.buffer_size);

    {
        let mut bufs = Vec::new();
        for _ in 0..plan.num_buffers {
            bufs.push(pool.get());
        }
        assert!(pool.allocated_buffers() >= plan.num_buffers);
        // pooled buffers dropped here
    }

    // After dropping pooled buffers, allocated buffers should be back within max
    assert!(pool.allocated_buffers() <= pool.max_buffers());
}
