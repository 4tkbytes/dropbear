package com.dropbear.ffi

import com.dropbear.EntityId
import com.dropbear.input.KeyCode
import com.dropbear.math.Transform

actual class NativeEngine {
    private var worldHandle: Long = 0L
    private var inputHandle: Long = 0L

    actual fun getEntity(label: String): Long? {
        return JNINative.getEntity(worldHandle, label)
    }

    @JvmName("init")
    fun init(worldHandle: Long, inputHandle: Long) {
        this.worldHandle = worldHandle
        this.inputHandle = inputHandle
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

    actual fun printInputState() {
        return JNINative.printInputState(inputHandle)
    }

    actual fun isKeyPressed(key: KeyCode): Boolean {
        return JNINative.isKeyPressed(inputHandle, key.ordinal)
    }
}