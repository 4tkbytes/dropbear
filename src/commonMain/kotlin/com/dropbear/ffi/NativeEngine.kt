package com.dropbear.ffi

import com.dropbear.EntityRef

expect class NativeEngine {
    fun getEntity(label: String): ULong?
}