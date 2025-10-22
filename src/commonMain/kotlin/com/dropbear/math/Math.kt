package com.dropbear.math

import kotlin.math.PI

fun degreesToRadians(degrees: Double): Double = degrees * PI / 180
fun radiansToDegrees(radians: Double): Double = radians * 180 / PI

fun normalizeAngle(angle: Double): Double {
    var normalized = angle % 360.0
    if (normalized > 180.0) {
        normalized -= 360.0
    } else if (normalized < -180.0) {
        normalized += 360.0
    }
    return normalized
}

fun normalizeRadians(radians: Double): Double {
    var normalized = radians % (2 * PI)
    if (normalized > PI) {
        normalized -= 2 * PI
    } else if (normalized < -PI) {
        normalized += 2 * PI
    }
    return normalized
}