/**
 * @file windjammer.hpp
 * @brief Main header for Windjammer C++ SDK
 * 
 * Modern C++20 bindings for the Windjammer Game Engine.
 * 
 * @example
 * @code
 * #include <windjammer/windjammer.hpp>
 * 
 * int main() {
 *     wj::App app;
 *     app.run();
 *     return 0;
 * }
 * @endcode
 */

#pragma once

#include <windjammer/core/app.hpp>
#include <windjammer/math/vec2.hpp>
#include <windjammer/math/vec3.hpp>
#include <windjammer/math/vec4.hpp>
#include <windjammer/math/quat.hpp>
#include <windjammer/core/transform.hpp>
#include <windjammer/core/time.hpp>

/// Windjammer namespace
namespace wj {
    /// SDK version
    constexpr const char* VERSION = "0.1.0";
}

