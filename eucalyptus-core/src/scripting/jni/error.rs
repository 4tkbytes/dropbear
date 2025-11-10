use parking_lot::Mutex;
use std::sync::LazyLock;

/// Shared buffer holding the most recent JNI error message.
static LAST_ERROR_MESSAGE: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new(String::new()));

fn buffer() -> &'static Mutex<String> {
    LazyLock::force(&LAST_ERROR_MESSAGE)
}

/// Returns a raw pointer to the shared error buffer so it can be cached on the
/// Kotlin side. The pointer is stable for the lifetime of the process.
pub fn get_last_error_message_ptr() -> *const Mutex<String> {
    buffer() as *const Mutex<String>
}

/// Provides direct access to the shared error buffer for callers that prefer a
/// reference over the raw pointer.
pub fn last_error_message_mutex() -> &'static Mutex<String> {
    buffer()
}

/// Replaces the stored error message with `message`.
pub fn set_last_error_message(message: impl AsRef<str>) {
    let mut guard = buffer().lock();
    guard.clear();
    guard.push_str(message.as_ref());
}

/// Clears any stored error message.
pub fn clear_last_error_message() {
    buffer().lock().clear();
}

/// Clones and returns the currently stored error message.
///
/// The buffer contents remain untouched so callers can fetch the message multiple
/// times if necessary.
pub fn get_last_error_message() -> String {
    buffer().lock().clone()
}

/// Convenience helper for appending additional context onto the current
/// error message.
pub fn append_last_error_message(fragment: impl AsRef<str>) {
    let mut guard = buffer().lock();
    if !guard.is_empty() {
        guard.push('\n');
    }
    guard.push_str(fragment.as_ref());
}
