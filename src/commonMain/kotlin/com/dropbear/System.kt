package com.dropbear

open class System {
    var currentEntity: EntityRef? = null
        private set

    private var engineRef: DropbearEngine? = null

    open fun load(engine: DropbearEngine) {}
    open fun update(engine: DropbearEngine, deltaTime: Float) {}
    open fun destroy(engine: DropbearEngine) {}

    fun attachEngine(engine: DropbearEngine) {
        engineRef = engine
        currentEntity?.engine = engine
    }

    fun setCurrentEntity(entity: Long) {
        val engine = engineRef ?: run {
            currentEntity = null
            return
        }

        val reference = EntityRef(EntityId(entity))
        reference.engine = engine
        currentEntity = reference
    }

    fun clearCurrentEntity() {
        currentEntity = null
    }
}
