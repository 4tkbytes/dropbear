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
        val length = sqrt(
            x.toDouble() * x.toDouble() +
                    y.toDouble() * y.toDouble()
        )
        if (length > 0.0) {
            @Suppress("UNCHECKED_CAST")
            x = (x.toDouble() / length) as T
        }

        return this
    }

    override operator fun plus(other: Vector2<T>): Vector2<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector2(
            x.toDouble() + other.x.toDouble(),
            y.toDouble() + other.x.toDouble(),
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
}

public typealias Vector2D = Vector2<Double>