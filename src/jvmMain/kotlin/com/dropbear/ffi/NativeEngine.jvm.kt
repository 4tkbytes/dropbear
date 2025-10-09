package com.dropbear.ffi

actual class NativeEngine {
    private var worldHandle: ULong = 0u

    actual fun getEntity(label: String): ULong? {
        val result = JNINative.getEntity(worldHandle.toLong(), label)
        return if (result < 0) {
            null
        } else {
            result.toULong()
        }
    }

    fun init(handle: ULong) {
        this.worldHandle = handle
        if (this.worldHandle == 0uL) {
            println("NativeEngine: Error - Invalid world handle received!")
            return
        }
    }
}