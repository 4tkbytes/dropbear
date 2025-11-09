package com.dropbear.asset

import com.dropbear.DropbearEngine

class TextureHandle(private val id: Long): Handle(id) {
    override fun asAssetHandle(): AssetHandle = AssetHandle(id)

    fun getName(engine: DropbearEngine): String? {
        return engine.native.getTextureName(id)
    }

    override fun toString(): String {
        return "TextureHandle(id=$id)"
    }
}