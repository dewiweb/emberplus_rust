//! Worker pool for parallel subscription and metering tasks.
//!
//! This module provides a pool of async workers to handle:
//! - Batch subscriptions in parallel
//! - High-frequency meter updates
//! - Value change callbacks without blocking the main connection

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock, Semaphore};
use tracing::{debug, info, warn};

use crate::{EmberClient, EmberValue, Result};

/// Task types for the worker pool
#[derive(Debug, Clone)]
pub enum WorkerTask {
    /// Subscribe to a path
    Subscribe { path: String },
    /// Unsubscribe from a path
    Unsubscribe { path: String },
    /// Set a value at path
    SetValue { path: String, value: EmberValue },
    /// Expand a path and get children
    Expand { path: String },
    /// Get element by path
    GetElement { path: String },
}

/// Result from a worker task
#[derive(Debug, Clone)]
pub enum TaskResult {
    Success { task_id: u64, path: String },
    Error { task_id: u64, path: String, error: String },
}

/// Configuration for the worker pool
#[derive(Debug, Clone)]
pub struct WorkerPoolConfig {
    /// Number of worker connections to maintain
    pub num_workers: usize,
    /// Maximum tasks in queue before backpressure
    pub queue_size: usize,
    /// Connection timeout in milliseconds
    pub connect_timeout_ms: u64,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            num_workers: 4,
            queue_size: 1000,
            connect_timeout_ms: 5000,
        }
    }
}

/// Worker pool for parallel Ember+ operations
pub struct WorkerPool {
    config: WorkerPoolConfig,
    address: String,
    task_tx: mpsc::Sender<(u64, WorkerTask)>,
    result_rx: Arc<RwLock<mpsc::Receiver<TaskResult>>>,
    result_tx: mpsc::Sender<TaskResult>,
    task_counter: Arc<RwLock<u64>>,
    semaphore: Arc<Semaphore>,
    shutdown_tx: broadcast::Sender<()>,
}

impl WorkerPool {
    /// Create a new worker pool connected to the given address
    pub async fn new(address: &str, config: WorkerPoolConfig) -> Result<Self> {
        let (task_tx, task_rx) = mpsc::channel(config.queue_size);
        let (result_tx, result_rx) = mpsc::channel(config.queue_size);
        let (shutdown_tx, _shutdown_rx) = broadcast::channel(1);
        
        let task_rx = Arc::new(RwLock::new(task_rx));
        let semaphore = Arc::new(Semaphore::new(config.num_workers));
        
        // Spawn worker tasks first (before moving shutdown_tx)
        for worker_id in 0..config.num_workers {
            let addr = address.to_string();
            let rx = task_rx.clone();
            let tx = result_tx.clone();
            let sem = semaphore.clone();
            let mut shutdown = shutdown_tx.subscribe();
            
            tokio::spawn(async move {
                info!("Worker {} starting", worker_id);
                
                // Each worker maintains its own connection
                let client = match EmberClient::connect(&addr).await {
                    Ok(c) => c,
                    Err(e) => {
                        warn!("Worker {} failed to connect: {}", worker_id, e);
                        return;
                    }
                };
                
                let _ = client.get_directory().await;
                
                loop {
                    tokio::select! {
                        _ = shutdown.recv() => {
                            debug!("Worker {} shutting down", worker_id);
                            let _ = client.disconnect().await;
                            break;
                        }
                        task = async {
                            let mut rx_guard = rx.write().await;
                            rx_guard.recv().await
                        } => {
                            if let Some((task_id, task)) = task {
                                let _permit = sem.acquire().await.unwrap();
                                let result = Self::process_task(&client, task_id, task).await;
                                let _ = tx.send(result).await;
                            }
                        }
                    }
                }
            });
        }
        
        let pool = Self {
            config: config.clone(),
            address: address.to_string(),
            task_tx,
            result_rx: Arc::new(RwLock::new(result_rx)),
            result_tx: result_tx.clone(),
            task_counter: Arc::new(RwLock::new(0)),
            semaphore,
            shutdown_tx,
        };
        
        info!("Worker pool started with {} workers", config.num_workers);
        Ok(pool)
    }
    
