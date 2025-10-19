package com.dropbear.input

import com.dropbear.DropbearEngine

class InputState(private val engine: DropbearEngine) {

    fun printInputState() {
        engine.native.printInputState()
    }

    fun isKeyPressed(key: KeyCode): Boolean {
        return engine.native.isKeyPressed(key)
    }
}