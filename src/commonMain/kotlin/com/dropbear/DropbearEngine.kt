package com.dropbear

import com.dropbear.ffi.NativeEngine

class DropbearEngine(val native: NativeEngine) {

    fun getEntity(label: String): Result<EntityRef> {
        val entityId = native.getEntity(label)
        return (when (entityId) {
            null -> {
                Result.failure(Exception("JNI returned null"))
            }
            else -> {
                Result.success(EntityRef(EntityId(entityId)))
            }
        })
    }
}