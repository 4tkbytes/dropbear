package com.dropbear.ffi

import com.dropbear.Camera
import com.dropbear.EntityId
import com.dropbear.asset.AssetHandle
import com.dropbear.asset.ModelHandle
import com.dropbear.asset.TextureHandle
import com.dropbear.input.KeyCode
import com.dropbear.input.MouseButton
import com.dropbear.math.Transform
import com.dropbear.math.Vector2D

expect class NativeEngine {
    fun getEntity(label: String): Long?
    fun getAsset(eucaURI: String): Long?

    fun getModel(entityHandle: Long): Long?
    fun setModel(entityHandle: Long, modelHandle: Long)
    fun isUsingModel(entityHandle: Long, modelHandle: Long): Boolean
    fun isModelHandle(id: Long): Boolean

    fun getTexture(entityHandle: Long, name: String): Long?
    fun setTextureOverride(entityHandle: Long, oldMaterialName: String, newTextureHandle: TextureHandle)
    fun isUsingTexture(entityHandle: Long, textureHandle: Long): Boolean
    fun isTextureHandle(id: Long): Boolean
    fun getTextureName(textureHandle: Long): String?
    fun getAllTextures(entityHandle: Long): Array<String>

    fun getCamera(label: String): Camera?
    fun getAttachedCamera(entityId: EntityId): Camera?
    fun setCamera(camera: Camera);

    fun getTransform(entityId: EntityId): Transform?
    fun setTransform(entityId: EntityId, transform: Transform)

    // ------------------------ MODEL PROPERTIES -------------------------

    fun getStringProperty(entityHandle: Long, label: String): String?
    fun getIntProperty(entityHandle: Long, label: String): Int?
    fun getLongProperty(entityHandle: Long, label: String): Long?
    fun getDoubleProperty(entityHandle: Long, label: String): Double?
    fun getFloatProperty(entityHandle: Long, label: String): Float?
    fun getBoolProperty(entityHandle: Long, label: String): Boolean?
    fun getVec3Property(entityHandle: Long, label: String): FloatArray?

    fun setStringProperty(entityHandle: Long, label: String, value: String)
    fun setIntProperty(entityHandle: Long, label: String, value: Int)
    fun setLongProperty(entityHandle: Long, label: String, value: Long)
    fun setFloatProperty(entityHandle: Long, label: String, value: Double)
    fun setBoolProperty(entityHandle: Long, label: String, value: Boolean)
    fun setVec3Property(entityHandle: Long, label: String, value: FloatArray)


    // --------------------------- INPUT STATE ---------------------------

    /**
     * Prints the input state, typically used for debugging.
     */
    fun printInputState()
    fun isKeyPressed(key: KeyCode): Boolean
    fun getMousePosition(): Vector2D?
    fun isMouseButtonPressed(button: MouseButton): Boolean
    fun getMouseDelta(): Vector2D?
    fun isCursorLocked(): Boolean
    fun setCursorLocked(locked: Boolean)
    fun isCursorHidden(): Boolean
    fun setCursorHidden(hidden: Boolean)
    fun getLastMousePos(): Vector2D?
    fun getLastErrorMsg(): String?
    fun getLastErrorMsgPtr(): Long
//    fun getConnectedGamepads(): List<Gamepad>

    // -------------------------------------------------------------------
}
