package com.dropbear.ffi

import com.dropbear.EntityId
import com.dropbear.input.KeyCode
import com.dropbear.math.Transform

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

    /**
     * Prints the input state, typically used for debugging.
     */
    fun printInputState()

    /**
     * Checks if a Key is pressed by its KeyCode
     */
    fun isKeyPressed(key: KeyCode): Boolean
}