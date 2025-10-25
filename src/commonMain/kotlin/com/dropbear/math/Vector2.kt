package com.dropbear.math

import kotlin.jvm.JvmField
import kotlin.math.sqrt

/**
 * A class for holding a vector of `2` values of the same type.
 */
class Vector2<T: Number>(
    @JvmField var x: T,
    @JvmField var y: T,
) : Vector<T, Vector2<T>>() {
    companion object {
        /**
         * Creates a new [com.dropbear.math.Vector2] of type `T` with one value.
         */
        fun <T: Number> uniform(value: T): Vector2<T> {
            return Vector2(value, value)
        }

        /**
         * Creates a [com.dropbear.math.Vector2] of type [Double] filled with only zeroes
         */
        fun zero(): Vector2<Double> {
            return Vector2(0.0, 0.0)
        }
    }

    fun asDoubleVector(): Vector2<Double> {
        return Vector2(this.x.toDouble(), this.y.toDouble())
    }

    override fun normalize(): Vector2<T> {
        val length = length()
        if (length > 0.0) {
            val invLength = 1.0 / length
            @Suppress("UNCHECKED_CAST")
            x = (x.toDouble() * invLength) as T
            @Suppress("UNCHECKED_CAST")
            y = (y.toDouble() * invLength) as T
        }
        return this
    }

    override operator fun plus(other: Vector2<T>): Vector2<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector2(
            x.toDouble() + other.x.toDouble(),
            y.toDouble() + other.y.toDouble()
        ) as Vector2<T>
    }

    override operator fun plus(scalar: T): Vector2<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector2(
            x.toDouble() + scalar.toDouble(),
            y.toDouble() + scalar.toDouble()
        ) as Vector2<T>
    }

    override operator fun minus(other: Vector2<T>): Vector2<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector2(
            x.toDouble() - other.x.toDouble(),
            y.toDouble() - other.y.toDouble()
        ) as Vector2<T>
    }

    override operator fun minus(scalar: T): Vector2<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector2(
            x.toDouble() - scalar.toDouble(),
            y.toDouble() - scalar.toDouble()
        ) as Vector2<T>
    }

    override operator fun times(other: Vector2<T>): Vector2<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector2(
            x.toDouble() * other.x.toDouble(),
            y.toDouble() * other.y.toDouble()
        ) as Vector2<T>
    }

    override operator fun times(scalar: T): Vector2<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector2(
            x.toDouble() * scalar.toDouble(),
            y.toDouble() * scalar.toDouble()
        ) as Vector2<T>
    }

    override operator fun div(other: Vector2<T>): Vector2<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector2(
            x.toDouble() / other.x.toDouble(),
            y.toDouble() / other.y.toDouble()
        ) as Vector2<T>
    }

    override operator fun div(scalar: T): Vector2<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector2(
            x.toDouble() / scalar.toDouble(),
            y.toDouble() / scalar.toDouble(),
        ) as Vector2<T>
    }

    fun lengthSquared(): Double {
        val dx = x.toDouble()
        val dy = y.toDouble()
        return dx * dx + dy * dy
    }

    fun dot(other: Vector2<T>): Double {
        return x.toDouble() * other.x.toDouble() + y.toDouble() * other.y.toDouble()
    }

    fun distanceTo(other: Vector2<T>): Double {
        val dx = x.toDouble() - other.x.toDouble()
        val dy = y.toDouble() - other.y.toDouble()
        return sqrt(dx * dx + dy * dy)
    }

    fun normalizedCopy(): Vector2<Double> {
        val length = length()
        if (length == 0.0) {
            return zero()
        }
        val invLength = 1.0 / length
        return Vector2(
            x.toDouble() * invLength,
            y.toDouble() * invLength
        )
    }

    fun lerp(target: Vector2<T>, alpha: Double): Vector2<Double> {
        val inverse = 1.0 - alpha
        return Vector2(
            x.toDouble() * inverse + target.x.toDouble() * alpha,
            y.toDouble() * inverse + target.y.toDouble() * alpha
        )
    }

    operator fun unaryMinus(): Vector2<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector2(-x.toDouble(), -y.toDouble()) as Vector2<T>
    }

    fun toVector3(z: T): Vector3<T> {
        return Vector3(x, y, z)
    }

    fun toVector4(z: T, w: T): Vector4<T> {
        return Vector4(x, y, z, w)
    }

    operator fun component1(): T = x
    operator fun component2(): T = y

    /**
     * Returns the magnitude/length of the vector
     *
     * ## Math
     * `sqrt(x^2 + y^2)`
     */
    override fun length(): Double {
        return sqrt(x.toDouble() * x.toDouble() + y.toDouble() * y.toDouble())
    }

    override fun copy(): Vector2<T> {
        return Vector2(x, y)
    }

    override fun toString(): String {
        return "Vector2(x=$x, y=$y)"
    }
}

typealias Vector2D = Vector2<Double>