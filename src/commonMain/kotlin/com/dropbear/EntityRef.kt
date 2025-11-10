package com.dropbear

import com.dropbear.asset.ModelHandle
import com.dropbear.asset.TextureHandle
import com.dropbear.math.Transform

/**
 * A reference to an ECS Entity stored inside the dropbear engine.
 *
 * The dropbear engine prefers careful mutability, which is why a reference is passed (as a handle) instead
 * of its full information. Also conserves memory.
 *
 * The ECS system the dropbear engine uses is `hecs` ECS, which is a Rust crate that has blazing fast
 * querying systems. The id passed is just a primitive integer value that points to the entity in the world.
 *
 * @property id The unique identifier of the entity as set by `hecs::World`
 */
class EntityRef(val id: EntityId = EntityId(0L)) {
    lateinit var engine: DropbearEngine

    override fun toString(): String {
        return "EntityRef(id=$id)"
    }

    /**
     * Fetches the transform component for the entity.
     */
    fun getTransform(): Transform? {
        return engine.native.getTransform(id)
    }

    /**
     * Sets and replaces the transform component for the entity.
     */
    fun setTransform(transform: Transform?) {
        if (transform == null) return
        return engine.native.setTransform(id, transform)
    }

    /**
     * Fetches the property of the ModelProperty component on the entity.
     */
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

    /**
     * Sets a property of the ModelProperty component on the entity.
     *
     * # Supported types
     * - [kotlin.String]
     * - [kotlin.Long]
     * - [kotlin.Int]
     * - [kotlin.Double]
     * - [kotlin.Float]
     * - [kotlin.Boolean]
     * - [com.dropbear.math.Vector3]
     */
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

    /**
     * Fetches the attached camera for the entity.
     *
     * Returns null if no camera is attached as a component according to the editor.
     */
    fun getAttachedCamera(): Camera? {
        val result = engine.native.getAttachedCamera(id)
        if (result != null) {
            result.engine = this.engine
        }
        return result
    }

    /**
     * Fetches the texture for the given material name in the model.
     */
    fun getTexture(materialName: String): TextureHandle? {
        val result = engine.native.getTexture(id.id, materialName)
        return if (result == null) {
            null
        } else {
            TextureHandle(result)
        }
    }

    /**
     * Returns an array containing the texture identifiers applied to this entity's model.
     */
    fun getAllTextures(): Array<String> {
        return engine.native.getAllTextures(id.id)
    }

    /**
     * Checks if the current model being rendered by this entity contains the texture with the given [TextureHandle]
     */
    fun hasTexture(textureHandle: TextureHandle): Boolean {
        return engine.native.isUsingTexture(id.id, textureHandle.raw())
    }

    /**
     * Fetches the active model that is currently being used
     */
    fun getModel(): ModelHandle? {
        val result = engine.native.getModel(id.id)
        return if (result == null) {
            null
        } else {
            ModelHandle(result)
        }
    }

    /**
     * Sets the active model for the entity from a ModelHandle
     */
    fun setModel(modelHandle: ModelHandle) {
        engine.native.setModel(id.id, modelHandle.raw())
    }

    /**
     * Checks if the entity is currently using the given model handle.
     *
     * Returns false if not using, true if is.
     */
    fun usingModel(modelHandle: ModelHandle): Boolean {
        return engine.native.isUsingModel(id.id, modelHandle.raw())
    }

    /**
     * Sets a texture override for the given material on the active model.
     */
    fun setTextureOverride(materialName: String, textureHandle: TextureHandle) {
        engine.native.setTextureOverride(id.id, materialName, textureHandle)
    }
}