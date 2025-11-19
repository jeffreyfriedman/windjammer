/**
 * @file quat.hpp
 * @brief Quaternion class for rotations
 */

#pragma once

#include <ostream>

namespace wj {

/**
 * @brief Quaternion for rotations.
 */
struct Quat {
    float x = 0.0f; ///< X component
    float y = 0.0f; ///< Y component
    float z = 0.0f; ///< Z component
    float w = 1.0f; ///< W component

    /// Default constructor
    constexpr Quat() = default;

    /// Constructor with components
    constexpr Quat(float x, float y, float z, float w) : x(x), y(y), z(z), w(w) {}

    /// Identity quaternion (0, 0, 0, 1)
    [[nodiscard]] static constexpr Quat identity() noexcept {
        return {0.0f, 0.0f, 0.0f, 1.0f};
    }

    /// Stream output operator
    friend std::ostream& operator<<(std::ostream& os, const Quat& q) {
        return os << "Quat(" << q.x << ", " << q.y << ", " << q.z << ", " << q.w << ")";
    }
};

} // namespace wj

