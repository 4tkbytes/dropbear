package com.dropbear

import com.dropbear.asset.AssetHandle
import com.dropbear.ffi.NativeEngine
import com.dropbear.input.InputState
import com.dropbear.logging.Logger

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

    /**
     * Fetches an [EntityRef] with the given label.
     */
    fun getEntity(label: String): EntityRef? {
        val entityId = native.getEntity(label)
        val entityRef = if (entityId != null) EntityRef(EntityId(entityId)) else null
        entityRef?.engine = this
        return entityRef
    }

    /**
     * Fetches the information of the camera with the given label.
     */
    fun getCamera(label: String): Camera? {
        val result = native.getCamera(label)
        if (result != null) {
            result.engine = this
        }
        return result
    }

    /**
     * Gets the current [InputState] for that frame.
     */
    fun getInputState(): InputState {
        if (this.inputState == null) {
            Logger.trace("InputState not initialised, creating new one")
            this.inputState = InputState(this)
        }
        return this.inputState!!
    }

    /**
     * Fetches the asset information from the internal AssetRegistry (located in
     * `dropbear_engine::asset::AssetRegistry`).
     *
     * ## Warning
     * The eucalyptus asset URI (or `euca://`) is case-sensitive.
     */
    fun getAsset(eucaURI: String): AssetHandle? {
        val id = native.getAsset(eucaURI)
        return if (id != null) AssetHandle(id) else null
    }

    /**
     * Globally sets whether exceptions should be thrown when an error occurs.
     *
     * This can be run in your update loop without consequences.
     */
    fun callExceptionOnError(toggle: Boolean) = DropbearEngine.callExceptionOnError(toggle)

    /**
     * Fetches the last error message during the native call. 
     */
    fun getLastErrorMsg(): String? = native.getLastErrorMsg()

    fun getLastErrorMsgPtr(): Long = native.getLastErrorMsgPtr()
}