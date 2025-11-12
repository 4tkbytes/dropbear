package com.dropbear.math

import kotlin.math.PI

/**
 * Converts degrees to radians
 */
fun degreesToRadians(degrees: Double): Double = degrees * PI / 180
/**
 * Converts radians to degrees
 */
fun radiansToDegrees(radians: Double): Double = radians * 180 / PI

/**
 * Normalises an angle (in degrees) to a point between 0 and 360 degrees
 */
fun normalizeAngle(angle: Double): Double {
    var normalized = angle % 360.0
    if (normalized > 180.0) {
        normalized -= 360.0
    } else if (normalized < -180.0) {
        normalized += 360.0
    }
    return normalized
}

/**
 * Normalises an angle (in radians) to a point between 0 and 2*PI
 */
fun normalizeRadians(radians: Double): Double {
    var normalized = radians % (2 * PI)
    if (normalized > PI) {
        normalized -= 2 * PI
    } else if (normalized < -PI) {
        normalized += 2 * PI
    }
    return normalized
}