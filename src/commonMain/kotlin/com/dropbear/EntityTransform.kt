package com.dropbear

import com.dropbear.math.Transform

/**
 * A component that contains the local and world transforms of an entity.
 */
class EntityTransform(var local: Transform, var world: Transform) {

    /**
     * Walks up the hierarchy to find the transform of the parent, then multiply to create a propagated [Transform].
     */
    fun propagate(): Transform? {
        return null
    }

    /**
     * Sets the local and world transforms to the engine.
     */
    fun set() {

    }
}