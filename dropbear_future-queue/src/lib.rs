//! Enabling multithreading for functions and apps that are purely single threaded.
//!
//! This was originally a module in my [dropbear](https://github.com/4tkbytes/dropbear) game engine,
//! however I thought there were barely any libraries that had future queuing. It takes inspiration
//! from Unity and how they handle their events.
//!
//! # Example
//! ```rust
//! use dropbear_future_queue::{FutureQueue, FutureStatus};
//!
//! # tokio_test::block_on(async {
//! // create new queue
//! let queue = FutureQueue::new();
//!
//! // create a new handle to keep for reference
//! let handle = queue.push(async move {
//!     tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
//!     67 + 41
//! });
//!
//! // Check initial status
//! assert!(matches!(queue.get_status(&handle), Some(FutureStatus::NotPolled)));
//!
//! // execute the futures
//! queue.poll();
//!
//! // Wait a bit for completion and check result
//! tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
//!
//! if let Some(result) = queue.exchange_as::<i32>(&handle) {
//!     println!("67 + 41 = {}", result);
//!     assert_eq!(result, 108);
//! }
//! # });
//! ```

use ahash::{HashMap, HashMapExt};
use parking_lot::Mutex;
use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::oneshot;

/// A type used for a future.
///
/// It must include a [`Send`] trait to be usable for the [`FutureQueue`]
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
/// A clonable generic result. It can/preferred to be downcasted to your specific type.
pub type AnyResult = Arc<dyn Any + Send + Sync>;
/// Internal function: A result receiver
type ResultReceiver = oneshot::Receiver<AnyResult>;
/// A type for storing the queue. It uses a [`VecDeque`] to store [`FutureHandle`]'s and [`BoxFuture`]
pub type FutureStorage = Arc<Mutex<VecDeque<(FutureHandle, BoxFuture<()>)>>>;
/// A type recommended to be used by [`FutureQueue`] to allow being thrown around in your app
pub type Throwable<T> = Rc<RefCell<T>>;

/// A status showing the future, used by the [`ResultReceiver`] and [`ResultSender`]
#[derive(Clone, Debug)]
pub enum FutureStatus {
    NotPolled,
    CurrentlyPolling,
    Completed,
}

/// A handle to the future task
#[derive(Default, Copy, Clone, Eq, Hash, PartialEq, Debug)]
pub struct FutureHandle {
    pub id: u64,
}

/// Internal storage per handle — separate from FutureHandle
struct HandleEntry {
    receiver: Option<ResultReceiver>,
    status: FutureStatus,
    cached_result: Option<AnyResult>,
}

/// A queue for polling futures. It is stored in here until [`FutureQueue::poll`] is run.
pub struct FutureQueue {
    /// The queue for the futures.
    queued: FutureStorage,
    /// A place to store all handle data
    handle_registry: Arc<Mutex<HashMap<FutureHandle, HandleEntry>>>,
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
    pub fn push<F, T>(&self, future: F) -> FutureHandle
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + Sync + 'static,
    {
        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;

        let id = FutureHandle { id };

        let (sender, receiver) = oneshot::channel::<AnyResult>();

        let entry = HandleEntry {
            receiver: Some(receiver),
            status: FutureStatus::NotPolled,
            cached_result: None,
        };

        self.handle_registry.lock().insert(id, entry);

        let wrapped_future: Pin<Box<dyn Future<Output = ()> + Send>> = Box::pin(async move {
            log("Starting future execution");
            let result = future.await;
            let boxed_result: AnyResult = Arc::new(result);
            log("Future completed, sending result");

            let _ = sender.send(boxed_result);

            // Don't update status here - let the exchange method handle it
            log("Result sent via channel");
        });

        self.queued.lock().push_back((id, wrapped_future));

        id
    }

    /// Polls all the futures in the future queue and resolves the handles.
    ///
    /// This function spawns a new async thread for each item inside the thread and
    /// sends updates to the Handle's receiver.
    pub fn poll(&self) {
        let mut queue = self.queued.lock();
        log("Locked queue for polling");

        if queue.is_empty() {
            log("Queue is empty, nothing to poll");
            return;
        }

        let mut futures_to_spawn = Vec::new();

        while let Some((id, future)) = queue.pop_front() {
            log(format!("Processing future with id: {:?}", id));

            {
                let mut registry = self.handle_registry.lock();
                if let Some(entry) = registry.get_mut(&id) {
                    entry.status = FutureStatus::CurrentlyPolling;
                    log("Updated status to CurrentlyPolling");
                }
            }

            futures_to_spawn.push(future);
        }

        drop(queue);

        for future in futures_to_spawn {
            log("Spawning future with tokio");
            tokio::spawn(future);
        }
    }

    /// Exchanges the future for the result.
    ///
    /// When the handle is not successful, it will return nothing. When the handle is successful,
    /// it will return the result. The result is cached and can be retrieved multiple times.
    pub fn exchange(&self, handle: &FutureHandle) -> Option<AnyResult> {
        let mut registry = self.handle_registry.lock();
        if let Some(entry) = registry.get_mut(handle) {
            match &entry.status {
                FutureStatus::Completed => {
                    log("FutureStatus::Completed - returning cached result");
                    entry.cached_result.clone()
                }
                _ => {
                    log("Future not completed yet, checking receiver");
                    if let Some(receiver) = entry.receiver.as_mut() {
                        match receiver.try_recv() {
                            Ok(result) => {
                                log("Received result from channel");
                                entry.status = FutureStatus::Completed;
                                entry.cached_result = Some(result.clone());
                                entry.receiver = None; // Remove receiver as it's no longer needed
                                Some(result)
                            }
                            Err(oneshot::error::TryRecvError::Empty) => {
                                log("Channel is empty - future still running");
                                None
                            }
                            Err(oneshot::error::TryRecvError::Closed) => {
                                log("Channel is closed - future may have panicked");
                                None
                            }
                        }
                    } else {
                        log("No receiver available");
                        None
                    }
                }
            }
        } else {
            log("Handle not found in registry");
            None
        }
    }

