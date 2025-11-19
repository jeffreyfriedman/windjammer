/**
 * @file time.hpp
 * @brief Time utilities
 */

#pragma once

#include <ostream>

namespace wj {

/**
 * @brief Time information for the current frame.
 */
class Time {
public:
    float delta_seconds = 0.016f; ///< Time since last frame in seconds (~60 FPS)
    float total_seconds = 0.0f;   ///< Total time since application start in seconds
    int frame_count = 0;          ///< Current frame number

    /// Default constructor
    constexpr Time() = default;

    /// Stream output operator
    friend std::ostream& operator<<(std::ostream& os, const Time& t) {
        return os << "Time(delta=" << t.delta_seconds 
                  << ", total=" << t.total_seconds << ")";
    }
};

} // namespace wj

