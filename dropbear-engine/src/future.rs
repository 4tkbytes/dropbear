//! Polling and async features in the dropbear engine where scenes are single threaded.

use std::any::Any;
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use ahash::{HashMap, HashMapExt};
use parking_lot::Mutex;
use tokio::sync::oneshot;
use std::future::Future;

/// A type used for a future.
///
/// It must include a [`Send`] trait to be usable for the [`FutureQueue`]
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + Sync>>;
pub type AnyResult = Arc<dyn Any + Send + Sync>;
pub type ResultSender = oneshot::Sender<AnyResult>;
pub type ResultReceiver = oneshot::Receiver<AnyResult>;
pub type FutureStorage = Arc<Mutex<VecDeque<(u64, BoxFuture<()>)>>>;

/// A status showing the future, used by the [`ResultReceiver`] and [`ResultSender`]
#[derive(Clone)]
pub enum FutureStatus {
    NotPolled,
    CurrentlyPolling,
    Completed(AnyResult),
}

/// A handle to the future task
#[derive(Default, Clone)]
pub struct FutureHandle {
    pub id: u64,
}

/// Internal storage per handle — separate from FutureHandle
struct HandleEntry {
    receiver: ResultReceiver,
    status: FutureStatus,
}

/// A queue used for futures.
pub struct FutureQueue {
    /// The queue for the futures.
    queued: FutureStorage,
    /// A place to store all handle data
    handle_registry: Arc<Mutex<HashMap<u64, HandleEntry>>>,
    /// Next id to be processed
    next_id: Arc<Mutex<u64>>,
}

impl FutureQueue {
    /// Creates a new [`Arc<FutureQueue>`].
    pub fn new() -> Self {
        Self {
            queued: Arc::new(Mutex::new(VecDeque::new())),
            handle_registry: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Pushes a future to the FutureQueue. It will sit and wait
    /// to be processed until [`FutureQueue::poll`] is called.
    ///
    /// This creates a new hash by using the [`ahash`] crate. The type is not required
    /// to implement [`std::hash::Hash`].
    pub fn push<F, T>(&self, future: F) -> FutureHandle
    where
        F: Future<Output = T> + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;

        let (sender, receiver) = oneshot::channel();

        let entry = HandleEntry {
            receiver,
            status: FutureStatus::NotPolled,
        };

        self.handle_registry.lock().insert(id, entry);

        let registry_clone = self.handle_registry.clone();
        let wrapped_future = Box::pin(async move {
            let result = future.await;
            let boxed_result = Arc::new(result) as AnyResult;

            let _ = sender.send(boxed_result.clone());

            let mut registry = registry_clone.lock();
            if let Some(entry) = registry.get_mut(&id) {
                entry.status = FutureStatus::Completed(boxed_result);
            }
        });

        self.queued.lock().push_back((id, wrapped_future));

        FutureHandle { id } // 👈 Simple handle
    }

    /// Polls all the futures in the future queue and resolves the handles.
    ///
    /// This function spawns a new async thread for each item inside the thread and
    /// sends updates to the Handle's receiver.
    pub fn poll(&self) { // 👈 Removed unused generics <T, F>
        let mut queue = self.queued.lock();
        let mut futures_to_spawn = Vec::new();

        while let Some((id, future)) = queue.pop_front() {
            // Update status to CurrentlyPolling
            if let Some(entry) = self.handle_registry.lock().get_mut(&id) {
                entry.status = FutureStatus::CurrentlyPolling;
            }

            futures_to_spawn.push(future);
        }

        for future in futures_to_spawn {
            tokio::spawn(future);
        }
    }

    /// Exchanges the future for the result.
    ///
    /// When the handle is not successful, it will return nothing. When the handle is successful,
    /// it will return the result and drop the handle, removing the usage of it.
    pub fn exchange(&self, handle: &FutureHandle) -> Option<AnyResult> {
        let mut registry = self.handle_registry.lock();
        if let Some(entry) = registry.get_mut(&handle.id) {
            match &entry.status {
                FutureStatus::Completed(result) => {
                    return Some(result.clone()); // Clone the Arc
                }
                _ => {
                    return match entry.receiver.try_recv() {
                        Ok(result) => {
                            entry.status = FutureStatus::Completed(result.clone());
                            Some(result)
                        }
                        Err(_) => None,
                    }
                }
            }
        }
        None
    }

    /// Exchanges the handle and safely downcasts it into a specific type.
    pub fn exchange_as<T: Any + Send + Sync + 'static>(&self, handle: &FutureHandle) -> Option<Arc<T>> {
        self.exchange(handle)?
            .downcast()
            .ok()
    }

