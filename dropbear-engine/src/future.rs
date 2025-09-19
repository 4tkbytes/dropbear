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