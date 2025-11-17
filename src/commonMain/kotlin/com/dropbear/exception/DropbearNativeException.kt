package com.dropbear.exception

/**
 * Exception thrown when a native call fails.
 *
 * This will only be thrown if `DropbearEngine.callExceptionOnError()` is enabled or if
 * the error is crucial to the runtime. Otherwise, expect that function to return null on error.
 */
class DropbearNativeException(message: String? = null, cause: Throwable? = null): Exception(message, cause)
