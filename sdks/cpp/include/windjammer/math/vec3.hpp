/**
 * @file vec3.hpp
 * @brief 3D vector class
 */

#pragma once

#include <cmath>
#include <ostream>

namespace wj {

/**
 * @brief 3D vector.
 */
struct Vec3 {
    float x = 0.0f; ///< X component
    float y = 0.0f; ///< Y component
    float z = 0.0f; ///< Z component

    /// Default constructor
    constexpr Vec3() = default;

    /// Constructor with components
    constexpr Vec3(float x, float y, float z) : x(x), y(y), z(z) {}

    /// Addition operator
    [[nodiscard]] constexpr Vec3 operator+(const Vec3& other) const noexcept {
        return {x + other.x, y + other.y, z + other.z};
    }

    /// Subtraction operator
    [[nodiscard]] constexpr Vec3 operator-(const Vec3& other) const noexcept {
        return {x - other.x, y - other.y, z - other.z};
    }

    /// Scalar multiplication operator
    [[nodiscard]] constexpr Vec3 operator*(float scalar) const noexcept {
        return {x * scalar, y * scalar, z * scalar};
    }

    /// Friend scalar multiplication operator (scalar * vec)
    [[nodiscard]] friend constexpr Vec3 operator*(float scalar, const Vec3& v) noexcept {
        return v * scalar;
    }

    /// Calculates the length of the vector
    [[nodiscard]] float length() const noexcept {
        return std::sqrt(x * x + y * y + z * z);
    }

    /// Returns a normalized copy of the vector
    [[nodiscard]] Vec3 normalized() const noexcept {
        const float len = length();
        if (len > 0.0f) {
            return {x / len, y / len, z / len};
        }
        return zero();
    }

    /// Calculates the dot product with another vector
    [[nodiscard]] constexpr float dot(const Vec3& other) const noexcept {
        return x * other.x + y * other.y + z * other.z;
    }

    /// Calculates the cross product with another vector
    [[nodiscard]] constexpr Vec3 cross(const Vec3& other) const noexcept {
        return {
            y * other.z - z * other.y,
            z * other.x - x * other.z,
            x * other.y - y * other.x
        };
    }

    /// Zero vector (0, 0, 0)
    [[nodiscard]] static constexpr Vec3 zero() noexcept {
        return {0.0f, 0.0f, 0.0f};
    }

    /// One vector (1, 1, 1)
    [[nodiscard]] static constexpr Vec3 one() noexcept {
        return {1.0f, 1.0f, 1.0f};
    }

    /// Up vector (0, 1, 0)
    [[nodiscard]] static constexpr Vec3 up() noexcept {
        return {0.0f, 1.0f, 0.0f};
    }

    /// Forward vector (0, 0, -1)
    [[nodiscard]] static constexpr Vec3 forward() noexcept {
        return {0.0f, 0.0f, -1.0f};
    }

    /// Right vector (1, 0, 0)
    [[nodiscard]] static constexpr Vec3 right() noexcept {
        return {1.0f, 0.0f, 0.0f};
    }

    /// Stream output operator
    friend std::ostream& operator<<(std::ostream& os, const Vec3& v) {
        return os << "Vec3(" << v.x << ", " << v.y << ", " << v.z << ")";
    }
};

} // namespace wj

