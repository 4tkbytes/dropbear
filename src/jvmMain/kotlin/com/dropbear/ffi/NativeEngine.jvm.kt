package com.dropbear.ffi

import com.dropbear.Camera
import com.dropbear.EntityId
import com.dropbear.input.KeyCode
import com.dropbear.input.MouseButton
import com.dropbear.input.MouseButtonCodes
import com.dropbear.math.Transform
import com.dropbear.math.Vector2D

actual class NativeEngine {
    private var worldHandle: Long = 0L
    private var inputHandle: Long = 0L

    /**
     * The handle/pointer to the graphics queue.
     *
     * Contrary-to-belief, this is different from the `Arc<SharedGraphicsContext>` handle
     * as such in the game engine, but rather a pointer to a static variable called `GRAPHICS_COMMANDS`.
     *
     * Since winit (the windowing library) requires all commands to be done on the main thread, this variable
     * allows for "commands" to be sent over and processed on the main thread with the crossbeam_channels library.
     */
    private var graphicsHandle: Long = 0L

    actual fun getEntity(label: String): Long? {
        val result = JNINative.getEntity(worldHandle, label)
        return if (result == -1L) {
            null
        } else {
            result
        }
    }

    @JvmName("init")
    fun init(worldHandle: Long, inputHandle: Long, graphicsHandle: Long) {
        this.worldHandle = worldHandle
        this.inputHandle = inputHandle
        this.graphicsHandle = graphicsHandle
        if (this.worldHandle < 0L) {
            println("NativeEngine: Error - Invalid world handle received!")
            return
        }
        if (this.inputHandle < 0L) {
            println("NativeEngine: Error - Invalid input handle received!")
            return
        }
        if (this.graphicsHandle < 0L) {
            println("NativeEngine: Error - Invalid graphics handle received!")
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
        JNINative.setCursorLocked(inputHandle, graphicsHandle, locked)
    }

    actual fun getLastMousePos(): Vector2D? {
        val result = JNINative.getLastMousePos(inputHandle);
        return Vector2D(result[0].toDouble(), result[1].toDouble())
    }

    actual fun getStringProperty(entityHandle: Long, label: String): String? {
        return JNINative.getStringProperty(worldHandle, entityHandle, label)
    }

    actual fun getIntProperty(entityHandle: Long, label: String): Int? {
        val result = JNINative.getIntProperty(worldHandle, entityHandle, label)
        return if (result == 650911) {
            null
        } else {
            result
        }
    }

    actual fun getLongProperty(entityHandle: Long, label: String): Long? {
        val result = JNINative.getLongProperty(worldHandle, entityHandle, label)
        return if (result == 6509112938) {
            null
        } else {
            result
        }
    }

    actual fun getFloatProperty(entityHandle: Long, label: String): Float? {
        val result = JNINative.getFloatProperty(worldHandle, entityHandle, label)
        return if (result.isNaN()) {
            null
        } else {
            result.toFloat()
        }
    }

    actual fun getDoubleProperty(entityHandle: Long, label: String): Double? {
        val result = JNINative.getFloatProperty(worldHandle, entityHandle, label)
        return if (result.isNaN()) {
            null
        } else {
            result
        }
    }

    actual fun getBoolProperty(entityHandle: Long, label: String): Boolean? {
        return JNINative.getBoolProperty(worldHandle, entityHandle, label)
    }

    actual fun getVec3Property(entityHandle: Long, label: String): FloatArray? {
        return JNINative.getVec3Property(worldHandle, entityHandle, label)
    }

    actual fun setStringProperty(entityHandle: Long, label: String, value: String) {
        JNINative.setStringProperty(worldHandle, entityHandle, label, value)
    }

    actual fun setIntProperty(entityHandle: Long, label: String, value: Int) {
        JNINative.setIntProperty(worldHandle, entityHandle, label, value)
    }

    actual fun setLongProperty(entityHandle: Long, label: String, value: Long) {
        JNINative.setLongProperty(worldHandle, entityHandle, label, value)
    }

    actual fun setFloatProperty(entityHandle: Long, label: String, value: Double) {
        JNINative.setFloatProperty(worldHandle, entityHandle, label, value)
    }

    actual fun setBoolProperty(entityHandle: Long, label: String, value: Boolean) {
        JNINative.setBoolProperty(worldHandle, entityHandle, label, value)
    }

    actual fun setVec3Property(entityHandle: Long, label: String, value: FloatArray) {
        JNINative.setVec3Property(worldHandle, entityHandle, label, value)
    }

    actual fun getCamera(label: String): Camera? {
        return JNINative.getCamera(worldHandle, label)
    }

    actual fun getAttachedCamera(entityId: EntityId): Camera? {
        return JNINative.getAttachedCamera(worldHandle, entityId.id)
    }

    actual fun setCamera(camera: Camera) {
        JNINative.setCamera(worldHandle, camera)
    }

    actual fun isCursorHidden(): Boolean {
        return JNINative.isCursorHidden(inputHandle)
    }

    actual fun setCursorHidden(hidden: Boolean) {
        JNINative.setCursorHidden(inputHandle, graphicsHandle, hidden)
    }
}