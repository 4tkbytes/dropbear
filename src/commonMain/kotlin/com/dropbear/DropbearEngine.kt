package com.dropbear

import com.dropbear.asset.AssetHandle
import com.dropbear.ffi.NativeEngine
import com.dropbear.input.InputState
import com.dropbear.logging.Logger
import com.dropbear.math.Transform

internal var exceptionOnError: Boolean = false

class DropbearEngine(val native: NativeEngine) {
    private var inputState: InputState? = null

    companion object {
        /**
         * Globally sets whether exceptions should be thrown when an error occurs.
         *
         * This can be run in your update loop without consequences.
         */
        fun callExceptionOnError(toggle: Boolean) {
            exceptionOnError = toggle
        }
    }

    fun getEntity(label: String): EntityRef? {
        val entityId = native.getEntity(label)
        val entityRef = if (entityId != null) EntityRef(EntityId(entityId)) else null
        entityRef?.engine = this
        return entityRef
    }

    fun getCamera(label: String): Camera? {
        val result = native.getCamera(label)
        if (result != null) {
            result.engine = this
        }
        return result
    }

    fun getInputState(): InputState {
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

    fun getAsset(eucaURI: String): AssetHandle? {
        val id = native.getAsset(eucaURI)
        return if (id != null) AssetHandle(id) else null
    }
}