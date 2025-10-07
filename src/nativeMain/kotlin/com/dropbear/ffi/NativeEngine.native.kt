@file:OptIn(ExperimentalForeignApi::class)
@file:Suppress("EXPECT_ACTUAL_CLASSIFIERS_ARE_IN_BETA_WARNING")

package com.dropbear.ffi

import kotlinx.cinterop.*

actual class NativeEngine {
    actual fun getEntity(label: String): Long? {
        return com.dropbear.ffi.native.dropbear_get_entity(label)
//        return 0L
    }
}