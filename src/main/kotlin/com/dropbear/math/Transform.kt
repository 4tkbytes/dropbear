package com.dropbear.math

import com.dropbear.ffi.NativeEngine

/**
 * A class that keeps all the values of a Transform, which consists of a
 * position ([Vector3D]), rotation([QuaternionD]) and a scale([Vector3D]).
 * 
 * This class provides direct access to transform data from the Rust engine.
 */
class Transform private constructor(
    private var _position: Vector3D,
    private var _rotation: QuaternionD,
    private var _scale: Vector3D
) {
    /**
     * Get the position from the native engine
     */
    var position: Vector3D
        get() {
            _position = Vector3D(
                NativeEngine.getPositionX(),
                NativeEngine.getPositionY(),
                NativeEngine.getPositionZ()
            )
            return _position
        }
        set(value) {
            _position = value
            NativeEngine.setPosition(value.x, value.y, value.z)
        }
    
    /**
     * Get the rotation from the native engine
     */
    var rotation: QuaternionD
        get() {
            _rotation = Quaternion(
                NativeEngine.getRotationX(),
                NativeEngine.getRotationY(),
                NativeEngine.getRotationZ(),
                NativeEngine.getRotationW()
            )
            return _rotation
        }
        set(value) {
            _rotation = value
            NativeEngine.setRotation(value.x, value.y, value.z, value.w)
        }
    
    /**
     * Get the scale from the native engine
     */
    var scale: Vector3D
        get() {
            _scale = Vector3D(
                NativeEngine.getScaleX(),
                NativeEngine.getScaleY(),
                NativeEngine.getScaleZ()
            )
            return _scale
        }
        set(value) {
            _scale = value
            NativeEngine.setScale(value.x, value.y, value.z)
        }
    
    companion object {
        /**
         * Creates a new Transform with everything to its default values.
         * This creates a live Transform that reads from the native engine.
         */
        fun default(): Transform {
            return Transform(
                Vector3D.zero(),
                Quaternion.identity(),
                Vector3D(1.0, 1.0, 1.0)
            )
        }
        
        /**
         * Creates a Transform from the current native engine state
         */
        fun fromNative(): Transform {
            val pos = Vector3D(
                NativeEngine.getPositionX(),
                NativeEngine.getPositionY(),
                NativeEngine.getPositionZ()
            )
            val rot = Quaternion(
                NativeEngine.getRotationX(),
                NativeEngine.getRotationY(),
                NativeEngine.getRotationZ(),
                NativeEngine.getRotationW()
            )
            val scale = Vector3D(
                NativeEngine.getScaleX(),
                NativeEngine.getScaleY(),
                NativeEngine.getScaleZ()
            )
            return Transform(pos, rot, scale)
        }
    }
    
    /**
     * Translate (move) the transform by a delta
     */
    fun translate(delta: Vector3D) {
        val current = position
        position = Vector3D(
            current.x + delta.x,
            current.y + delta.y,
            current.z + delta.z
        )
    }
}
