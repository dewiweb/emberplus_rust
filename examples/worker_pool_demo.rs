//! Worker Pool demonstration for parallel subscriptions
//!
//! Run with: cargo run --example worker_pool_demo

use ember_plus::{WorkerPool, WorkerPoolConfig, WorkerTask, Result};
use std::time::Instant;

const LAWO_ADDRESS: &str = "192.168.100.199:9000";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("worker_pool_demo=debug,ember_plus=info")
        .init();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     Worker Pool Demo - Parallel Subscriptions              ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Configure worker pool with 4 workers
    let config = WorkerPoolConfig {
        num_workers: 4,
        queue_size: 500,
        connect_timeout_ms: 5000,
    };

    println!("1. Creating worker pool with {} workers...", config.num_workers);
    let start = Instant::now();
    
    let pool = WorkerPool::new(LAWO_ADDRESS, config).await?;
    println!("   ✓ Pool created in {:?}", start.elapsed());

    // Simulate subscribing to 100 paths
    let paths: Vec<String> = (0..100)
        .map(|i| format!("28548.28549.{}", 250000 + i))
        .collect();
    let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();

    println!("\n2. Submitting {} subscription tasks...", paths.len());
    let start = Instant::now();
    
    let task_ids = pool.subscribe_batch(&path_refs).await?;
    println!("   ✓ {} tasks submitted in {:?}", task_ids.len(), start.elapsed());

    println!("\n3. Waiting for results...");
    let start = Instant::now();
    
    let results = pool.wait_for_results(task_ids.len()).await;
    
    let success_count = results.iter().filter(|r| matches!(r, ember_plus::TaskResult::Success { .. })).count();
    let error_count = results.len() - success_count;
    
    println!("   ✓ {} results in {:?}", results.len(), start.elapsed());
    println!("   ✓ Success: {}, Errors: {}", success_count, error_count);

    // Test expand batch
    println!("\n4. Testing expand batch...");
    let expand_paths = ["28548", "28548.28549", "28548.259634"];
    let start = Instant::now();
    
    let expand_ids = pool.expand_batch(&expand_paths).await?;
    let expand_results = pool.wait_for_results(expand_ids.len()).await;
    
    println!("   ✓ {} expands completed in {:?}", expand_results.len(), start.elapsed());

    // Shutdown
    println!("\n5. Shutting down pool...");
    pool.shutdown().await;
    
    println!("\n════════════════════════════════════════════════════════════");
    println!("  Worker pool demo complete!");
    println!("════════════════════════════════════════════════════════════");

    Ok(())
}
