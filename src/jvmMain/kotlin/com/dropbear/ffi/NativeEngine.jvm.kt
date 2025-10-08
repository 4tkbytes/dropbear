package com.dropbear.ffi

actual class NativeEngine {
    private var worldHandle: ULong = 0u
    private val jni = JNINative()

    actual fun getEntity(label: String): ULong? {
        val result = jni.getEntity(worldHandle.toLong(), label)
        return if (result < 0) {
            null
        } else {
            result.toULong()
        }
    }

    actual fun init(handle: ULong) {
        this.worldHandle = handle
        if (this.worldHandle == 0uL) {
            println("NativeEngine: Error - Invalid world handle received!")
            return
        }
    }
}