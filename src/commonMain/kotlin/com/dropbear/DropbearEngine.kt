package com.dropbear

import com.dropbear.ffi.NativeEngine
import com.dropbear.input.InputState
import com.dropbear.logging.Logger
import com.dropbear.math.Transform

class DropbearEngine(val native: NativeEngine) {
    private var inputState: InputState? = null

    public fun getEntity(label: String): EntityRef? {
        val entityId = native.getEntity(label)
        val entityRef = if (entityId != null) EntityRef(EntityId(entityId)) else null
        entityRef?.engine = this
        return entityRef
    }

    public fun getInputState(): InputState {
        if (this.inputState == null) {
            Logger.trace("InputState not initialised, creating new one")
            this.inputState = InputState(this)
        }
        return this.inputState!!
    }

    internal fun getTransform(entityId: EntityId): Transform? {
        val result = native.getTransform(entityId)
        return result
    }

    internal fun setTransform(entityId: EntityId, transform: Transform) {
        native.setTransform(entityId, transform)
    }
}