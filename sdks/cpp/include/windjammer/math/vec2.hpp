/**
 * @file vec2.hpp
 * @brief 2D vector class
 */

#pragma once

#include <cmath>
#include <ostream>

namespace wj {

/**
 * @brief 2D vector.
 */
struct Vec2 {
    float x = 0.0f; ///< X component
    float y = 0.0f; ///< Y component

    /// Default constructor
    constexpr Vec2() = default;

    /// Constructor with components
    constexpr Vec2(float x, float y) : x(x), y(y) {}

    /// Addition operator
    [[nodiscard]] constexpr Vec2 operator+(const Vec2& other) const noexcept {
        return {x + other.x, y + other.y};
    }

    /// Subtraction operator
    [[nodiscard]] constexpr Vec2 operator-(const Vec2& other) const noexcept {
        return {x - other.x, y - other.y};
    }

    /// Scalar multiplication operator
    [[nodiscard]] constexpr Vec2 operator*(float scalar) const noexcept {
        return {x * scalar, y * scalar};
    }

    /// Friend scalar multiplication operator (scalar * vec)
    [[nodiscard]] friend constexpr Vec2 operator*(float scalar, const Vec2& v) noexcept {
        return v * scalar;
    }

    /// Calculates the length of the vector
    [[nodiscard]] float length() const noexcept {
        return std::sqrt(x * x + y * y);
    }

    /// Returns a normalized copy of the vector
    [[nodiscard]] Vec2 normalized() const noexcept {
        const float len = length();
        if (len > 0.0f) {
            return {x / len, y / len};
        }
        return zero();
    }

    /// Calculates the dot product with another vector
    [[nodiscard]] constexpr float dot(const Vec2& other) const noexcept {
        return x * other.x + y * other.y;
    }

    /// Zero vector (0, 0)
    [[nodiscard]] static constexpr Vec2 zero() noexcept {
        return {0.0f, 0.0f};
    }

    /// One vector (1, 1)
    [[nodiscard]] static constexpr Vec2 one() noexcept {
        return {1.0f, 1.0f};
    }

    /// Stream output operator
    friend std::ostream& operator<<(std::ostream& os, const Vec2& v) {
        return os << "Vec2(" << v.x << ", " << v.y << ")";
    }
};

} // namespace wj

