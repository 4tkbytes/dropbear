# dropbear-futurequeue

A helper queue for polling futures in single threaded systems such as in winit.

## Example

```rust
// create new queue
let queue = FutureQueue::new();

// create a new handle to keep for reference
let handle = queue.push(async move {
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    67 + 41
});

// check initial status
assert!(matches!(queue.get_status(&handle), Some(FutureStatus::NotPolled)));

// execute the futures
queue.poll();

// wait for the task to do its job (this can be simulated with an update loop)
tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

// check the result
if let Some(result) = queue.exchange_as::<i32>(&handle) {
    println!("67 + 41 = {}", result);
    assert_eq!(result, 108);
}
```
