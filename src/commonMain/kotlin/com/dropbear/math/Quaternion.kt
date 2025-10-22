@file:Suppress("UNCHECKED_CAST")

package com.dropbear.math

import kotlin.jvm.JvmField
import kotlin.math.asin
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlin.math.sin
import kotlin.math.sqrt

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

        fun fromEulerAngles(pitch: Double, yaw: Double, roll: Double): Quaternion<Double> {
            val sp = sin(pitch * 0.5)
            val cp = cos(pitch * 0.5)
            val sy = sin(yaw * 0.5)
            val cy = cos(yaw * 0.5)
            val sr = sin(roll * 0.5)
            val cr = cos(roll * 0.5)

            return Quaternion(
                x = cy * sr * cp - sy * cr * sp,
                y = sy * cr * cp + cy * sr * sp,
                z = sy * sr * cp - cy * cr * sp,
                w = cy * cr * cp + sy * sr * sp
            )
        }
    }

    fun <T : Number> rotateX(angleRadians: T): Quaternion<T> {
        val halfAngle = angleRadians.toDouble() * 0.5
        return Quaternion(sin(halfAngle), 0.0, 0.0, cos(halfAngle)) as Quaternion<T>
    }

    fun <T: Number> rotateY(angleRadians: T): Quaternion<T> {
        val halfAngle = angleRadians.toDouble() * 0.5
        return Quaternion(0.0, sin(halfAngle), 0.0, cos(halfAngle)) as Quaternion<T>
    }

    fun <T: Number> rotateZ(angleRadians: T): Quaternion<T> {
        val halfAngle = angleRadians.toDouble() * 0.5
        return Quaternion(0.0, 0.0, sin(halfAngle), cos(halfAngle)) as Quaternion<T>
    }

    fun <T: Number> conjugate(): Quaternion<T> {
        return Quaternion(-x.toDouble(), -y.toDouble(), -z.toDouble(), w.toDouble()) as Quaternion<T>
    }

    fun <T: Number> normalized(): Quaternion<T> {
        val len = sqrt(x.toDouble().pow(2) + y.toDouble().pow(2) + z.toDouble().pow(2) + w.toDouble().pow(2))
        if (len == 0.0) return identity() as Quaternion<T>
        val invLen = 1.0 / len
        return Quaternion(
            x.toDouble() * invLen,
            y.toDouble() * invLen,
            z.toDouble() * invLen,
            w.toDouble() * invLen
        ) as Quaternion<T>
    }

    fun toEulerAngles(): Vector3<Double> {
        val q: Quaternion<Double> = normalized()
        val xx = q.x * q.x
        val yy = q.y * q.y
        val zz = q.z * q.z

        val sinPitch = 2.0 * (q.w * q.x - q.y * q.z)
        val pitch = asin(max(-1.0, min(1.0, sinPitch)))

        val sinYaw = 2.0 * (q.w * q.y + q.z * q.x)
        val cosYaw = 1.0 - 2.0 * (xx + zz)
        val yaw = atan2(sinYaw, cosYaw)

        val sinRoll = 2.0 * (q.w * q.z + q.x * q.y)
        val cosRoll = 1.0 - 2.0 * (yy + xx)
        val roll = atan2(sinRoll, cosRoll)

        return Vector3D(pitch, yaw, roll)
    }
}