# dropbear-futurequeue

A helper queue for polling futures in single threaded systems such as in winit.

# Example

```rust
use tokio::runtime::Runtime;
use tokio::time::sleep;
use dropbear_future_queue::FutureQueue;

fn main() {
    // requires a tokio thread
    let rt = Runtime::new().unwrap();
    let _guard = rt.enter();

    // create new queue
    let queue = FutureQueue::new();

    // create a new handle to keep for reference
    let handle = queue.push(async move {
        sleep(1000).await;
        67 + 41
    });

    // assume this is the event loop
    loop {
        // executes all the futures in the database
        queue.poll();

        println!("Current status of compututation: {:?}", queue.get_status(&handle));

        // check if it is ready to be taken
        if let Some(result) = queue.exchange_as::<i32>(&handle) {
            println!("67 + 41 = {}", result);
            break;
        }

        // cleans up any ids not needed anymore.
        queue.cleanup()
    }
}
```