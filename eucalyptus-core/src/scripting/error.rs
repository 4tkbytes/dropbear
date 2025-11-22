#[allow(unused_imports)]
use crate::scripting::ScriptTarget;

/// A trait implemented by the different script types ([ScriptTarget::JVM], [ScriptTarget::Native]) which allow
/// for populating the last error and getting the contents of the last error.
#[allow(dead_code)]
pub trait LastErrorMessage {
    /// Fetches the last error message.
    ///
    /// # Platform specific behaviours
    /// - [ScriptTarget::JVM] - On the JVM, there is a variable listed as `com.dropbear.lastErrorMessage`, which happens to be static.
    ///   This variable is returned by the JVM. Returns `null` (for Rust, [None]) if there is no error.
    /// - [ScriptTarget::Native] - Just like the JVM target, the variable is accessed using a CName (`dropbear_get_last_error_message`),
    ///   and the value (`com.dropbear.lastErrorMessage`) is returned. If the pointer is null, it means there was no error that has occurred (yet).
    fn get_last_error(&self) -> Option<String>;

    /// Sets the error message.
    ///
    /// # Platform specific behaviours
    /// - [ScriptTarget::JVM] - On the JVM, `com.dropbear.lastErrorMessage` (static) is set with the help of [jni::JNIEnv::set_static_field].
    /// - [ScriptTarget::Native] - `com.dropbear.lastErrorMessage` (static) is accessed using a CName (`dropbear_set_last_error_message`) and the value is set as a [CString].
    fn set_last_error(&self, err_msg: impl Into<String>) -> anyhow::Result<()>;
}
