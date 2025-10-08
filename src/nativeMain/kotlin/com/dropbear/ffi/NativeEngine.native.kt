@file:OptIn(ExperimentalForeignApi::class, ExperimentalNativeApi::class)
@file:Suppress("EXPECT_ACTUAL_CLASSIFIERS_ARE_IN_BETA_WARNING")

package com.dropbear.ffi

import co.touchlab.kermit.Logger
import kotlinx.cinterop.*
import kotlin.experimental.ExperimentalNativeApi

actual class NativeEngine {
    private var worldHandle: ULong = 0u

    actual fun init(handle: ULong) {
        this.worldHandle = handle
        if (this.worldHandle == 0uL) {
            Logger.i("NativeEngine: Error - Invalid world handle received!")
            return
        } else {
            Logger.i("NativeEngine: Initialized with world handle: ${this.worldHandle}")
        }
    }

    actual fun getEntity(label: String): ULong? {
        TODO("Not yet implemented")
    }
}