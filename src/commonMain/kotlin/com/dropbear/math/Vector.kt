package com.dropbear.math

/**
 * Abstract class all Vectors inherit.
 */
abstract class Vector<T: Number, SELF: Vector<T, SELF>> {
    abstract fun normalize(): SELF

    abstract operator fun plus(other: SELF): SELF
    abstract operator fun plus(scalar: T): SELF

    abstract operator fun minus(other: SELF): SELF
    abstract operator fun minus(scalar: T): SELF

    abstract operator fun times(other: SELF): SELF
    abstract operator fun times(scalar: T): SELF

    abstract operator fun div(other: SELF): SELF
    abstract operator fun div(scalar: T): SELF

    abstract fun length(): Double
}