/**
 * @file vec4.hpp
 * @brief 4D vector class
 */

#pragma once

#include <ostream>

namespace wj {

/**
 * @brief 4D vector.
 */
struct Vec4 {
    float x = 0.0f; ///< X component
    float y = 0.0f; ///< Y component
    float z = 0.0f; ///< Z component
    float w = 0.0f; ///< W component

    /// Default constructor
    constexpr Vec4() = default;

    /// Constructor with components
    constexpr Vec4(float x, float y, float z, float w) : x(x), y(y), z(z), w(w) {}

    /// Zero vector (0, 0, 0, 0)
    [[nodiscard]] static constexpr Vec4 zero() noexcept {
        return {0.0f, 0.0f, 0.0f, 0.0f};
    }

    /// Stream output operator
    friend std::ostream& operator<<(std::ostream& os, const Vec4& v) {
        return os << "Vec4(" << v.x << ", " << v.y << ", " << v.z << ", " << v.w << ")";
    }
};

} // namespace wj

