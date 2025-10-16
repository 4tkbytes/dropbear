package com.dropbear

import com.dropbear.math.Transform

class EntityRef(val id: EntityId = EntityId(0L)) {
    lateinit var engine: DropbearEngine

    override fun toString(): String {
        return "EntityRef(id=$id)"
    }

    fun getTransform(): Transform? {
        return engine.getTransform(id)
    }

    fun setTransform(transform: Transform?) {
        if (transform == null) return
        return engine.setTransform(id, transform)
    }
}