    /// Retrieve a handle by u64 ID
    pub fn get_handle(&self, id: u64) -> Option<FutureHandle> {
        let registry = self.handle_registry.lock();
        if registry.contains_key(&id) {
            Some(FutureHandle { id })
        } else {
            None
        }
    }

    /// Get status of a handle
    pub fn get_status(&self, id: u64) -> Option<FutureStatus> {
        let registry = self.handle_registry.lock();
        registry.get(&id).map(|entry| entry.status.clone())
    }

    /// Cleans up any completed handles and removes them from the registry.
    ///
    /// You can do this manually, however this is typically done at the end of the frame.
    pub fn cleanup(&self) {
        let mut registry = self.handle_registry.lock();
        let completed_ids: Vec<u64> = registry
            .iter()
            .filter_map(|(&id, entry)| {
                matches!(entry.status, FutureStatus::Completed(_)).then_some(id)
            })
            .collect();

        for id in completed_ids {
            registry.remove(&id);
        }
    }
}

impl Default for FutureQueue {
    fn default() -> Self {
        Self::new()
    }
}

mod tests {
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::watch;
    use tokio::time::sleep;
    use crate::future::{FutureHandle, FutureQueue, FutureStatus};

    #[tokio::test]
    async fn test_basic_future_completion() {
        let queue = Arc::new(FutureQueue::new());

        // Push a simple future
        let handle = queue.push(async {
            sleep(Duration::from_millis(10)).await;
            42i32
        });

        // Poll to start it
        queue.poll();

        // Wait for completion
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Exchange result
        let result = queue.exchange_as::<i32>(&handle).unwrap();
        assert_eq!(*result, 42);
    }

    #[tokio::test]
    async fn test_multiple_futures() {
        let queue = Arc::new(FutureQueue::new());

        let handles: Vec<FutureHandle> = (0..5)
            .map(|i| {
                queue.push(async move {
                    sleep(Duration::from_millis(10 + i * 5)).await;
                    i * 10
                })
            })
            .collect();

        queue.poll();

        // Wait for all to complete
        tokio::time::sleep(Duration::from_millis(10000)).await;

        // Check all results
        for (i, handle) in handles.iter().enumerate() {
            let result = queue.exchange_as::<i32>(handle).unwrap();
            assert_eq!(*result, (i * 10) as i32);
        }
    }

    #[tokio::test]
    async fn test_status_tracking() {
        let queue = Arc::new(FutureQueue::new());

        let handle = queue.push(async {
            sleep(Duration::from_millis(50)).await;
            "done".to_string()
        });

        // Before polling
        assert!(matches!(queue.get_status(handle.id), Some(FutureStatus::NotPolled)));

        queue.poll();

        // After polling, before completion
        assert!(matches!(queue.get_status(handle.id), Some(FutureStatus::CurrentlyPolling)));

        // Wait for completion
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be completed
        assert!(matches!(queue.get_status(handle.id), Some(FutureStatus::Completed(_))));

        // Exchange should still work
        let result = queue.exchange_as::<String>(&handle).unwrap();
        assert_eq!(*result, "done");
    }

    #[tokio::test]
    async fn test_exchange_before_completion_returns_none() {
        let queue = Arc::new(FutureQueue::new());

        let handle = queue.push(async {
            sleep(Duration::from_millis(100)).await;
            true
        });

        queue.poll();

        // Try to exchange immediately — should return None
        assert!(queue.exchange_as::<bool>(&handle).is_none());

        // Wait and try again
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(queue.exchange_as::<bool>(&handle).is_some());
    }

