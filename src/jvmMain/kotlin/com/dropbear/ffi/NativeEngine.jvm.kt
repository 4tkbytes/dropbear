package com.dropbear.ffi

actual class NativeEngine {
    private val jni = JNINative()

    actual fun getEntity(label: String): Long? {
        return jni.getEntity(label)
    }
}