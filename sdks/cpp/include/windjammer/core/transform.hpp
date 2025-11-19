/**
 * @file transform.hpp
 * @brief Transform component
 */

#pragma once

#include <windjammer/math/vec3.hpp>
#include <ostream>

namespace wj {

/**
 * @brief Transform component for position, rotation, and scale.
 */
struct Transform {
    Vec3 position = Vec3::zero(); ///< Position of the transform
    Vec3 rotation = Vec3::zero(); ///< Rotation of the transform
    Vec3 scale = Vec3::one();     ///< Scale of the transform

    /// Default constructor
    constexpr Transform() = default;

    /// Stream output operator
    friend std::ostream& operator<<(std::ostream& os, const Transform& t) {
        return os << "Transform(pos=" << t.position 
                  << ", rot=" << t.rotation 
                  << ", scale=" << t.scale << ")";
    }
};

} // namespace wj

