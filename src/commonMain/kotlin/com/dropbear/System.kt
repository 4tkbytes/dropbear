package com.dropbear

fun interface System {
    fun update(engine: DropbearEngine, current_entity: EntityId, deltaTime: Float)
}