package com.dropbear.math

import kotlin.jvm.JvmField
import kotlin.math.sqrt

/**
 * A class for holding a vector of `4` values of the same type.
 */
class Vector4<T: Number>(
    @JvmField var x: T,
    @JvmField var y: T,
    @JvmField var z: T,
    @JvmField var w: T,
) : Vector<T, Vector4<T>>() {
    companion object {
        /**
         * Creates a new [com.dropbear.math.Vector4] of type `T` with one value.
         */
        fun <T: Number> uniform(value: T): Vector4<T> {
            return Vector4(value, value, value, value)
        }

        /**
         * Creates a [com.dropbear.math.Vector4] of type [Double] filled with only zeroes
         */
        fun zero(): Vector4<Double> {
            return Vector4(0.0, 0.0, 0.0, 0.0)
        }

        fun x(): Vector4D {
            return Vector4D(1.0, 0.0, 0.0, 0.0)
        }

        fun y(): Vector4D {
            return Vector4D(0.0, 1.0, 0.0, 0.0)
        }

        fun z(): Vector4D {
            return Vector4D(0.0, 0.0, 1.0, 0.0)
        }

        fun w(): Vector4D {
            return Vector4D(0.0, 0.0, 0.0, 1.0)
        }
    }

    fun asDoubleVector(): Vector4<Double> {
        return Vector4(x.toDouble(), y.toDouble(), z.toDouble(), w.toDouble())
    }

    override fun normalize(): Vector4<T> {
        val length = length()
        if (length > 0.0) {
            val invLength = 1.0 / length
            @Suppress("UNCHECKED_CAST")
            x = (x.toDouble() * invLength) as T
            @Suppress("UNCHECKED_CAST")
            y = (y.toDouble() * invLength) as T
            @Suppress("UNCHECKED_CAST")
            z = (z.toDouble() * invLength) as T
            @Suppress("UNCHECKED_CAST")
            w = (w.toDouble() * invLength) as T
        }
        return this
    }

    override operator fun plus(other: Vector4<T>): Vector4<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector4(
            x.toDouble() + other.x.toDouble(),
            y.toDouble() + other.y.toDouble(),
            z.toDouble() + other.z.toDouble(),
            w.toDouble() + other.w.toDouble()
        ) as Vector4<T>
    }

    override operator fun plus(scalar: T): Vector4<T> {
        val value = scalar.toDouble()
        @Suppress("UNCHECKED_CAST")
        return Vector4(
            x.toDouble() + value,
            y.toDouble() + value,
            z.toDouble() + value,
            w.toDouble() + value
        ) as Vector4<T>
    }

    override operator fun minus(other: Vector4<T>): Vector4<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector4(
            x.toDouble() - other.x.toDouble(),
            y.toDouble() - other.y.toDouble(),
            z.toDouble() - other.z.toDouble(),
            w.toDouble() - other.w.toDouble()
        ) as Vector4<T>
    }

    override operator fun minus(scalar: T): Vector4<T> {
        val value = scalar.toDouble()
        @Suppress("UNCHECKED_CAST")
        return Vector4(
            x.toDouble() - value,
            y.toDouble() - value,
            z.toDouble() - value,
            w.toDouble() - value
        ) as Vector4<T>
    }

    override operator fun times(other: Vector4<T>): Vector4<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector4(
            x.toDouble() * other.x.toDouble(),
            y.toDouble() * other.y.toDouble(),
            z.toDouble() * other.z.toDouble(),
            w.toDouble() * other.w.toDouble()
        ) as Vector4<T>
    }

    override operator fun times(scalar: T): Vector4<T> {
        val value = scalar.toDouble()
        @Suppress("UNCHECKED_CAST")
        return Vector4(
            x.toDouble() * value,
            y.toDouble() * value,
            z.toDouble() * value,
            w.toDouble() * value
        ) as Vector4<T>
    }

    override operator fun div(other: Vector4<T>): Vector4<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector4(
            x.toDouble() / other.x.toDouble(),
            y.toDouble() / other.y.toDouble(),
            z.toDouble() / other.z.toDouble(),
            w.toDouble() / other.w.toDouble()
        ) as Vector4<T>
    }

    override operator fun div(scalar: T): Vector4<T> {
        val value = scalar.toDouble()
        @Suppress("UNCHECKED_CAST")
        return Vector4(
            x.toDouble() / value,
            y.toDouble() / value,
            z.toDouble() / value,
            w.toDouble() / value
        ) as Vector4<T>
    }

    fun lengthSquared(): Double {
        val dx = x.toDouble()
        val dy = y.toDouble()
        val dz = z.toDouble()
        val dw = w.toDouble()
        return dx * dx + dy * dy + dz * dz + dw * dw
    }

    fun dot(other: Vector4<T>): Double {
        return x.toDouble() * other.x.toDouble() +
                y.toDouble() * other.y.toDouble() +
                z.toDouble() * other.z.toDouble() +
                w.toDouble() * other.w.toDouble()
    }

    fun distanceTo(other: Vector4<T>): Double {
        val dx = x.toDouble() - other.x.toDouble()
        val dy = y.toDouble() - other.y.toDouble()
        val dz = z.toDouble() - other.z.toDouble()
        val dw = w.toDouble() - other.w.toDouble()
        return sqrt(dx * dx + dy * dy + dz * dz + dw * dw)
    }

    fun normalizedCopy(): Vector4<Double> {
        val length = length()
        if (length == 0.0) {
            return zero()
        }
        val invLength = 1.0 / length
        return Vector4(
            x.toDouble() * invLength,
            y.toDouble() * invLength,
            z.toDouble() * invLength,
            w.toDouble() * invLength
        )
    }

    fun lerp(target: Vector4<T>, alpha: Double): Vector4<Double> {
        val inverse = 1.0 - alpha
        return Vector4(
            x.toDouble() * inverse + target.x.toDouble() * alpha,
            y.toDouble() * inverse + target.y.toDouble() * alpha,
            z.toDouble() * inverse + target.z.toDouble() * alpha,
            w.toDouble() * inverse + target.w.toDouble() * alpha
        )
    }

    operator fun unaryMinus(): Vector4<T> {
        @Suppress("UNCHECKED_CAST")
        return Vector4(-x.toDouble(), -y.toDouble(), -z.toDouble(), -w.toDouble()) as Vector4<T>
    }

    fun toVector3(): Vector3<T> {
        return Vector3(x, y, z)
    }

    operator fun component1(): T = x
    operator fun component2(): T = y
    operator fun component3(): T = z
    operator fun component4(): T = w

    /**
     * Returns the magnitude/length of the vector
     */
    override fun length(): Double {
        return sqrt(lengthSquared())
    }

    override fun copy(): Vector4<T> {
        return Vector4(x, y, z, w)
    }

    override fun toString(): String {
        return "Vector4(x=$x, y=$y, z=$z, w=$w)"
    }
}

typealias Vector4D = Vector4<Double>
