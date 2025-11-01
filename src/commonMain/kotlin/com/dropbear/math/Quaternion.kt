package com.dropbear.math

import kotlin.jvm.JvmField
import kotlin.math.PI
import kotlin.math.acos
import kotlin.math.asin
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.min
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
        fun identity(): QuaternionD {
            return Quaternion(0.0, 0.0, 0.0, 1.0)
        }

        fun fromEulerAngles(pitch: Double, yaw: Double, roll: Double): QuaternionD {
            val halfPitch = pitch * 0.5
            val halfYaw = yaw * 0.5
            val halfRoll = roll * 0.5
            val sp = sin(halfPitch)
            val cp = cos(halfPitch)
            val sy = sin(halfYaw)
            val cy = cos(halfYaw)
            val sr = sin(halfRoll)
            val cr = cos(halfRoll)

            return Quaternion(
                x = cy * sr * cp - sy * cr * sp,
                y = sy * cr * cp + cy * sr * sp,
                z = sy * sr * cp - cy * cr * sp,
                w = cy * cr * cp + sy * sr * sp
            )
        }

        fun fromAxisAngle(axis: Vector3<Double>, angleRadians: Double): QuaternionD {
            val normalizedAxis = axis.normalizedCopy()
            val halfAngle = angleRadians * 0.5
            val sinHalf = sin(halfAngle)
            return Quaternion(
                normalizedAxis.x * sinHalf,
                normalizedAxis.y * sinHalf,
                normalizedAxis.z * sinHalf,
                cos(halfAngle)
            )
        }

        fun rotateX(angleRadians: Double): QuaternionD {
            return fromAxisAngle(Vector3D(1.0, 0.0, 0.0), angleRadians)
        }

        fun rotateY(angleRadians: Double): QuaternionD {
            return fromAxisAngle(Vector3D(0.0, 1.0, 0.0), angleRadians)
        }

        fun rotateZ(angleRadians: Double): QuaternionD {
            return fromAxisAngle(Vector3D(0.0, 0.0, 1.0), angleRadians)
        }

        fun fromToRotation(from: Vector3<Double>, to: Vector3<Double>): QuaternionD {
            val start = from.normalizedCopy()
            val end = to.normalizedCopy()
            val dot = (start.x * end.x) + (start.y * end.y) + (start.z * end.z)
            if (dot >= 1.0 - 1e-6) {
                return identity()
            }
            if (dot <= -1.0 + 1e-6) {
                val orthogonal = if (kotlin.math.abs(start.x) < 0.9) {
                    Vector3D(1.0, 0.0, 0.0)
                } else {
                    Vector3D(0.0, 1.0, 0.0)
                }
                val axis = start.cross(orthogonal).normalizedCopy()
                return fromAxisAngle(axis, PI)
            }
            val axis = start.cross(end)
            val angle = acos(dot.coerceIn(-1.0, 1.0))
            return fromAxisAngle(axis, angle)
        }
    }

    fun <T : Number> rotateX(angleRadians: T): Quaternion<T> {
        val halfAngle = angleRadians.toDouble() * 0.5
        @Suppress("UNCHECKED_CAST")
        return Quaternion(sin(halfAngle), 0.0, 0.0, cos(halfAngle)) as Quaternion<T>
    }

    fun <T: Number> rotateY(angleRadians: T): Quaternion<T> {
        val halfAngle = angleRadians.toDouble() * 0.5
        @Suppress("UNCHECKED_CAST")
        return Quaternion(0.0, sin(halfAngle), 0.0, cos(halfAngle)) as Quaternion<T>
    }

    fun <T: Number> rotateZ(angleRadians: T): Quaternion<T> {
        val halfAngle = angleRadians.toDouble() * 0.5
        @Suppress("UNCHECKED_CAST")
        return Quaternion(0.0, 0.0, sin(halfAngle), cos(halfAngle)) as Quaternion<T>
    }

    fun asDoubleQuaternion(): QuaternionD {
        return Quaternion(x.toDouble(), y.toDouble(), z.toDouble(), w.toDouble())
    }

    fun conjugate(): QuaternionD {
        return Quaternion(-x.toDouble(), -y.toDouble(), -z.toDouble(), w.toDouble())
    }

    fun dot(other: Quaternion<*>): Double {
        return x.toDouble() * other.x.toDouble() +
                y.toDouble() * other.y.toDouble() +
                z.toDouble() * other.z.toDouble() +
                w.toDouble() * other.w.toDouble()
    }

    fun lengthSquared(): Double {
        return dot(this)
    }

    fun length(): Double {
        return sqrt(lengthSquared())
    }

    fun isNormalized(epsilon: Double = 1e-6): Boolean {
        return kotlin.math.abs(length() - 1.0) <= epsilon
    }

    fun normalized(): QuaternionD {
        val len = length()
        if (len == 0.0) {
            return identity()
        }
        val invLen = 1.0 / len
        return Quaternion(
            x.toDouble() * invLen,
            y.toDouble() * invLen,
            z.toDouble() * invLen,
            w.toDouble() * invLen
        )
    }

    fun normalizeInPlace(): Quaternion<T> {
        val normalized = normalized()
        @Suppress("UNCHECKED_CAST")
        x = normalized.x as T
        @Suppress("UNCHECKED_CAST")
        y = normalized.y as T
        @Suppress("UNCHECKED_CAST")
        z = normalized.z as T
        @Suppress("UNCHECKED_CAST")
        w = normalized.w as T
        return this
    }

    operator fun <R: Number> plus(other: Quaternion<R>): QuaternionD {
        return Quaternion(
            x.toDouble() + other.x.toDouble(),
            y.toDouble() + other.y.toDouble(),
            z.toDouble() + other.z.toDouble(),
            w.toDouble() + other.w.toDouble()
        )
    }

    operator fun <R: Number> minus(other: Quaternion<R>): QuaternionD {
        return Quaternion(
            x.toDouble() - other.x.toDouble(),
            y.toDouble() - other.y.toDouble(),
            z.toDouble() - other.z.toDouble(),
            w.toDouble() - other.w.toDouble()
        )
    }

    operator fun unaryMinus(): QuaternionD {
        return Quaternion(-x.toDouble(), -y.toDouble(), -z.toDouble(), -w.toDouble())
    }

    operator fun times(scalar: Number): QuaternionD {
        val value = scalar.toDouble()
        return Quaternion(
            x.toDouble() * value,
            y.toDouble() * value,
            z.toDouble() * value,
            w.toDouble() * value
        )
    }

    operator fun div(scalar: Number): QuaternionD {
        val value = scalar.toDouble()
        require(value != 0.0) { "Cannot divide Quaternion by zero." }
        val inv = 1.0 / value
        return Quaternion(
            x.toDouble() * inv,
            y.toDouble() * inv,
            z.toDouble() * inv,
            w.toDouble() * inv
        )
    }

    operator fun <R: Number> times(other: Quaternion<R>): QuaternionD {
        val ax = x.toDouble()
        val ay = y.toDouble()
        val az = z.toDouble()
        val aw = w.toDouble()
        val bx = other.x.toDouble()
        val by = other.y.toDouble()
        val bz = other.z.toDouble()
        val bw = other.w.toDouble()
        return Quaternion(
            aw * bx + ax * bw + ay * bz - az * by,
            aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw,
            aw * bw - ax * bx - ay * by - az * bz
        )
    }

    fun inverse(): QuaternionD {
        val lenSq = lengthSquared()
        if (lenSq == 0.0) {
            return identity()
        }
        val conjugate = conjugate()
        val inv = 1.0 / lenSq
        return Quaternion(
            conjugate.x * inv,
            conjugate.y * inv,
            conjugate.z * inv,
            conjugate.w * inv
        )
    }

    fun rotate(vector: Vector3<T>): Vector3<Double> {
        val doubleVector = vector.asDoubleVector()
        val vectorQuat = Quaternion(doubleVector.x, doubleVector.y, doubleVector.z, 0.0)
        val rotated = (this * vectorQuat) * conjugate()
        return Vector3(rotated.x, rotated.y, rotated.z)
    }

    operator fun times(vector: Vector3<T>): Vector3<Double> {
        return rotate(vector)
    }

    fun nlerp(other: Quaternion<*>, t: Double): QuaternionD {
        val alpha = t.coerceIn(0.0, 1.0)
        val inverse = 1.0 - alpha
        return Quaternion(
            x.toDouble() * inverse + other.x.toDouble() * alpha,
            y.toDouble() * inverse + other.y.toDouble() * alpha,
            z.toDouble() * inverse + other.z.toDouble() * alpha,
            w.toDouble() * inverse + other.w.toDouble() * alpha
        ).normalized()
    }

    fun slerp(other: Quaternion<*>, t: Double): QuaternionD {
        val alpha = t.coerceIn(0.0, 1.0)
        var q1 = normalized()
        var q2 = Quaternion(
            other.x.toDouble(),
            other.y.toDouble(),
            other.z.toDouble(),
            other.w.toDouble()
        ).normalized()

        var dot = q1.x * q2.x + q1.y * q2.y + q1.z * q2.z + q1.w * q2.w
        if (dot < 0.0) {
            dot = -dot
            q2 = Quaternion(-q2.x, -q2.y, -q2.z, -q2.w)
        }

        dot = min(1.0, max(-1.0, dot))

        if (dot > 0.9995) {
            return Quaternion(
                q1.x + alpha * (q2.x - q1.x),
                q1.y + alpha * (q2.y - q1.y),
                q1.z + alpha * (q2.z - q1.z),
                q1.w + alpha * (q2.w - q1.w)
            ).normalized()
        }

        val theta0 = acos(dot)
        val sinTheta0 = sin(theta0)
        val theta = theta0 * alpha
        val sinTheta = sin(theta)
        val s0 = cos(theta) - dot * sinTheta / sinTheta0
        val s1 = sinTheta / sinTheta0

        return Quaternion(
            q1.x * s0 + q2.x * s1,
            q1.y * s0 + q2.y * s1,
            q1.z * s0 + q2.z * s1,
            q1.w * s0 + q2.w * s1
        ).normalized()
    }

    fun toEulerAngles(): Vector3D {
        val q = normalized()
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

    fun toAxisAngle(): Pair<Vector3D, Double> {
        val q = normalized()
        val angle = 2.0 * acos(q.w)
        val sinHalfAngle = sqrt(1.0 - q.w * q.w)
        if (sinHalfAngle < 1e-6) {
            return Vector3D(1.0, 0.0, 0.0) to angle
        }
        return Vector3D(q.x / sinHalfAngle, q.y / sinHalfAngle, q.z / sinHalfAngle) to angle
    }

    fun toVector4(): Vector4<Double> {
        return Vector4(x.toDouble(), y.toDouble(), z.toDouble(), w.toDouble())
    }

    override fun toString(): String {
        return "Quaternion(x=$x, y=$y, z=$z, w=$w)"
    }
}