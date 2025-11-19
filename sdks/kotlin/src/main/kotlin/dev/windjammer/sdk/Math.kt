package dev.windjammer.sdk

import kotlin.math.sqrt

/**
 * 2D vector.
 */
data class Vec2(val x: Float, val y: Float) {
    operator fun plus(other: Vec2) = Vec2(x + other.x, y + other.y)
    operator fun minus(other: Vec2) = Vec2(x - other.x, y - other.y)
    operator fun times(scalar: Float) = Vec2(x * scalar, y * scalar)
    operator fun div(scalar: Float) = Vec2(x / scalar, y / scalar)
    operator fun unaryMinus() = Vec2(-x, -y)

    fun length() = sqrt(x * x + y * y)
    fun normalized() = if (length() > 0) this / length() else ZERO
    infix fun dot(other: Vec2) = x * other.x + y * other.y

    companion object {
        val ZERO = Vec2(0f, 0f)
        val ONE = Vec2(1f, 1f)
        val UP = Vec2(0f, 1f)
        val DOWN = Vec2(0f, -1f)
        val LEFT = Vec2(-1f, 0f)
        val RIGHT = Vec2(1f, 0f)
    }
}

/**
 * 3D vector.
 */
data class Vec3(val x: Float, val y: Float, val z: Float) {
    operator fun plus(other: Vec3) = Vec3(x + other.x, y + other.y, z + other.z)
    operator fun minus(other: Vec3) = Vec3(x - other.x, y - other.y, z - other.z)
    operator fun times(scalar: Float) = Vec3(x * scalar, y * scalar, z * scalar)
    operator fun div(scalar: Float) = Vec3(x / scalar, y / scalar, z / scalar)
    operator fun unaryMinus() = Vec3(-x, -y, -z)

    fun length() = sqrt(x * x + y * y + z * z)
    fun normalized() = if (length() > 0) this / length() else ZERO
    infix fun dot(other: Vec3) = x * other.x + y * other.y + z * other.z
    infix fun cross(other: Vec3) = Vec3(
        y * other.z - z * other.y,
        z * other.x - x * other.z,
        x * other.y - y * other.x
    )

    companion object {
        val ZERO = Vec3(0f, 0f, 0f)
        val ONE = Vec3(1f, 1f, 1f)
        val UP = Vec3(0f, 1f, 0f)
        val DOWN = Vec3(0f, -1f, 0f)
        val LEFT = Vec3(-1f, 0f, 0f)
        val RIGHT = Vec3(1f, 0f, 0f)
        val FORWARD = Vec3(0f, 0f, -1f)
        val BACK = Vec3(0f, 0f, 1f)
    }
}

/**
 * 4D vector.
 */
data class Vec4(val x: Float, val y: Float, val z: Float, val w: Float) {
    operator fun plus(other: Vec4) = Vec4(x + other.x, y + other.y, z + other.z, w + other.w)
    operator fun minus(other: Vec4) = Vec4(x - other.x, y - other.y, z - other.z, w - other.w)
    operator fun times(scalar: Float) = Vec4(x * scalar, y * scalar, z * scalar, w * scalar)
    operator fun div(scalar: Float) = Vec4(x / scalar, y / scalar, z / scalar, w / scalar)
    operator fun unaryMinus() = Vec4(-x, -y, -z, -w)

    fun length() = sqrt(x * x + y * y + z * z + w * w)
    fun normalized() = if (length() > 0) this / length() else ZERO
    infix fun dot(other: Vec4) = x * other.x + y * other.y + z * other.z + w * other.w

    companion object {
        val ZERO = Vec4(0f, 0f, 0f, 0f)
        val ONE = Vec4(1f, 1f, 1f, 1f)
    }
}

/**
 * Quaternion for rotations.
 */
data class Quat(val x: Float, val y: Float, val z: Float, val w: Float) {
    operator fun times(other: Quat) = Quat(
        w * other.x + x * other.w + y * other.z - z * other.y,
        w * other.y - x * other.z + y * other.w + z * other.x,
        w * other.z + x * other.y - y * other.x + z * other.w,
        w * other.w - x * other.x - y * other.y - z * other.z
    )

    fun length() = sqrt(x * x + y * y + z * z + w * w)
    fun normalized() = if (length() > 0) {
        val len = length()
        Quat(x / len, y / len, z / len, w / len)
    } else IDENTITY

    companion object {
        val IDENTITY = Quat(0f, 0f, 0f, 1f)
    }
}

