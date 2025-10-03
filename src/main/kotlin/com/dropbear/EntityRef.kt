package com.dropbear

import com.dropbear.math.Transform
import com.dropbear.math.Vector3D

/**
 * A class to hold a reference to an entity.
 */
class EntityRef(val label: String) {
    fun getTransform(engine: DropbearEngine): Transform {
        return engine.getTransform()
    }
    
    fun setPosition(position: Vector3D, engine: DropbearEngine) {
        val transform = engine.getTransform()
        transform.position = position
    }
}
