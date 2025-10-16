package com.dropbear.math

import kotlin.jvm.JvmField

typealias QuaternionD = Quaternion<Double>

class Quaternion<T: Number>(
    @JvmField var x: T,
    @JvmField var y: T,
    @JvmField var z: T,
    @JvmField var w: T
) {
    companion object {
        fun identity(): Quaternion<Double> {
            return Quaternion(0.0, 0.0, 0.0, 1.0)
        }
    }
}