    /// Process a single task
    async fn process_task(client: &EmberClient, task_id: u64, task: WorkerTask) -> TaskResult {
        match task {
            WorkerTask::Subscribe { path } => {
                match client.subscribe(&path).await {
                    Ok(_) => TaskResult::Success { task_id, path },
                    Err(e) => TaskResult::Error { task_id, path, error: e.to_string() },
                }
            }
            WorkerTask::Unsubscribe { path } => {
                match client.unsubscribe(&path).await {
                    Ok(_) => TaskResult::Success { task_id, path },
                    Err(e) => TaskResult::Error { task_id, path, error: e.to_string() },
                }
            }
            WorkerTask::SetValue { path, value } => {
                match client.set_value(&path, value).await {
                    Ok(_) => TaskResult::Success { task_id, path },
                    Err(e) => TaskResult::Error { task_id, path, error: e.to_string() },
                }
            }
            WorkerTask::Expand { path } => {
                match client.expand(&path).await {
                    Ok(_) => TaskResult::Success { task_id, path },
                    Err(e) => TaskResult::Error { task_id, path, error: e.to_string() },
                }
            }
            WorkerTask::GetElement { path } => {
                match client.get_element_by_path(&path).await {
                    Ok(_) => TaskResult::Success { task_id, path },
                    Err(e) => TaskResult::Error { task_id, path, error: e.to_string() },
                }
            }
        }
    }
    
    /// Submit a task to the pool
    pub async fn submit(&self, task: WorkerTask) -> Result<u64> {
        let mut counter = self.task_counter.write().await;
        *counter += 1;
        let task_id = *counter;
        
        self.task_tx.send((task_id, task)).await
            .map_err(|_| crate::Error::Connection("Worker pool queue full".to_string()))?;
        
        Ok(task_id)
    }
    
    /// Submit multiple subscribe tasks in batch
    pub async fn subscribe_batch(&self, paths: &[&str]) -> Result<Vec<u64>> {
        let mut task_ids = Vec::with_capacity(paths.len());
        
        for path in paths {
            let task_id = self.submit(WorkerTask::Subscribe { 
                path: path.to_string() 
            }).await?;
            task_ids.push(task_id);
        }
        
        Ok(task_ids)
    }
    
    /// Submit multiple expand tasks in batch
    pub async fn expand_batch(&self, paths: &[&str]) -> Result<Vec<u64>> {
        let mut task_ids = Vec::with_capacity(paths.len());
        
        for path in paths {
            let task_id = self.submit(WorkerTask::Expand { 
                path: path.to_string() 
            }).await?;
            task_ids.push(task_id);
        }
        
        Ok(task_ids)
    }
    
    /// Get pending results (non-blocking)
    pub async fn try_recv_result(&self) -> Option<TaskResult> {
        let mut rx = self.result_rx.write().await;
        rx.try_recv().ok()
    }
    
    /// Wait for a specific number of results
    pub async fn wait_for_results(&self, count: usize) -> Vec<TaskResult> {
        let mut results = Vec::with_capacity(count);
        let mut rx = self.result_rx.write().await;
        
        for _ in 0..count {
            if let Some(result) = rx.recv().await {
                results.push(result);
            }
        }
        
        results
    }
    
    /// Get the number of workers
    pub fn num_workers(&self) -> usize {
        self.config.num_workers
    }
    
    /// Shutdown the worker pool
    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
        info!("Worker pool shutdown initiated");
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        // Shutdown signal will be sent when shutdown_tx is dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_worker_pool_config() {
        let config = WorkerPoolConfig::default();
        assert_eq!(config.num_workers, 4);
        assert_eq!(config.queue_size, 1000);
    }
}
