@file:OptIn(ExperimentalForeignApi::class, ExperimentalNativeApi::class)

package com.dropbear.ffi

import co.touchlab.kermit.Logger
import com.dropbear.DropbearEngine
import kotlinx.cinterop.ExperimentalForeignApi
import kotlin.experimental.ExperimentalNativeApi

@CName("dropbear_entry")
fun entry(worldHandle: ULong) {
    Logger.i { "Starting kotlin scripting guest" }
    val nativeEngine = NativeEngine()
    nativeEngine.init(worldHandle)
}

@CName("dropbear_load")
fun loadScriptByTag(tag: String?) {

}

@CName("dropbear_update")
fun updateScriptByTag(tag: String?, deltaTime: Double) {

}
