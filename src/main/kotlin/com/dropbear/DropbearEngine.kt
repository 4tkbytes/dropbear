package com.dropbear

import com.dropbear.ffi.NativeEngine

class DropbearEngine {
    var nativeEngine: NativeEngine? = null

    // figure out how to create a new class
    /**
     * Fetches the currently active entity the script is attached to.
     *
     * If there is no entity this script is attached to, it will return
     * a `null` in the form of a [Result]
     */
    fun getActiveEntity(): Result<EntityRef> {
        return Result.failure(Exception("Function not implemented"))
    }

    /**
     * Fetches an entity based on its label in the editor.
     *
     * If there is no entity under that label, it will return a
     * `null` in the form of a [Result]
     */
    fun getEntity(label: String): EntityRef? {
        return null
    }
}