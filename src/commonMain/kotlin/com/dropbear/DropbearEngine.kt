package com.dropbear

import com.dropbear.ffi.NativeEngine
import getProjectScriptMetadata

class DropbearEngine(val native: NativeEngine) {
    private val globalScripts = mutableListOf<System>()

    fun init() {
        val scriptRegistration = getProjectScriptMetadata()
    }

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