package com.dropbear.ffi

expect class NativeEngine {
    fun getEntity(label: String): Long?
}