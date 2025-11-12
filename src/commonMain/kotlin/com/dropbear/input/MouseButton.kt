package com.dropbear.input

/**
 * Buttons that can be pressed on a mouse
 */
sealed class MouseButton {
    object Left : MouseButton()
    object Right : MouseButton()
    object Middle : MouseButton()
    object Back : MouseButton()
    object Forward : MouseButton()
    data class Other(val value: Int) : MouseButton()
}