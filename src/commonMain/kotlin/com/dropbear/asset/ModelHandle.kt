package com.dropbear.asset

class ModelHandle(private val id: Long): Handle(id) {
    override fun asAssetHandle(): AssetHandle = AssetHandle(id)

    override fun toString(): String {
        return "ModelHandle(id=$id)"
    }
}