package com.dropbear

import com.dropbear.ffi.NativeEngine

class DropbearEngine(val native: NativeEngine) {
    /**
     * Fetches an entity based on its label in the editor.
     *
     * If there is no entity under that label, it will return a
     * `null` in the form of a [Result]
     */
    fun getEntity(label: String): Result<EntityRef> {
        val entityId = native.getEntity(label)
        return (when (entityId) {
            null -> {
                Result.failure(Exception("JNI returned null"))
            }
            -1L -> {
                // -1L means that query couldn't find entity with such a label
                Result.failure(Exception("Entity with id $label not found"))
            }
            else -> {
                Result.success(EntityRef(EntityId(entityId)))
            }
        })
    }
}