package com.dropbear.input

import com.dropbear.ffi.NativeEngine

/**
 * High-level input management wrapper for accessing input state
 * from Kotlin scripts.
 */
object Input {
    /**
     * Check if a specific key is currently pressed
     * 
     * @param keyCode The key to check (use KeyCode constants)
     * @return true if the key is pressed, false otherwise
     * 
     * @example
     * ```kotlin
     * if (Input.isKeyPressed(KeyCode.W)) {
     *     // Move forward
     * }
     * ```
     */
    fun isKeyPressed(keyCode: Long): Boolean {
        return NativeEngine.isKeyPressed(keyCode)
    }
    
    /**
     * Get the current mouse X position
     */
    fun getMouseX(): Double {
        return NativeEngine.getMouseX()
    }
    
    /**
     * Get the current mouse Y position
     */
    fun getMouseY(): Double {
        return NativeEngine.getMouseY()
    }
    
    /**
     * Get the mouse position as a pair
     */
    fun getMousePosition(): Pair<Double, Double> {
        return Pair(getMouseX(), getMouseY())
    }
}
