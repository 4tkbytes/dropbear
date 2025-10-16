package com.dropbear.ffi

import com.dropbear.EntityId
import com.dropbear.math.Transform

expect class NativeEngine {
    fun getEntity(label: String): Long?
    fun getTransform(entityId: EntityId): Transform?
    fun setTransform(entityId: EntityId, transform: Transform)
}