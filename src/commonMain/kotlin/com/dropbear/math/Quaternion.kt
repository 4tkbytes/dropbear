package com.dropbear.math

typealias QuaternionD = Quaternion<Double>

class Quaternion<T: Number>(var x: T, var y: T, var z: T, var w: T) {
    companion object {
        fun identity(): Quaternion<Double> {
            return Quaternion(0.0, 0.0, 0.0, 1.0)
        }
    }
}