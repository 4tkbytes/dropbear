package com.dropbear.asset

/**
 * Describes a handle of an asset, or anything really.
 *
 * Aims to allow people to group up different handle types ([AssetHandle], [ModelHandle] etc...)
 * into a list or a vector.
 */
abstract class Handle(private val id: Long) {
    /**
     * Returns the raw id of the handle
     */
    fun raw(): Long = id

    /**
     * Returns the handle as an [AssetHandle].
     *
     * This will not return null as all handles are a type of [AssetHandle].
     */
    abstract fun asAssetHandle(): AssetHandle

    override fun toString(): String {
        return "Handle(id=$id)"
    }
}