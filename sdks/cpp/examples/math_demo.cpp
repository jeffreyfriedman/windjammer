/**
 * @file math_demo.cpp
 * @brief Math Demo Example
 * 
 * Demonstrates vector math operations.
 */

#include <windjammer/windjammer.hpp>
#include <iostream>

int main() {
    std::cout << "=== Windjammer Math Demo (C++) ===\n\n";
    
    // Vec2 operations
    std::cout << "Vec2 Operations:\n";
    auto v2a = wj::Vec2{3.0f, 4.0f};
    auto v2b = wj::Vec2{1.0f, 2.0f};
    std::cout << "  v2a = " << v2a << "\n";
    std::cout << "  v2b = " << v2b << "\n";
    std::cout << "  v2a + v2b = " << (v2a + v2b) << "\n";
    std::cout << "  v2a - v2b = " << (v2a - v2b) << "\n";
    std::cout << "  v2a * 2.0 = " << (v2a * 2.0f) << "\n";
    std::cout << "  v2a.length() = " << v2a.length() << "\n";
    std::cout << "  v2a.normalized() = " << v2a.normalized() << "\n";
    std::cout << "  v2a.dot(v2b) = " << v2a.dot(v2b) << "\n\n";
    
    // Vec3 operations
    std::cout << "Vec3 Operations:\n";
    auto v3a = wj::Vec3{1.0f, 2.0f, 3.0f};
    auto v3b = wj::Vec3{4.0f, 5.0f, 6.0f};
    std::cout << "  v3a = " << v3a << "\n";
    std::cout << "  v3b = " << v3b << "\n";
    std::cout << "  v3a + v3b = " << (v3a + v3b) << "\n";
    std::cout << "  v3a - v3b = " << (v3a - v3b) << "\n";
    std::cout << "  v3a * 2.0 = " << (v3a * 2.0f) << "\n";
    std::cout << "  v3a.length() = " << v3a.length() << "\n";
    std::cout << "  v3a.normalized() = " << v3a.normalized() << "\n";
    std::cout << "  v3a.dot(v3b) = " << v3a.dot(v3b) << "\n";
    std::cout << "  v3a.cross(v3b) = " << v3a.cross(v3b) << "\n\n";
    
    // Static vectors
    std::cout << "Static Vectors:\n";
    std::cout << "  Vec3::zero() = " << wj::Vec3::zero() << "\n";
    std::cout << "  Vec3::one() = " << wj::Vec3::one() << "\n";
    std::cout << "  Vec3::up() = " << wj::Vec3::up() << "\n";
    std::cout << "  Vec3::forward() = " << wj::Vec3::forward() << "\n";
    std::cout << "  Vec3::right() = " << wj::Vec3::right() << "\n\n";
    
    // Transform
    std::cout << "Transform:\n";
    auto transform = wj::Transform{
        .position = {0.0f, 5.0f, 10.0f},
        .rotation = {0.0f, 45.0f, 0.0f},
        .scale = {1.0f, 1.0f, 1.0f}
    };
    std::cout << "  " << transform << "\n\n";
    
    return 0;
}

