package com.dropbear.ffi

import com.dropbear.EntityRef

expect class NativeEngine {
    fun init(handle: ULong)
    fun getEntity(label: String): ULong?
}