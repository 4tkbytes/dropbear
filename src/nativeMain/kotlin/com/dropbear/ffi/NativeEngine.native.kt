@file:OptIn(ExperimentalForeignApi::class, ExperimentalNativeApi::class)
@file:Suppress("EXPECT_ACTUAL_CLASSIFIERS_ARE_IN_BETA_WARNING")

package com.dropbear.ffi

import co.touchlab.kermit.Logger
import com.dropbear.ffi.generated.dropbear_get_entity
import kotlinx.cinterop.*
import kotlin.experimental.ExperimentalNativeApi

actual class NativeEngine {
    private var worldHandle: COpaquePointer? = null

    fun init(handle: COpaquePointer?) {
        this.worldHandle = handle
        if (this.worldHandle == null) {
            Logger.i("NativeEngine: Error - Invalid world handle received!")
        }
    }

    actual fun getEntity(label: String): Long? {
        val world = worldHandle ?: return null
        memScoped {
            val outEntity = alloc<LongVar>()
            val result = dropbear_get_entity(
                label = label,
                world_ptr = world.reinterpret(),
                out_entity = outEntity.ptr
            )
            return if (result == 0) outEntity.value else null
        }
    }
}