# deny_query_one_mut_lint

A custom lint for all codebases in the dropbear-engine that check if the usage of [`hecs::World::query_one_mut`] 
is used. 

### Cause

The editor is required to have the world be threadsafe. This is achieved with the help of `Arc<RwLock<T>>`. 
This is a great way to achieve threadsafety, however writing to the world often creates a 
[deadlock](https://en.wikipedia.org/wiki/Deadlock_(computer_science)). A deadlock in Rust is undetectable during
compile-time and when ran during runtime, it can cause the program to freeze and not respond. 

### Fix

To solve this conundrum, I have implemented a couple of fixes:
1. Using a [`std::sync::Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html) allows for locking, which allows 
for mutability but blocks that thread. Furthermore, it cannot/shouldn't be used in threads where a mutex uses `Send`. 
To fix this, I switched to [`parking_lot::RwLock`](https://docs.rs/parking_lot/latest/parking_lot/type.RwLock.html), 
which doesn't use `Send` (making it even more threadsafe).
Conveniently, it also includes deadlock detection (as an enabled feature), which can point out if a deadlock is in progress or if an expensive
operation is being run. 
2. I have switched from `hecs::World::query_one_mut` to `hecs::World::query_one` to use only the `RwLock::read(&self)`,
which doesn't require a mutable reference from `self.world`. 

Even with such fixes, deadlocks are inevitable in the codebase. This crate aims to fix this by creating a custom rustc
lint. 

Note: https://blog.guillaume-gomez.fr/articles/2024-01-18+Writing+your+own+Rust+linter