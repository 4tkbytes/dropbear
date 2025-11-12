package com.dropbear.asset

/**
 * Another type of asset handle, this wraps the id of the asset
 * into something that only models can access. 
 */
class ModelHandle(private val id: Long): Handle(id) {
    override fun asAssetHandle(): AssetHandle = AssetHandle(id)

    override fun toString(): String {
        return "ModelHandle(id=$id)"
    }
}