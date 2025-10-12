package com.dropbear

open class System {
    public var currentEntity: EntityRef? = null

    open fun load(engine: DropbearEngine) {}
    open fun update(engine: DropbearEngine, deltaTime: Float) {}
    open fun destroy(engine: DropbearEngine) {}
    fun setCurrentEntity(entity: Long) {
        this.currentEntity = EntityRef(EntityId(entity))
    }
}