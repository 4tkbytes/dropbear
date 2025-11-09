package com.dropbear.exception

/**
 * Exception thrown when a native call fails.
 */
class DropbearNativeException(message: String? = null, cause: Throwable? = null): Exception(message, cause)