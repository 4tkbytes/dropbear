package com.dropbear.ffi

import com.dropbear.EntityId
import com.dropbear.input.KeyCode
import com.dropbear.input.MouseButton
import com.dropbear.input.MouseButtonCodes
import com.dropbear.math.Transform
import com.dropbear.math.Vector2D

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

    actual fun getMousePosition(): Vector2D? {
        val result = JNINative.getMousePosition(inputHandle);
        return Vector2D(result[0].toDouble(), result[1].toDouble())
    }

    actual fun isMouseButtonPressed(button: MouseButton): Boolean {
        val buttonCode: Int = when (button) {
            MouseButton.Left -> MouseButtonCodes.LEFT
            MouseButton.Right -> MouseButtonCodes.RIGHT
            MouseButton.Middle -> MouseButtonCodes.MIDDLE
            MouseButton.Back -> MouseButtonCodes.BACK
            MouseButton.Forward -> MouseButtonCodes.FORWARD
            is MouseButton.Other -> button.value
        }

        return JNINative.isMouseButtonPressed(inputHandle, buttonCode)
    }

    actual fun getMouseDelta(): Vector2D? {
        val result = JNINative.getMouseDelta(inputHandle);
        return Vector2D(result[0].toDouble(), result[1].toDouble())
    }

    actual fun isCursorLocked(): Boolean {
        return JNINative.isCursorLocked(inputHandle)
    }

    actual fun setCursorLocked(locked: Boolean) {
        JNINative.setCursorLocked(inputHandle, locked)
    }

    actual fun getLastMousePos(): Vector2D? {
        val result = JNINative.getLastMousePos(inputHandle);
        return Vector2D(result[0].toDouble(), result[1].toDouble())
    }
}