package com.dropbear

import com.dropbear.math.Vector3D

/**
 * A class to hold a reference to an entity.
 *
 * The dropbear engine interface is made in Rust, which uses a
 * borrow checker paradigm, which heavily utilises immutability
 * unless explicitly provided with the `mut` keyword.
 *
 * Because of this, the primary source of the World (place to store
 * entities) is stored in Rust, and passing an EntityRef (which contains
 * an ID) follows Rust's ideologies of passing references instead of the entire entity,
 * which provides immutability.
 *
 * To edit any values part of the entity, take a look at the functions provided
 * by [EntityRef], which will require a reference to [DropbearEngine] to push the commands.
 */
class EntityRef {
    var label: String = ""

    /**
     * Sets the position of the entity by a Vector
     */
    fun setPosition(position: Vector3D, engine: DropbearEngine) {}
}