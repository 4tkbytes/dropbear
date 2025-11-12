package com.dropbear.asset

import com.dropbear.DropbearEngine

/**
 * A handle that describes the type of asset in the ASSET_REGISTRY
 */
class AssetHandle(private val id: Long): Handle(id) {
    /**
     * Converts an [AssetHandle] to a [ModelHandle].
     *
     * It can return null if the asset is not a model.
     */
    fun asModelHandle(engine: DropbearEngine): ModelHandle? {
        val result = engine.native.isModelHandle(id)
        return if (result) {
            ModelHandle(id)
        } else {
            null
        }
    }

    /**
     * Converts an [AssetHandle] to a [TextureHandle].
     *
     * It can return null if the asset is not a texture.
     */
    fun asTextureHandle(engine: DropbearEngine): TextureHandle? {
        val result = engine.native.isTextureHandle(id)
        return if (result) {
            TextureHandle(id)
        } else {
            null
        }
    }

    override fun asAssetHandle(): AssetHandle {
        return this
    }

    override fun toString(): String {
        return "AssetHandle(id=$id)"
    }
}