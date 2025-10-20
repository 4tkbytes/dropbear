package com.dropbear.ffi

import com.dropbear.EntityId
import com.dropbear.input.KeyCode
import com.dropbear.input.MouseButton
import com.dropbear.math.Transform
import com.dropbear.math.Vector2D

expect class NativeEngine {
    /**
     * Fetches the entity from its label, returning a [Long] (the entity ID)
     */
    fun getEntity(label: String): Long?

    /**
     * Fetches the [Transform] component of an entity by it's ID
     */
    fun getTransform(entityId: EntityId): Transform?

    /**
     * Sets an entities [Transform] component.
     */
    fun setTransform(entityId: EntityId, transform: Transform)

    // ------------------------ MODEL PROPERTIES -------------------------

    fun getStringProperty(entityHandle: Long, label: String): String?
    fun getIntProperty(entityHandle: Long, label: String): Int?
    fun getLongProperty(entityHandle: Long, label: String): Long?
    fun getDoubleProperty(entityHandle: Long, label: String): Double?
    fun getFloatProperty(entityHandle: Long, label: String): Float?
    fun getBoolProperty(entityHandle: Long, label: String): Boolean?
    fun getVec3Property(entityHandle: Long, label: String): FloatArray?

    fun setStringProperty(entityHandle: Long, label: String, value: String)
    fun setIntProperty(entityHandle: Long, label: String, value: Int)
    fun setLongProperty(entityHandle: Long, label: String, value: Long)
    fun setFloatProperty(entityHandle: Long, label: String, value: Double)
    fun setBoolProperty(entityHandle: Long, label: String, value: Boolean)
    fun setVec3Property(entityHandle: Long, label: String, value: FloatArray)


    // --------------------------- INPUT STATE ---------------------------

    /**
     * Prints the input state, typically used for debugging.
     */
    fun printInputState()
    /**
     * Checks if a Key is pressed by its KeyCode
     */
    fun isKeyPressed(key: KeyCode): Boolean
    fun getMousePosition(): Vector2D?
    fun isMouseButtonPressed(button: MouseButton): Boolean
    fun getMouseDelta(): Vector2D?
    fun isCursorLocked(): Boolean
    fun setCursorLocked(locked: Boolean)
    fun getLastMousePos(): Vector2D?
//    fun getConnectedGamepads(): List<Gamepad>

    // -------------------------------------------------------------------
}