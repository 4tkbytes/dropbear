package com.dropbear

import com.dropbear.asset.ModelHandle
import com.dropbear.asset.TextureHandle
import com.dropbear.exception.DropbearNativeException
import com.dropbear.math.Transform

class EntityRef(val id: EntityId = EntityId(0L)) {
    lateinit var engine: DropbearEngine

    override fun toString(): String {
        return "EntityRef(id=$id)"
    }

    fun getTransform(): Transform? {
        return engine.getTransform(id)
    }

    fun setTransform(transform: Transform?) {
        if (transform == null) return
        return engine.setTransform(id, transform)
    }

    inline fun <reified T> getProperty(key: String): T? {
        return when (T::class) {
            String::class -> engine.native.getStringProperty(id.id, key) as T?
            Long::class -> engine.native.getLongProperty(id.id, key) as T?
            Int::class -> engine.native.getIntProperty(id.id, key) as T?
            Double::class -> engine.native.getDoubleProperty(id.id, key) as T?

            Float::class -> engine.native.getFloatProperty(id.id, key) as T?
            Boolean::class -> engine.native.getBoolProperty(id.id, key) as T?
            FloatArray::class -> engine.native.getVec3Property(id.id, key) as T?
            else -> throw IllegalArgumentException("Unsupported property type: ${T::class}")
        }
    }

    fun setProperty(key: String, value: Any) {
        when (value) {
            is String -> engine.native.setStringProperty(id.id, key, value)
            is Long -> engine.native.setLongProperty(id.id, key, value)
            is Int -> engine.native.setIntProperty(id.id, key, value)
            is Double -> engine.native.setFloatProperty(id.id, key, value)
            is Float -> engine.native.setFloatProperty(id.id, key, value.toDouble())
            is Boolean -> engine.native.setBoolProperty(id.id, key, value)
            is FloatArray -> {
                require(value.size == 3) { "Vec3 property must have exactly 3 elements" }
                engine.native.setVec3Property(id.id, key, value)
            }
            else -> throw IllegalArgumentException("Unsupported property type: ${value::class}")
        }
    }

    fun getAttachedCamera(): Camera? {
        val result = engine.native.getAttachedCamera(id)
        if (result != null) {
            result.engine = this.engine
        }
        return result
    }

    fun getTexture(materialName: String): TextureHandle? {
        val result = engine.native.getTexture(id.id, materialName)
        if (result == -1L) {
            if (exceptionOnError) {
                throw DropbearNativeException("Unable to get texture for material $materialName")
            }
            return null
        } else {
            return TextureHandle(result ?: throw Exception("Native returned null texture handle"))
        }
    }

    fun hasTexture(eucaURI: String): Boolean {
        return engine.native.isUsingTexture(id.id, eucaURI)
    }

    fun setModel(modelHandle: ModelHandle) {
        engine.native.setModel(id.id, modelHandle.raw())
    }

    fun usingModel(modelHandle: ModelHandle): Boolean {
        return engine.native.isUsingModel(id.id, modelHandle.raw())
    }

    fun setTextureOverride(materialName: String, textureHandle: TextureHandle) {
        engine.native.setTextureOverride(id.id, materialName, textureHandle)
    }
}