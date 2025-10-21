package com.dropbear

import com.dropbear.math.Vector3D

class Camera(
    val label: String,
    val id: EntityId, // it could be attached to nothing or an AdoptedEntity
    var eye: Vector3D = Vector3D.zero(),
    var target: Vector3D = Vector3D.zero(),
    var up: Vector3D = Vector3D.zero(),
    val aspect: Double = 0.0,
    var fov_y: Double = 0.0,
    var znear: Double = 0.0,
    var zfar: Double = 0.0,
    var yaw: Double = 0.0,
    var pitch: Double = 0.0,
    var speed: Double = 0.0,
    var sensitivity: Double = 0.0
) {
    internal lateinit var engine: DropbearEngine

    /**
     * Pushes the camera values to the world to be updated.
     */
    fun setCamera() {
        engine.native.setCamera(this)
    }
}