package com.dropbear

import com.dropbear.ffi.NativeEngine
import com.dropbear.math.Transform

class DropbearEngine(val native: NativeEngine) {
    public fun getEntity(label: String): EntityRef? {
        val entityId = native.getEntity(label)
        val entityRef = if (entityId != null) EntityRef(EntityId(entityId)) else null
        entityRef?.engine = this
        return entityRef
    }

    internal fun getTransform(entityId: EntityId): Transform? {
        val result = native.getTransform(entityId)
        return result
    }

    internal fun setTransform(entityId: EntityId, transform: Transform) {
        native.setTransform(entityId, transform)
    }
}