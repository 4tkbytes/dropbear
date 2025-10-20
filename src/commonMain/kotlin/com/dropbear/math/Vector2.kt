package com.dropbear.math

import kotlin.jvm.JvmField

/**
 * A class for holding a vector of `2` values of the same type.
 */
class Vector2<T: Number>(
    @JvmField var x: T,
    @JvmField var y: T,
) {
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
}

public typealias Vector2D = Vector2<Double>