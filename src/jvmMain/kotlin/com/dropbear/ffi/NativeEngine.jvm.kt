package com.dropbear.ffi

import com.dropbear.EntityId
import com.dropbear.math.Transform

actual class NativeEngine {
    private var worldHandle: Long = 0L

    actual fun getEntity(label: String): Long? {
        return JNINative.getEntity(worldHandle, label)
    }

    @JvmName("init")
    fun init(handle: Long) {
        this.worldHandle = handle
        if (this.worldHandle < 0L) {
            println("NativeEngine: Error - Invalid world handle received!")
            return
        }
    }

    actual fun getTransform(entityId: EntityId): Transform? {
        return JNINative.getTransform(worldHandle, entityId.id)
    }

    actual fun setTransform(entityId: EntityId, transform: Transform) {
        return JNINative.setTransform(worldHandle, entityId.id, transform)
    }
}