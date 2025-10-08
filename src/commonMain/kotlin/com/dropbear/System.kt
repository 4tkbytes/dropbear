package com.dropbear

interface System {
    fun load(engine: DropbearEngine)
    fun update(engine: DropbearEngine, deltaTime: Float)
    fun destroy(engine: DropbearEngine)
}