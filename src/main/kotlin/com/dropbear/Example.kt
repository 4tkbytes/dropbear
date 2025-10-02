package com.dropbear

class Example(override var engine: DropbearEngine) : RunnableScript {
    override fun load() {
        val entity = engine.getActiveEntity()

    }

    override fun update() {
        TODO("Not yet implemented")
    }
}