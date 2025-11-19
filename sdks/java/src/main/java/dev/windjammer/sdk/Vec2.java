package dev.windjammer.sdk;

/**
 * 2D vector.
 */
public record Vec2(float x, float y) {
    /**
     * Adds two vectors.
     */
    public Vec2 add(Vec2 other) {
        return new Vec2(x + other.x, y + other.y);
    }

    /**
     * Subtracts two vectors.
     */
    public Vec2 sub(Vec2 other) {
        return new Vec2(x - other.x, y - other.y);
    }

    /**
     * Multiplies by a scalar.
     */
    public Vec2 mul(float scalar) {
        return new Vec2(x * scalar, y * scalar);
    }

    /**
     * Calculates the length of the vector.
     */
    public float length() {
        return (float) Math.sqrt(x * x + y * y);
    }

    /**
     * Returns a normalized copy of the vector.
     */
    public Vec2 normalized() {
        float len = length();
        if (len > 0) {
            return new Vec2(x / len, y / len);
        }
        return zero();
    }

    /**
     * Calculates the dot product.
     */
    public float dot(Vec2 other) {
        return x * other.x + y * other.y;
    }

    /**
     * Zero vector (0, 0).
     */
    public static Vec2 zero() {
        return new Vec2(0, 0);
    }

    /**
     * One vector (1, 1).
     */
    public static Vec2 one() {
        return new Vec2(1, 1);
    }
}

