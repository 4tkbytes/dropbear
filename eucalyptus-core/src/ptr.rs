#![allow(dead_code)] // needed because some fields arent accessed
use std::marker::PhantomData;

/// A clonable Send/Sync pointer. Typically unsafe, but fuck it we ball.
/// Anything to not deal with Mutex and RwLock amirite???
#[derive(Clone)]
pub struct SafePointer<T> {
    ptr: *const T,
    _marker: PhantomData<T>
}

unsafe impl<T> Send for SafePointer<T> where T: Send {}

unsafe impl<T> Sync for SafePointer<T> where T: Sync {}

impl<T> SafePointer<T> {
    /// Creates a new safe pointer from an unsafe pointer
    pub fn new(ptr: *const T) -> Self {
        SafePointer {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Accesses the [`SafePointer`] as an unsafe pointer
    pub unsafe fn get(&self) -> *const T {
        self.ptr
    }
}
