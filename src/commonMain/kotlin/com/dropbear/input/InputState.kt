package com.dropbear.input

import com.dropbear.DropbearEngine
import com.dropbear.math.Vector2D

class InputState(private val engine: DropbearEngine) {

    fun printInputState() {
        engine.native.printInputState()
    }

    fun isKeyPressed(key: KeyCode): Boolean {
        return engine.native.isKeyPressed(key)
    }

    fun getMousePosition(): Vector2D {
        return engine.native.getMousePosition() ?: Vector2D(0.0, 0.0)
    }

    fun isMouseButtonPressed(button: MouseButton): Boolean {
        return engine.native.isMouseButtonPressed(button)
    }

    fun getMouseDelta(): Vector2D {
        return engine.native.getMouseDelta() ?: Vector2D(0.0, 0.0)
    }

    fun isCursorLocked(): Boolean {
        return engine.native.isCursorLocked()
    }

    fun setCursorLocked(locked: Boolean) {
        return engine.native.setCursorLocked(locked)
    }

    fun getLastMousePos(): Vector2D {
        return engine.native.getLastMousePos() ?: Vector2D(0.0, 0.0)
    }

    fun getConnectedGamepads(): List<Gamepad> {
        TODO("Not yet implemented")
//        return engine.native.getConnectedGamepads()
    }
}