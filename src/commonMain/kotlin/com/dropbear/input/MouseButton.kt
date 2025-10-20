package com.dropbear.input

sealed class MouseButton {
    object Left : MouseButton()
    object Right : MouseButton()
    object Middle : MouseButton()
    object Back : MouseButton()
    object Forward : MouseButton()
    data class Other(val value: Int) : MouseButton()
}