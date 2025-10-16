package com.dropbear.math

import kotlin.jvm.JvmField

/**
 * A class for holding a vector of `3` values of the same type.
 */
class Vector3<T: Number>(
    @JvmField var x: T,
    @JvmField var y: T,
    @JvmField var z: T,
) {
    companion object {
        /**
         * Creates a new [com.dropbear.math.Vector3] of type `T` with one value.
         */
        fun <T: Number> uniform(value: T): Vector3<T> {
            return Vector3(value, value, value)
        }

        /**
         * Creates a [com.dropbear.math.Vector3] of type [Double] filled with only zeroes
         */
        fun zero(): Vector3<Double> {
            return Vector3(0.0, 0.0, 0.0)
        }
    }

    fun asDoubleVector(): Vector3<Double> {
        return Vector3(this.x.toDouble(), this.y.toDouble(), this.z.toDouble())
    }

}

typealias Vector3D = Vector3<Double>