    #[tokio::test]
    async fn test_cleanup_removes_completed_handles() {
        let queue = Arc::new(FutureQueue::new());

        let handle = queue.push(async {
            sleep(Duration::from_millis(10)).await;
            123i32
        });

        queue.poll();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Should be in registry before cleanup
        assert!(queue.get_handle(handle.id).is_some());

        queue.cleanup();

        // Should be removed after cleanup
        assert!(queue.get_handle(handle.id).is_none());
    }

    #[tokio::test]
    async fn test_progress_channel_integration() {
        let queue = Arc::new(FutureQueue::new());

        let (progress_tx, mut progress_rx) = watch::channel(0.0f32);

        let handle = queue.push(async move {
            progress_tx.send(0.25).unwrap();
            sleep(Duration::from_millis(20)).await;

            progress_tx.send(0.75).unwrap();
            sleep(Duration::from_millis(20)).await;

            progress_tx.send(1.0).unwrap();
            "final_result".to_string()
        });

        queue.poll();

        // Check progress updates
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(*progress_rx.borrow_and_update(), 0.25);

        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(*progress_rx.borrow_and_update(), 0.75);

        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(*progress_rx.borrow_and_update(), 1.0);

        // Check final result
        let result = queue.exchange_as::<String>(&handle).unwrap();
        assert_eq!(*result, "final_result");
    }

    #[tokio::test]
    async fn test_error_handling() {
        let queue = Arc::new(FutureQueue::new());

        let handle = queue.push(async {
            sleep(Duration::from_millis(10)).await;
            Result::<i32, &'static str>::Err("something went wrong")
        });

        queue.poll();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let result = queue.exchange_as::<Result<i32, &'static str>>(&handle).unwrap();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "something went wrong");
    }

    #[tokio::test]
    async fn test_get_handle_returns_correct_handle() {
        let queue = Arc::new(FutureQueue::new());

        let handle = queue.push(async {
            sleep(Duration::from_millis(10)).await;
            999i32
        });

        let retrieved_handle = queue.get_handle(handle.id).unwrap();
        assert_eq!(retrieved_handle.id, handle.id);

        // Invalid ID should return None
        assert!(queue.get_handle(999999).is_none());
    }

    #[tokio::test]
    async fn test_exchange_by_id() {
        let queue = Arc::new(FutureQueue::new());

        let handle = queue.push(async {
            sleep(Duration::from_millis(10)).await;
            "test_string".to_string()
        });

        queue.poll();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let result = queue.exchange_as::<String>(&handle).unwrap();
        assert_eq!(*result, "test_string");
    }

    #[tokio::test]
    async fn test_concurrent_futures() {
        let queue = Arc::new(FutureQueue::new());

        // Push 10 concurrent futures
        let handles: Vec<FutureHandle> = (0..10)
            .map(|i| {
                queue.push(async move {
                    // Simulate variable work
                    let delay = Duration::from_millis(10 + (i * 5) as u64);
                    sleep(delay).await;
                    i
                })
            })
            .collect();

        queue.poll();

        // Wait for all to complete
        tokio::time::sleep(Duration::from_millis(10000)).await;

        // Verify all results
        for (i, handle) in handles.iter().enumerate() {
            let result = queue.exchange_as::<usize>(handle).unwrap();
            assert_eq!(*result, i);
        }
    }

    #[tokio::test]
    async fn test_downcast_failure_returns_none() {
        let queue = Arc::new(FutureQueue::new());

        let handle = queue.push(async {
            sleep(Duration::from_millis(10)).await;
            42i32
        });

        queue.poll();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Try to downcast to wrong type
        let result = queue.exchange_as::<String>(&handle);
        assert!(result.is_none());

        // But correct type works
        let result = queue.exchange_as::<i32>(&handle).unwrap();
        assert_eq!(*result, 42);
    }
}