    /// Exchanges the future for the result, taking ownership and consuming the cached result.
    ///
    /// When the handle is not successful, it will return nothing. When the handle is successful,
    /// it will return the result and remove it from the cache, allowing Arc::try_unwrap to succeed.
    /// This method can only be called once per completed future.
    pub fn exchange_owned(&self, handle: &FutureHandle) -> Option<AnyResult> {
        let mut registry = self.handle_registry.lock();
        if let Some(entry) = registry.get_mut(handle) {
            match &entry.status {
                FutureStatus::Completed => {
                    log("FutureStatus::Completed - taking ownership of cached result");
                    entry.cached_result.take()
                }
                _ => {
                    log("Future not completed yet, checking receiver");
                    if let Some(receiver) = entry.receiver.as_mut() {
                        match receiver.try_recv() {
                            Ok(result) => {
                                log("Received result from channel");
                                entry.status = FutureStatus::Completed;
                                entry.receiver = None; // Remove receiver as it's no longer needed
                                Some(result)
                            }
                            Err(oneshot::error::TryRecvError::Empty) => {
                                log("Channel is empty - future still running");
                                None
                            }
                            Err(oneshot::error::TryRecvError::Closed) => {
                                log("Channel is closed - future may have panicked");
                                None
                            }
                        }
                    } else {
                        log("No receiver available");
                        None
                    }
                }
            }
        } else {
            log("Handle not found in registry");
            None
        }
    }

    /// Exchanges the handle and safely downcasts it into a specific type.
    pub fn exchange_as<T: Any + Send + Sync + 'static>(&self, handle: &FutureHandle) -> Option<T> {
        self.exchange(handle)?
            .downcast::<T>()
            .ok()
            .and_then(|arc| Arc::try_unwrap(arc).ok())
    }

    /// Exchanges the handle taking ownership and safely downcasts it into a specific type.
    /// This method consumes the cached result, allowing Arc::try_unwrap to succeed.
    pub fn exchange_owned_as<T: Any + Send + Sync + 'static>(
        &self,
        handle: &FutureHandle,
    ) -> Option<T> {
        self.exchange_owned(handle)?
            .downcast::<T>()
            .ok()
            .and_then(|arc| Arc::try_unwrap(arc).ok())
    }

    /// Get status of a handle
    pub fn get_status(&self, handle: &FutureHandle) -> Option<FutureStatus> {
        let registry = self.handle_registry.lock();
        registry.get(handle).map(|entry| entry.status.clone())
    }

    /// Cleans up any completed handles and removes them from the registry.
    ///
    /// You can do this manually, however this is typically done at the end of the frame.
    pub fn cleanup(&self) {
        let mut registry = self.handle_registry.lock();
        let completed_ids: Vec<FutureHandle> = registry
            .iter()
            .filter_map(|(&id, entry)| {
                matches!(entry.status, FutureStatus::Completed).then_some(id)
            })
            .collect();

        for id in completed_ids {
            registry.remove(&id);
        }
    }
}

/// Internal function for logging to a file for tests (when stdout is not available).
///
/// Only logs if the [`LOG_TO_FILE`] constant is set to true.
#[cfg(test)]
fn log(msg: impl ToString) {
    use std::io::Write;

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("test.log")
        .unwrap();
    file.write_all(format!("{}\n", msg.to_string()).as_bytes())
        .unwrap();
}

#[cfg(not(test))]
fn log(_msg: impl ToString) {}

impl Default for FutureQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn test_future_queue() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let queue = FutureQueue::new();
            log("Created new queue");

            let handle = queue.push(async move {
                log("Inside the pushed future - starting work");
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                log("Inside the pushed future - work completed");
                67 + 41
            });
            log("Created new handle");

            queue.poll();
            log("Initial poll completed");

            let mut attempts = 0;
            let max_attempts = 100;
            let start_time = std::time::Instant::now();

            loop {
                attempts += 1;
                log(format!("Attempt {}: Checking for result", attempts));
                log(format!(
                    "Time since start: {} ms",
                    start_time.elapsed().as_millis()
                ));

                if let Some(result) = queue.exchange(&handle) {
                    let result = result.downcast::<i32>().unwrap();
                    log(format!("Success! 67 + 41 = {}", result));
                    assert_eq!(*result, 108);
                    break;
                }

                if attempts >= max_attempts {
                    log("Max attempts reached - test failed");
                    panic!("Future never completed");
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }

            log("Test completed successfully");
        });
}

// #[tokio::test]
#[test]
fn test_exchange_owned_as() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let queue = FutureQueue::new();

            let handle = queue.push(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                67 + 41
            });

            queue.poll();

            let mut attempts = 0;
            let max_attempts = 100;

            loop {
                attempts += 1;

                if let Some(result) = queue.exchange_owned_as::<i32>(&handle) {
                    assert_eq!(result, 108);
                    break;
                }

                if attempts >= max_attempts {
                    panic!("Future never completed");
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }

            assert!(queue.exchange_owned_as::<i32>(&handle).is_none());
        });
}
