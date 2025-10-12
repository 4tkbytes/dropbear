package com.dropbear.ffi

actual class NativeEngine {
    private var worldHandle: Long = 0L

    actual fun getEntity(label: String): Long? {
        val result = JNINative.getEntity(worldHandle, label)
        return if (result < 0) {
            null
        } else {
            result
        }
    }

    @JvmName("init")
    fun init(handle: Long) {
        this.worldHandle = handle
        if (this.worldHandle < 0L) {
            println("NativeEngine: Error - Invalid world handle received!")
            return
        }
    }
}