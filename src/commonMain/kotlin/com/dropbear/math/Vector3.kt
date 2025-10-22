package com.dropbear.math

import kotlin.jvm.JvmField
import kotlin.math.sqrt

/**
 * A class for holding a vector of `3` values of the same type.
 */
class Vector3<T: Number>(
    @JvmField var x: T,
    @JvmField var y: T,
    @JvmField var z: T,
) : Vector<T, Vector3<T>>() {
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

        fun x(): Vector3D {
            return Vector3D(1.0, 0.0, 0.0)
        }

        fun y(): Vector3D {
            return Vector3D(0.0, 1.0, 0.0)
        }

        fun z(): Vector3D {
            return Vector3D(0.0, 0.0, 1.0)
        }
    }

    /**
     * Returns the [Vector3] to a [Vector3D] (Vector3 of type `Double`)
     */
    fun asDoubleVector(): Vector3<Double> {
        return Vector3(this.x.toDouble(), this.y.toDouble(), this.z.toDouble())
    }

    override fun normalize(): Vector3<T> {
        val length = sqrt(
            x.toDouble() * x.toDouble() +
                    y.toDouble() * y.toDouble() +
                    z.toDouble() * z.toDouble()
        )

        if (length > 0.0) {
            @Suppress("UNCHECKED_CAST")
            x = (x.toDouble() / length) as T
            @Suppress("UNCHECKED_CAST")
            y = (y.toDouble() / length) as T
            @Suppress("UNCHECKED_CAST")
            z = (z.toDouble() / length) as T
        }
        return this
    }

    override operator fun plus(other: Vector3<T>): Vector3<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector3(
            x.toDouble() + other.x.toDouble(),
            y.toDouble() + other.y.toDouble(),
            z.toDouble() + other.z.toDouble()
        ) as Vector3<T>
    }

    override operator fun plus(scalar: T): Vector3<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector3(
            x.toDouble() + scalar.toDouble(),
            y.toDouble() + scalar.toDouble(),
            z.toDouble() + scalar.toDouble()
        ) as Vector3<T>
    }

    override operator fun minus(other: Vector3<T>): Vector3<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector3(
            x.toDouble() - other.x.toDouble(),
            y.toDouble() - other.y.toDouble(),
            z.toDouble() - other.z.toDouble()
        ) as Vector3<T>
    }

    override operator fun minus(scalar: T): Vector3<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector3(
            x.toDouble() - scalar.toDouble(),
            y.toDouble() - scalar.toDouble(),
            z.toDouble() - scalar.toDouble()
        ) as Vector3<T>
    }

    override operator fun times(other: Vector3<T>): Vector3<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector3(
            x.toDouble() * other.x.toDouble(),
            y.toDouble() * other.y.toDouble(),
            z.toDouble() * other.z.toDouble()
        ) as Vector3<T>
    }

    override operator fun times(scalar: T): Vector3<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector3(
            x.toDouble() * scalar.toDouble(),
            y.toDouble() * scalar.toDouble(),
            z.toDouble() * scalar.toDouble()
        ) as Vector3<T>
    }

    override operator fun div(other: Vector3<T>): Vector3<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector3(
            x.toDouble() / other.x.toDouble(),
            y.toDouble() / other.y.toDouble(),
            z.toDouble() / other.z.toDouble()
        ) as Vector3<T>
    }

    override operator fun div(scalar: T): Vector3<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector3(
            x.toDouble() / scalar.toDouble(),
            y.toDouble() / scalar.toDouble(),
            z.toDouble() / scalar.toDouble()
        ) as Vector3<T>
    }

    /**
     * Returns the magnitude/length of the vector
     *
     * ## Math
     * `sqrt(x^2 + y^2 + z^2)`
     */
    override fun length(): Double {
        return sqrt(x.toDouble() * x.toDouble() + y.toDouble() * y.toDouble() + z.toDouble() * z.toDouble())
    }
}

typealias Vector3D = Vector3<Double>