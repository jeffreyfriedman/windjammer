package dev.windjammer.sdk;

/**
 * 3D vector.
 */
public record Vec3(float x, float y, float z) {
    /**
     * Adds two vectors.
     */
    public Vec3 add(Vec3 other) {
        return new Vec3(x + other.x, y + other.y, z + other.z);
    }

    /**
     * Subtracts two vectors.
     */
    public Vec3 sub(Vec3 other) {
        return new Vec3(x - other.x, y - other.y, z - other.z);
    }

    /**
     * Multiplies by a scalar.
     */
    public Vec3 mul(float scalar) {
        return new Vec3(x * scalar, y * scalar, z * scalar);
    }

    /**
     * Calculates the length of the vector.
     */
    public float length() {
        return (float) Math.sqrt(x * x + y * y + z * z);
    }

    /**
     * Returns a normalized copy of the vector.
     */
    public Vec3 normalized() {
        float len = length();
        if (len > 0) {
            return new Vec3(x / len, y / len, z / len);
        }
        return zero();
    }

    /**
     * Calculates the dot product.
     */
    public float dot(Vec3 other) {
        return x * other.x + y * other.y + z * other.z;
    }

    /**
     * Calculates the cross product.
     */
    public Vec3 cross(Vec3 other) {
        return new Vec3(
            y * other.z - z * other.y,
            z * other.x - x * other.z,
            x * other.y - y * other.x
        );
    }

    /**
     * Zero vector (0, 0, 0).
     */
    public static Vec3 zero() {
        return new Vec3(0, 0, 0);
    }

    /**
     * One vector (1, 1, 1).
     */
    public static Vec3 one() {
        return new Vec3(1, 1, 1);
    }

    /**
     * Up vector (0, 1, 0).
     */
    public static Vec3 up() {
        return new Vec3(0, 1, 0);
    }

    /**
     * Forward vector (0, 0, -1).
     */
    public static Vec3 forward() {
        return new Vec3(0, 0, -1);
    }

    /**
     * Right vector (1, 0, 0).
     */
    public static Vec3 right() {
        return new Vec3(1, 0, 0);
    }
}

