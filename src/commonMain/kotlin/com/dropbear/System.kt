package com.dropbear

/**
 * A class that contains the basic information of a system. 
 * 
 * The dropbear engine follows an ECS paradigm, with logic being
 * provided as Systems. 
 * 
 * The main functions you would want to look at is `load`, 
 * `update` and `destroy`(not impl). 
 */
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
