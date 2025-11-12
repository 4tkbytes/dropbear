package com.dropbear.math

/**
 * Represents a position, rotation and scale, typically
 * attached to an entity. 
 */
class Transform(
    var position: Vector3D,
    var rotation: QuaternionD,
    var scale: Vector3D
) {
    constructor(px: Double, py: Double, pz: Double,
                rx: Double, ry: Double, rz: Double, rw: Double,
                sx: Double, sy: Double, sz: Double)
            : this(
        Vector3D(px, py, pz),
        QuaternionD(rx, ry, rz, rw),
        Vector3D(sx, sy, sz)
            )

    override fun toString(): String {
        return "Transform(position=$position, rotation=$rotation, scale=$scale)"
    }
}