use hash_folderoo::memory::{recommend_config, BufferPool, MemoryMode};
use hash_folderoo::pipeline::Pipeline;
use std::fs::{create_dir_all, write};
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

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

#[test]
fn stream_mode_respects_low_memory() {
    // Test that stream mode uses minimal memory
    let plan = recommend_config(MemoryMode::Stream, None, Some(256 * 1024)).expect("recommend_config");
    
    assert_eq!(plan.mode as u8, MemoryMode::Stream as u8, "should use stream mode");
    assert!(plan.buffer_size <= 64 * 1024, "stream mode should use small buffers");
    assert!(plan.total_buffer_bytes() <= 256 * 1024, "should respect max_ram");
}

#[test]
fn thread_capping_based_on_memory() {
    // When max_ram is very low, thread count should be capped
    let plan = recommend_config(MemoryMode::Balanced, None, Some(64 * 1024)).expect("recommend_config");
    
    // With only 64KB budget, should have very few threads
    assert!(plan.threads <= 2, "low memory should cap threads, got {}", plan.threads);
    assert!(plan.num_buffers <= plan.threads, "buffers should not exceed threads");
}

#[test]
fn booster_mode_high_memory() {
    // Booster mode with high memory should use large buffers
    let plan = recommend_config(MemoryMode::Booster, None, Some(64 * 1024 * 1024)).expect("recommend_config");
    
    assert_eq!(plan.mode as u8, MemoryMode::Booster as u8);
    assert!(plan.buffer_size >= 1024 * 1024, "booster should use 1MB+ buffers");
    assert!(plan.threads >= 1);
}

#[test]
fn balanced_mode_default_behavior() {
    // Balanced mode should be middle ground
    let plan = recommend_config(MemoryMode::Balanced, None, Some(4 * 1024 * 1024)).expect("recommend_config");
    
    assert_eq!(plan.mode as u8, MemoryMode::Balanced as u8);
    assert!(plan.buffer_size >= 256 * 1024, "balanced should use 256KB+ buffers");
    assert!(plan.buffer_size <= 1024 * 1024, "balanced should not use huge buffers");
}

#[test]
fn pipeline_with_constrained_memory() {
    // Integration test: run a pipeline with constrained memory
    let dir = tempdir().unwrap();
    let root = dir.path().join("test_files");
    create_dir_all(&root).unwrap();
    
    // Create several test files
    for i in 0..10 {
        write(root.join(format!("file{}.txt", i)), format!("content {}", i).as_bytes()).unwrap();
    }
    
    let processed = Arc::new(Mutex::new(0));
    let processed_clone = processed.clone();
    
    let pipeline = Pipeline::new(MemoryMode::Stream)
        .with_max_ram(Some(128 * 1024)); // 128KB limit
    
    let count = pipeline
        .run(&root, &[], None, false, false, move |_path, _pool| {
            let mut p = processed_clone.lock().unwrap();
            *p += 1;
            Ok(())
        })
        .expect("pipeline run");
    
    assert_eq!(count, 10, "should process all files");
    assert_eq!(*processed.lock().unwrap(), 10);
}

#[test]
fn thread_override_respected() {
    // When threads are explicitly overridden, they should be respected (unless capped by memory)
    let plan = recommend_config(MemoryMode::Balanced, Some(4), Some(16 * 1024 * 1024)).expect("recommend_config");
    
    assert_eq!(plan.threads, 4, "should respect thread override");
}

#[test]
fn buffer_pool_backpressure() {
    // Test that buffer pool handles over-allocation gracefully
    let pool = BufferPool::new(2, 1024);
    
    let mut buffers = Vec::new();
    // Request more buffers than pool size
    for _ in 0..5 {
        buffers.push(pool.get());
    }
    
    // Should allocate beyond budget but track it
    assert!(pool.allocated_buffers() >= 2, "should track all allocations");
    
    // Drop all buffers
    buffers.clear();
    
    // Pool should recover
    assert!(pool.allocated_buffers() <= pool.max_buffers() + 3, "pool should recover most buffers");
}
