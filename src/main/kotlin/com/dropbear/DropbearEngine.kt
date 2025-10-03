package com.dropbear

import com.dropbear.ffi.NativeEngine
import com.dropbear.math.Transform

/**
 * Main interface to the Dropbear game engine.
 * 
 * This class provides high-level access to engine features for Kotlin scripts.
 * It wraps the low-level NativeEngine JNI bindings with a more ergonomic API.
 */
class DropbearEngine {
    /**
     * Get the transform of the current entity.
     * 
     * @return A Transform object that provides live access to position, rotation, and scale
     * 
     * @example
     * ```kotlin
     * val transform = engine.getTransform()
     * transform.position.y += 0.1 // Move up
     * ```
     */
    fun getTransform(): Transform {
        return Transform.fromNative()
    }

    /**
     * Fetches the currently active entity the script is attached to.
     *
     * If there is no entity this script is attached to, it will return
     * a `null` in the form of a [Result]
     */
    fun getActiveEntity(): Result<EntityRef> {
        // This would need additional native support to get entity ID and label
        return Result.failure(Exception("Function not fully implemented - requires entity label support"))
    }

    /**
     * Fetches an entity based on its label in the editor.
     *
     * If there is no entity under that label, it will return a
     * `null` in the form of a [Result]
     */
    fun getEntity(label: String): EntityRef? {
        // This would need additional native support for entity lookup by label
        return null
    }
}
