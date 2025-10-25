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
        val length = length()
        if (length > 0.0) {
            val invLength = 1.0 / length
            @Suppress("UNCHECKED_CAST")
            x = (x.toDouble() * invLength) as T
            @Suppress("UNCHECKED_CAST")
            y = (y.toDouble() * invLength) as T
            @Suppress("UNCHECKED_CAST")
            z = (z.toDouble() * invLength) as T
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

    fun lengthSquared(): Double {
        val dx = x.toDouble()
        val dy = y.toDouble()
        val dz = z.toDouble()
        return dx * dx + dy * dy + dz * dz
    }

    fun dot(other: Vector3<T>): Double {
        return x.toDouble() * other.x.toDouble() +
                y.toDouble() * other.y.toDouble() +
                z.toDouble() * other.z.toDouble()
    }

    fun cross(other: Vector3<T>): Vector3<Double> {
        val ax = x.toDouble()
        val ay = y.toDouble()
        val az = z.toDouble()
        val bx = other.x.toDouble()
        val by = other.y.toDouble()
        val bz = other.z.toDouble()
        return Vector3(
            ay * bz - az * by,
            az * bx - ax * bz,
            ax * by - ay * bx
        )
    }

    fun distanceTo(other: Vector3<T>): Double {
        val dx = x.toDouble() - other.x.toDouble()
        val dy = y.toDouble() - other.y.toDouble()
        val dz = z.toDouble() - other.z.toDouble()
        return sqrt(dx * dx + dy * dy + dz * dz)
    }

    fun normalizedCopy(): Vector3<Double> {
        val length = length()
        if (length == 0.0) {
            return zero()
        }
        val invLength = 1.0 / length
        return Vector3(
            x.toDouble() * invLength,
            y.toDouble() * invLength,
            z.toDouble() * invLength
        )
    }

    fun lerp(target: Vector3<T>, alpha: Double): Vector3<Double> {
        val inverse = 1.0 - alpha
        return Vector3(
            x.toDouble() * inverse + target.x.toDouble() * alpha,
            y.toDouble() * inverse + target.y.toDouble() * alpha,
            z.toDouble() * inverse + target.z.toDouble() * alpha
        )
    }

    operator fun unaryMinus(): Vector3<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector3(-x.toDouble(), -y.toDouble(), -z.toDouble()) as Vector3<T>
    }

    fun toVector2(): Vector2<T> {
        return Vector2(x, y)
    }

    fun toVector4(w: T): Vector4<T> {
        return Vector4(x, y, z, w)
    }

    operator fun component1(): T = x
    operator fun component2(): T = y
    operator fun component3(): T = z

    /**
     * Returns the magnitude/length of the vector
     *
     * ## Math
     * `sqrt(x^2 + y^2 + z^2)`
     */
    override fun length(): Double {
        return sqrt(x.toDouble() * x.toDouble() + y.toDouble() * y.toDouble() + z.toDouble() * z.toDouble())
    }

    override fun copy(): Vector3<T> {
        return Vector3(x, y, z)
    }

    override fun toString(): String {
        return "Vector3(x=$x, y=$y, z=$z)"
    }
}

typealias Vector3D = Vector3<Double>